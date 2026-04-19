use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::Command,
};

use crate::{
    model::{
        CreateProfileRequest, DeviceTemplate, DoctorCheck, DoctorReport, InstallRequest, Instance,
        Platform, Profile, Runtime,
    },
    provider::{PlatformProvider, TaskSender},
};

const ANDROID_CMDLINE_TOOLS_URL: &str =
    "https://dl.google.com/android/repository/commandlinetools-mac-14742923_latest.zip";
const ANDROID_CMDLINE_TOOLS_ARCHIVE: &str = "commandlinetools-mac-14742923_latest.zip";
const MANAGED_JAVA_FEATURE_VERSION: u16 = 21;
const SDKMANAGER_LICENSE_INPUT_REPEATS: usize = 200;

#[derive(Debug, Clone)]
/// Android Emulator平台Provider。
///
/// 当前实现优先做环境诊断，并为后续托管SDK下载、sdkmanager安装、
/// AVD创建和启动保留统一入口。
pub struct AndroidProvider {
    sdk_root: PathBuf,
    avd_root: Option<PathBuf>,
}

impl AndroidProvider {
    /// 创建Android Provider。
    ///
    /// `sdk_root`是Simdock托管SDK的默认目录；如果用户已有
    /// `ANDROID_SDK_ROOT`或`ANDROID_HOME`，诊断会优先识别已有环境。
    pub fn new(sdk_root: PathBuf) -> Self {
        Self {
            sdk_root,
            avd_root: None,
        }
    }

    /// 创建带托管AVD目录的Android Provider。
    ///
    /// 桌面端和CLI都应该使用这个构造器，让Simdock创建的虚拟设备
    /// 保存在应用自己的数据目录下，而不是散落到用户默认Android目录。
    pub fn with_avd_root(sdk_root: PathBuf, avd_root: PathBuf) -> Self {
        Self {
            sdk_root,
            avd_root: Some(avd_root),
        }
    }

    /// 返回Simdock托管的AVD目录。
    fn managed_avd_root(&self) -> PathBuf {
        self.avd_root.clone().unwrap_or_else(|| {
            self.sdk_root
                .parent()
                .map(|parent| parent.join("avd"))
                .unwrap_or_else(|| self.sdk_root.join("avd"))
        })
    }

    /// 返回Simdock托管Java运行时目录。
    ///
    /// Android的sdkmanager/avdmanager需要Java，但我们不要求用户安装
    /// 系统JDK；缺失时会把JRE放到这个目录并仅对子进程生效。
    fn managed_java_root(&self) -> PathBuf {
        self.sdk_root
            .parent()
            .map(|parent| parent.join("java-runtime"))
            .unwrap_or_else(|| self.sdk_root.join("java-runtime"))
    }

    /// 返回可能的Android SDK根目录。
    ///
    /// 顺序体现优先级：显式环境变量、macOS默认位置、Simdock托管目录。
    fn sdk_root_candidates(&self) -> Vec<(PathBuf, String)> {
        let mut candidates = Vec::new();

        if let Some(root) = env::var_os("ANDROID_SDK_ROOT").filter(|value| !value.is_empty()) {
            candidates.push((
                PathBuf::from(root),
                "ANDROID_SDK_ROOT environment variable".to_string(),
            ));
        }

        if let Some(root) = env::var_os("ANDROID_HOME").filter(|value| !value.is_empty()) {
            candidates.push((
                PathBuf::from(root),
                "ANDROID_HOME environment variable".to_string(),
            ));
        }

        if let Some(home_dir) = env::var_os("HOME").filter(|value| !value.is_empty()) {
            candidates.push((
                PathBuf::from(home_dir).join("Library/Android/sdk"),
                "default macOS Android SDK location".to_string(),
            ));
        }

        candidates.push((
            self.sdk_root.clone(),
            "Simdock managed SDK path".to_string(),
        ));
        candidates
    }

    /// 解析当前应该使用的Android SDK根目录。
    ///
    /// 返回值中的bool表示目录是否已经存在；不存在时安装流程可使用
    /// Simdock托管目录进行后续初始化。
    fn resolve_sdk_root(&self) -> (PathBuf, String, bool) {
        for (path, source) in self.sdk_root_candidates() {
            if path.exists() {
                return (path, source, true);
            }
        }

        (
            self.sdk_root.clone(),
            "Simdock managed SDK path".to_string(),
            false,
        )
    }
}

#[async_trait]
impl PlatformProvider for AndroidProvider {
    fn platform(&self) -> Platform {
        Platform::Android
    }

    async fn doctor(&self) -> Result<DoctorReport> {
        let (sdk_root, sdk_root_source, sdk_root_exists) = self.resolve_sdk_root();
        let sdk_root_check = DoctorCheck {
            key: "sdk_root".to_string(),
            ready: sdk_root_exists,
            detail: if sdk_root_exists {
                format!(
                    "Using Android SDK root at {} ({sdk_root_source})",
                    sdk_root.display()
                )
            } else {
                format!(
                    "Android SDK root not found yet; Simdock will provision {}",
                    sdk_root.display()
                )
            },
        };

        let java_probe = run_command("java", &["-version"]).await;
        let system_java_supported = java_probe_success_supported(&java_probe);
        let managed_java_root = self.managed_java_root();
        let managed_java_probe = probe_managed_java(&managed_java_root).await;
        let java_runtime_for_tools = if system_java_supported {
            JavaRuntime::System
        } else {
            managed_java_probe
                .as_ref()
                .map(|(runtime, _summary)| runtime.clone())
                .unwrap_or(JavaRuntime::System)
        };
        let doctor_avd_root = self.managed_avd_root();
        let tool_envs = android_tool_envs(&sdk_root, &doctor_avd_root, &java_runtime_for_tools);
        let java_check = if system_java_supported {
            DoctorCheck {
                key: "java_runtime".to_string(),
                ready: true,
                detail: format!("Java runtime available: {}", java_probe.summary()),
            }
        } else if let Some((runtime, summary)) = &managed_java_probe {
            DoctorCheck {
                key: "java_runtime".to_string(),
                ready: true,
                detail: format!(
                    "Managed Java runtime available at {}: {}",
                    runtime.java_home().display(),
                    summary
                ),
            }
        } else if java_probe.success {
            DoctorCheck {
                key: "java_runtime".to_string(),
                ready: false,
                detail: format!(
                    "System Java runtime is too old; Simdock will provision {} when installing Android",
                    managed_java_root.display()
                ),
            }
        } else {
            DoctorCheck {
                key: "java_runtime".to_string(),
                ready: false,
                detail: format!(
                    "Java runtime unavailable; Simdock will provision {} when installing Android",
                    managed_java_root.display()
                ),
            }
        };

        let sdkmanager_check = tool_check(
            "sdkmanager",
            locate_cmdline_tool(&sdk_root, "sdkmanager")
                .or_else(|| find_executable_in_path("sdkmanager")),
            &["--version"],
            &tool_envs,
        )
        .await;
        let avdmanager_check = tool_check(
            "avdmanager",
            locate_cmdline_tool(&sdk_root, "avdmanager")
                .or_else(|| find_executable_in_path("avdmanager")),
            &["list", "device", "-c"],
            &tool_envs,
        )
        .await;
        let emulator_check = tool_check(
            "emulator",
            locate_sdk_tool(&sdk_root, "emulator/emulator")
                .or_else(|| find_executable_in_path("emulator")),
            &["-version"],
            &tool_envs,
        )
        .await;
        let adb_check = tool_check(
            "adb",
            locate_sdk_tool(&sdk_root, "platform-tools/adb")
                .or_else(|| find_executable_in_path("adb")),
            &["version"],
            &tool_envs,
        )
        .await;

        let system_images_ready = sdk_root.join("system-images").exists()
            && fs::read_dir(sdk_root.join("system-images"))
                .ok()
                .is_some_and(|mut entries| entries.next().is_some());
        let system_image_check = DoctorCheck {
            key: "system_images".to_string(),
            ready: system_images_ready,
            detail: if system_images_ready {
                format!(
                    "At least one Android system image is installed under {}",
                    sdk_root.join("system-images").display()
                )
            } else {
                format!(
                    "No Android system image found under {}",
                    sdk_root.join("system-images").display()
                )
            },
        };

        Ok(DoctorReport::from_checks(
            Platform::Android,
            vec![
                sdk_root_check,
                java_check,
                sdkmanager_check,
                avdmanager_check,
                emulator_check,
                adb_check,
                system_image_check,
            ],
        ))
    }

    async fn list_runtimes(&self) -> Result<Vec<Runtime>> {
        Ok(Vec::new())
    }

    async fn list_device_templates(&self) -> Result<Vec<DeviceTemplate>> {
        Ok(Vec::new())
    }

    async fn install_runtime(
        &self,
        request: InstallRequest,
        task_sender: Option<TaskSender>,
    ) -> Result<()> {
        let task_id = "android-install".to_string();

        let api_level = normalize_android_api(&request.runtime_version);
        let abi = android_system_image_abi();
        let avd_name = android_avd_name(&api_level, abi);
        let avd_root = self.managed_avd_root();
        let java_root = self.managed_java_root();
        let cache_dir = self.sdk_root.join(".simdock-cache");

        emit_started(
            task_sender.as_ref(),
            &task_id,
            format!("Prepare Android emulator dependencies (API {api_level})"),
        );

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            8.0,
            "Resolving Android SDK root and managed directories",
        );
        let (sdk_root, sdk_source, _sdk_root_exists) = self.resolve_sdk_root();
        if let Err(error) = fs::create_dir_all(&sdk_root) {
            return fail_install(
                task_sender.as_ref(),
                &task_id,
                format!(
                    "Failed to create Android SDK root {}: {error}",
                    sdk_root.display()
                ),
            );
        }
        if let Err(error) = fs::create_dir_all(&avd_root) {
            return fail_install(
                task_sender.as_ref(),
                &task_id,
                format!(
                    "Failed to create Android AVD root {}: {error}",
                    avd_root.display()
                ),
            );
        }
        emit_log(
            task_sender.as_ref(),
            &task_id,
            format!(
                "Using Android SDK root: {} ({sdk_source})",
                sdk_root.display()
            ),
        );
        emit_log(
            task_sender.as_ref(),
            &task_id,
            format!("Using Android AVD root: {}", avd_root.display()),
        );

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            18.0,
            "Preparing managed Java runtime",
        );
        let java_runtime = match ensure_android_java_runtime(
            &java_root,
            &cache_dir,
            task_sender.as_ref(),
            &task_id,
        )
        .await
        {
            Ok(java_runtime) => java_runtime,
            Err(error) => return fail_install(task_sender.as_ref(), &task_id, error.to_string()),
        };

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            28.0,
            "Installing Android command-line tools",
        );
        let sdkmanager =
            match ensure_android_cmdline_tools(&sdk_root, task_sender.as_ref(), &task_id).await {
                Ok(sdkmanager) => sdkmanager,
                Err(error) => {
                    return fail_install(task_sender.as_ref(), &task_id, error.to_string());
                }
            };
        let avdmanager = match locate_cmdline_tool(&sdk_root, "avdmanager")
            .or_else(|| find_executable_in_path("avdmanager"))
        {
            Some(avdmanager) => avdmanager,
            None => {
                return fail_install(
                    task_sender.as_ref(),
                    &task_id,
                    "avdmanager was not found after installing Android command-line tools",
                );
            }
        };

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            42.0,
            "Accepting Android SDK licenses",
        );
        if let Err(error) = accept_android_licenses(
            &sdkmanager,
            &sdk_root,
            &avd_root,
            &java_runtime,
            task_sender.as_ref(),
            &task_id,
        )
        .await
        {
            return fail_install(task_sender.as_ref(), &task_id, error.to_string());
        }

        let system_image = format!("system-images;android-{api_level};google_apis;{abi}");
        let packages = vec![
            "platform-tools".to_string(),
            "emulator".to_string(),
            format!("platforms;android-{api_level}"),
            system_image.clone(),
        ];
        emit_progress(
            task_sender.as_ref(),
            &task_id,
            55.0,
            "Installing Android SDK packages with sdkmanager",
        );
        emit_log(
            task_sender.as_ref(),
            &task_id,
            format!("Using Android system image: {system_image}"),
        );
        emit_log(
            task_sender.as_ref(),
            &task_id,
            format!("Installing Android SDK packages: {}", packages.join(", ")),
        );
        if let Err(error) = install_android_packages(
            &sdkmanager,
            &sdk_root,
            &avd_root,
            &java_runtime,
            &packages,
            task_sender.as_ref(),
            &task_id,
        )
        .await
        {
            return fail_install(task_sender.as_ref(), &task_id, error.to_string());
        }

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            78.0,
            "Accepting Android SDK licenses",
        );
        if let Err(error) = accept_android_licenses(
            &sdkmanager,
            &sdk_root,
            &avd_root,
            &java_runtime,
            task_sender.as_ref(),
            &task_id,
        )
        .await
        {
            return fail_install(task_sender.as_ref(), &task_id, error.to_string());
        }

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            88.0,
            "Creating Android virtual device",
        );
        emit_log(
            task_sender.as_ref(),
            &task_id,
            request
                .device_name
                .as_ref()
                .map(|device| format!("Recommended virtual device: {device}"))
                .unwrap_or_else(|| "No virtual device selected yet".to_string()),
        );
        emit_log(
            task_sender.as_ref(),
            &task_id,
            format!("Using Android virtual device: {avd_name}"),
        );
        if let Err(error) = ensure_android_avd(
            &avdmanager,
            &sdk_root,
            &avd_root,
            &java_runtime,
            &avd_name,
            &system_image,
            request.device_name.as_deref(),
            task_sender.as_ref(),
            &task_id,
        )
        .await
        {
            return fail_install(task_sender.as_ref(), &task_id, error.to_string());
        }

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            100.0,
            "Android emulator is installed and ready",
        );
        emit_finished(task_sender.as_ref(), &task_id);

        Ok(())
    }

    async fn create_profile(&self, request: CreateProfileRequest) -> Result<Profile> {
        Ok(Profile {
            id: format!("android-{}", request.name),
            name: request.name,
            platform: request.platform,
            runtime_id: request.runtime_id,
            device_template_id: request.device_template_id,
            extra: serde_json::json!({}),
        })
    }

    async fn start(
        &self,
        _profile: &Profile,
        _task_sender: Option<TaskSender>,
    ) -> Result<Instance> {
        bail!("android simulator start is not implemented yet")
    }

    async fn stop(&self, _instance: &Instance) -> Result<()> {
        bail!("android simulator stop is not implemented yet")
    }
}

#[derive(Debug)]
struct CommandProbe {
    success: bool,
    stdout: String,
    stderr: String,
}

impl CommandProbe {
    /// 提取命令输出中最适合展示给用户的一行。
    fn summary(&self) -> String {
        first_meaningful_line(&self.stderr)
            .or_else(|| first_meaningful_line(&self.stdout))
            .unwrap_or("command returned no output")
            .to_string()
    }
}

#[derive(Debug, Clone)]
enum JavaRuntime {
    System,
    Managed { java_home: PathBuf },
}

impl JavaRuntime {
    /// 返回Java子进程需要的JAVA_HOME。
    fn java_home(&self) -> &Path {
        match self {
            Self::System => Path::new(""),
            Self::Managed { java_home } => java_home,
        }
    }

    /// 返回托管JRE的java可执行文件。
    fn managed_java_binary(java_home: &Path) -> PathBuf {
        java_home.join("bin/java")
    }

    /// 返回当前运行时是否需要显式设置JAVA_HOME。
    fn managed_home(&self) -> Option<&Path> {
        match self {
            Self::System => None,
            Self::Managed { java_home } => Some(java_home),
        }
    }
}

/// 发送任务开始事件。
fn emit_started(sender: Option<&TaskSender>, id: &str, title: impl Into<String>) {
    if let Some(sender) = sender {
        let _ = sender.send(crate::model::TaskEvent::Started {
            id: id.to_string(),
            title: title.into(),
        });
    }
}

/// 发送任务进度事件。
fn emit_progress(sender: Option<&TaskSender>, id: &str, pct: f32, message: impl Into<String>) {
    if let Some(sender) = sender {
        let _ = sender.send(crate::model::TaskEvent::Progress {
            id: id.to_string(),
            pct,
            message: message.into(),
        });
    }
}

/// 发送任务日志事件。
fn emit_log(sender: Option<&TaskSender>, id: &str, message: impl Into<String>) {
    if let Some(sender) = sender {
        let _ = sender.send(crate::model::TaskEvent::Log {
            id: id.to_string(),
            message: message.into(),
        });
    }
}

/// 发送任务完成事件。
fn emit_finished(sender: Option<&TaskSender>, id: &str) {
    if let Some(sender) = sender {
        let _ = sender.send(crate::model::TaskEvent::Finished { id: id.to_string() });
    }
}

/// 统一处理安装失败。
///
/// 安装流程一旦失败，既返回错误给CLI，也给GUI推送失败事件，避免界面
/// 长时间停留在“安装中”状态。
fn fail_install<T>(sender: Option<&TaskSender>, id: &str, error: impl Into<String>) -> Result<T> {
    let error = error.into();
    if let Some(sender) = sender {
        let _ = sender.send(crate::model::TaskEvent::Failed {
            id: id.to_string(),
            error: error.clone(),
        });
    }

    bail!(error)
}

/// 把用户输入的Android版本规格规整成API level。
///
/// GUI当前传入的是`"35"`；这里兼容`"android-35"`、`"API 35"`等
/// 常见写法，后续版本切换UI可以复用同一个入口。
fn normalize_android_api(requested_version: &str) -> String {
    let requested = requested_version.trim();
    let normalized = requested
        .strip_prefix("android-")
        .or_else(|| requested.strip_prefix("Android "))
        .or_else(|| requested.strip_prefix("API "))
        .unwrap_or(requested);
    let digits = normalized
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();

    if digits.is_empty() {
        "35".to_string()
    } else {
        digits
    }
}

/// 根据当前Mac CPU架构选择Android system image ABI。
///
/// Apple Silicon优先使用`arm64-v8a`，Intel Mac使用`x86_64`。
fn android_system_image_abi() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64-v8a",
        _ => "x86_64",
    }
}

/// 生成Simdock托管AVD名称。
fn android_avd_name(api_level: &str, abi: &str) -> String {
    format!(
        "simdock_api_{}_{}",
        api_level,
        abi.replace('-', "_").replace(';', "_")
    )
}

/// 根据当前Mac架构生成Adoptium API所需的arch名称。
fn adoptium_arch() -> Result<&'static str> {
    match std::env::consts::ARCH {
        "aarch64" => Ok("aarch64"),
        "x86_64" => Ok("x64"),
        other => bail!("No supported managed Java runtime is available for {other}"),
    }
}

/// 返回轻量JRE下载地址。
///
/// 这里使用Adoptium的稳定API，并请求`jre`而不是`jdk`，避免下载完整
/// 开发工具链，体积比完整JDK更小。
fn managed_java_download_url() -> Result<String> {
    let arch = adoptium_arch()?;
    Ok(format!(
        "https://api.adoptium.net/v3/binary/latest/{MANAGED_JAVA_FEATURE_VERSION}/ga/mac/{arch}/jre/hotspot/normal/eclipse"
    ))
}

/// 返回托管JRE缓存文件名。
fn managed_java_archive_name() -> Result<String> {
    Ok(format!(
        "temurin-jre-{MANAGED_JAVA_FEATURE_VERSION}-mac-{}.tar.gz",
        adoptium_arch()?
    ))
}

/// 判断当前系统Java是否足够新。
///
/// Android最新command-line tools通常要求现代Java。这里保守要求Java 17+；
/// 如果版本无法解析但命令可运行，则允许继续，避免误伤非标准发行版。
fn java_probe_success_supported(probe: &CommandProbe) -> bool {
    if !probe.success {
        return false;
    }

    java_major_version(probe).map_or(true, |major| major >= 17)
}

/// 从`java -version`输出里解析Java主版本号。
fn java_major_version(probe: &CommandProbe) -> Option<u16> {
    let output = format!("{}\n{}", probe.stderr, probe.stdout);

    for line in output.lines() {
        if let Some(version) = line.split('"').nth(1).and_then(parse_java_major_version) {
            return Some(version);
        }

        for token in line.split_whitespace() {
            if let Some(version) = parse_java_major_version(token) {
                return Some(version);
            }
        }
    }

    None
}

/// 解析Java版本字符串，兼容`1.8.0_...`和`21.0.10`两种格式。
fn parse_java_major_version(version: &str) -> Option<u16> {
    let mut parts = version.split(['.', '_', '-', '+']);
    let first = parts.next()?.parse::<u16>().ok()?;
    if first == 1 {
        parts.next()?.parse::<u16>().ok()
    } else {
        Some(first)
    }
}

/// Android SDK工具运行时需要的一组环境变量。
fn android_tool_envs<'a>(
    sdk_root: &'a Path,
    avd_root: &'a Path,
    java_runtime: &'a JavaRuntime,
) -> Vec<(&'static str, &'a Path)> {
    let mut envs = vec![
        ("ANDROID_SDK_ROOT", sdk_root),
        ("ANDROID_HOME", sdk_root),
        ("ANDROID_AVD_HOME", avd_root),
    ];

    if let Some(java_home) = java_runtime.managed_home() {
        envs.push(("JAVA_HOME", java_home));
    }

    envs
}

/// 确保Android SDK工具可用的Java运行时存在。
///
/// 优先复用系统Java；如果系统没有Java，就下载轻量JRE到Simdock
/// 托管目录，并仅通过JAVA_HOME传给sdkmanager/avdmanager。
async fn ensure_android_java_runtime(
    java_root: &Path,
    cache_dir: &Path,
    sender: Option<&TaskSender>,
    task_id: &str,
) -> Result<JavaRuntime> {
    let system_probe = run_command("java", &["-version"]).await;
    if java_probe_success_supported(&system_probe) {
        emit_log(
            sender,
            task_id,
            format!("System Java runtime available: {}", system_probe.summary()),
        );
        return Ok(JavaRuntime::System);
    }

    if system_probe.success {
        emit_log(
            sender,
            task_id,
            format!(
                "System Java runtime is too old: {}; Simdock will use managed Java runtime",
                system_probe.summary()
            ),
        );
    } else {
        emit_log(
            sender,
            task_id,
            format!(
                "System Java runtime unavailable: {}",
                system_probe.summary()
            ),
        );
    }

    if let Some((runtime, summary)) = probe_managed_java(java_root).await {
        emit_log(
            sender,
            task_id,
            format!(
                "Managed Java runtime available: {} ({summary})",
                runtime.java_home().display()
            ),
        );
        return Ok(runtime);
    }

    fs::create_dir_all(cache_dir)?;
    if let Some(parent) = java_root.parent() {
        fs::create_dir_all(parent)?;
    }

    let archive_name = managed_java_archive_name()?;
    let archive_path = cache_dir.join(&archive_name);
    if archive_path.exists() {
        emit_log(
            sender,
            task_id,
            format!(
                "Using cached managed Java runtime archive: {}",
                archive_path.display()
            ),
        );
    } else {
        let download_url = managed_java_download_url()?;
        let partial_archive_path = cache_dir.join(format!("{archive_name}.part"));
        let _ = fs::remove_file(&partial_archive_path);
        emit_log(
            sender,
            task_id,
            format!("Downloading managed Java runtime from {download_url}"),
        );
        let args = vec![
            "-fL".to_string(),
            "--silent".to_string(),
            "--show-error".to_string(),
            "--retry".to_string(),
            "3".to_string(),
            "-o".to_string(),
            partial_archive_path.display().to_string(),
            download_url,
        ];
        let probe = run_command_streamed("/usr/bin/curl", &args, &[], task_id, sender).await;
        if !probe.success {
            let _ = fs::remove_file(&partial_archive_path);
            bail!("Managed Java runtime download failed: {}", probe.summary());
        }
        fs::rename(partial_archive_path, &archive_path)?;
    }

    let extract_dir = java_root
        .parent()
        .map(|parent| parent.join(".simdock-java-extract"))
        .unwrap_or_else(|| java_root.with_file_name(".simdock-java-extract"));
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir)?;
    }
    fs::create_dir_all(&extract_dir)?;

    emit_log(
        sender,
        task_id,
        format!("Extracting managed Java runtime to {}", java_root.display()),
    );
    let args = vec![
        "-xzf".to_string(),
        archive_path.display().to_string(),
        "-C".to_string(),
        extract_dir.display().to_string(),
    ];
    let probe = run_command_streamed("/usr/bin/tar", &args, &[], task_id, sender).await;
    if !probe.success {
        let _ = fs::remove_dir_all(&extract_dir);
        bail!(
            "Managed Java runtime extraction failed: {}",
            probe.summary()
        );
    }

    let extracted_java_home = match find_java_home_in_dir(&extract_dir) {
        Some(java_home) => java_home,
        None => {
            let _ = fs::remove_dir_all(&extract_dir);
            bail!("Managed Java archive did not contain a usable java binary");
        }
    };

    if java_root.exists() {
        fs::remove_dir_all(java_root)?;
    }
    fs::rename(&extracted_java_home, java_root)?;
    let _ = fs::remove_dir_all(&extract_dir);

    let Some((runtime, summary)) = probe_managed_java(java_root).await else {
        bail!("Managed Java runtime was extracted but java -version failed");
    };
    emit_log(
        sender,
        task_id,
        format!(
            "Managed Java runtime is ready: {} ({summary})",
            runtime.java_home().display()
        ),
    );

    Ok(runtime)
}

/// 探测托管JRE是否已经可用。
async fn probe_managed_java(java_root: &Path) -> Option<(JavaRuntime, String)> {
    let java_binary = JavaRuntime::managed_java_binary(java_root);
    if !java_binary.exists() {
        return None;
    }

    let probe = run_path_command(&java_binary, &["-version"]).await;
    probe.success.then(|| {
        (
            JavaRuntime::Managed {
                java_home: java_root.to_path_buf(),
            },
            probe.summary(),
        )
    })
}

/// 从解压后的macOS JRE归档中定位真正的JAVA_HOME。
fn find_java_home_in_dir(root: &Path) -> Option<PathBuf> {
    let mut candidates = vec![root.to_path_buf(), root.join("Contents/Home")];

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            candidates.push(path.clone());
            candidates.push(path.join("Contents/Home"));
        }
    }

    candidates
        .into_iter()
        .find(|candidate| JavaRuntime::managed_java_binary(candidate).exists())
}

/// 确保Android command-line tools已安装，并返回sdkmanager路径。
///
/// 如果用户已有sdkmanager，直接复用；否则下载Google官方command-line
/// tools zip，解压到`<sdk_root>/cmdline-tools/latest`。
async fn ensure_android_cmdline_tools(
    sdk_root: &Path,
    sender: Option<&TaskSender>,
    task_id: &str,
) -> Result<PathBuf> {
    if let Some(sdkmanager) = locate_cmdline_tool(sdk_root, "sdkmanager")
        .or_else(|| find_executable_in_path("sdkmanager"))
    {
        emit_log(
            sender,
            task_id,
            format!(
                "Android command-line tools already installed at {}",
                sdkmanager.display()
            ),
        );
        return Ok(sdkmanager);
    }

    let cache_dir = sdk_root.join(".simdock-cache");
    let cmdline_tools_dir = sdk_root.join("cmdline-tools");
    fs::create_dir_all(&cache_dir)?;
    fs::create_dir_all(&cmdline_tools_dir)?;

    let archive_path = cache_dir.join(ANDROID_CMDLINE_TOOLS_ARCHIVE);
    if archive_path.exists() {
        emit_log(
            sender,
            task_id,
            format!(
                "Using cached Android command-line tools archive: {}",
                archive_path.display()
            ),
        );
    } else {
        emit_log(
            sender,
            task_id,
            format!("Downloading Android command-line tools from {ANDROID_CMDLINE_TOOLS_URL}"),
        );
        let args = vec![
            "-fL".to_string(),
            "--silent".to_string(),
            "--show-error".to_string(),
            "--retry".to_string(),
            "3".to_string(),
            "-o".to_string(),
            archive_path.display().to_string(),
            ANDROID_CMDLINE_TOOLS_URL.to_string(),
        ];
        let probe = run_command_streamed("/usr/bin/curl", &args, &[], task_id, sender).await;
        if !probe.success {
            bail!(
                "Android command-line tools download failed: {}",
                probe.summary()
            );
        }
    }

    let latest_dir = cmdline_tools_dir.join("latest");
    if !latest_dir.join("bin/sdkmanager").exists() {
        let extract_dir = cmdline_tools_dir.join(".simdock-extract");
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        fs::create_dir_all(&extract_dir)?;

        emit_log(
            sender,
            task_id,
            format!(
                "Extracting Android command-line tools to {}",
                latest_dir.display()
            ),
        );
        let args = vec![
            "-q".to_string(),
            "-o".to_string(),
            archive_path.display().to_string(),
            "-d".to_string(),
            extract_dir.display().to_string(),
        ];
        let probe = run_command_streamed("/usr/bin/unzip", &args, &[], task_id, sender).await;
        if !probe.success {
            let _ = fs::remove_dir_all(&extract_dir);
            bail!(
                "Android command-line tools extraction failed: {}",
                probe.summary()
            );
        }

        if latest_dir.exists() {
            fs::remove_dir_all(&latest_dir)?;
        }

        let unpacked_dir = extract_dir.join("cmdline-tools");
        if !unpacked_dir.exists() {
            let _ = fs::remove_dir_all(&extract_dir);
            bail!("Android command-line tools archive did not contain cmdline-tools directory");
        }

        fs::rename(&unpacked_dir, &latest_dir)?;
        let _ = fs::remove_dir_all(&extract_dir);
    }

    locate_cmdline_tool(sdk_root, "sdkmanager")
        .or_else(|| find_executable_in_path("sdkmanager"))
        .ok_or_else(|| anyhow!("sdkmanager was not found after command-line tools installation"))
}

/// 调用sdkmanager --licenses，并自动向stdin输入yes。
///
/// Android SDK license是安装emulator/system image的必经步骤；这里不让
/// 用户复制命令，而是由工具统一封装。
async fn accept_android_licenses(
    sdkmanager: &Path,
    sdk_root: &Path,
    avd_root: &Path,
    java_runtime: &JavaRuntime,
    sender: Option<&TaskSender>,
    task_id: &str,
) -> Result<()> {
    let args = vec![
        format!("--sdk_root={}", sdk_root.display()),
        "--licenses".to_string(),
    ];
    let envs = android_tool_envs(sdk_root, avd_root, java_runtime);
    let probe = run_path_command_streamed_with_input(
        sdkmanager,
        &args,
        &envs,
        Some(sdkmanager_license_input()),
        task_id,
        sender,
    )
    .await;

    if probe.success {
        Ok(())
    } else {
        bail!("Android SDK license acceptance failed: {}", probe.summary())
    }
}

/// 安装Android SDK基础包、emulator和目标system image。
async fn install_android_packages(
    sdkmanager: &Path,
    sdk_root: &Path,
    avd_root: &Path,
    java_runtime: &JavaRuntime,
    packages: &[String],
    sender: Option<&TaskSender>,
    task_id: &str,
) -> Result<()> {
    let mut args = vec![format!("--sdk_root={}", sdk_root.display())];
    args.extend(packages.iter().cloned());
    let envs = android_tool_envs(sdk_root, avd_root, java_runtime);
    let probe = run_path_command_streamed_with_input(
        sdkmanager,
        &args,
        &envs,
        Some(sdkmanager_license_input()),
        task_id,
        sender,
    )
    .await;

    if probe.success {
        Ok(())
    } else {
        bail!(
            "sdkmanager package installation failed: {}",
            probe.summary()
        )
    }
}

/// 创建或复用Simdock管理的Android虚拟设备。
async fn ensure_android_avd(
    avdmanager: &Path,
    sdk_root: &Path,
    avd_root: &Path,
    java_runtime: &JavaRuntime,
    avd_name: &str,
    system_image: &str,
    requested_device: Option<&str>,
    sender: Option<&TaskSender>,
    task_id: &str,
) -> Result<()> {
    let envs = android_tool_envs(sdk_root, avd_root, java_runtime);
    let list_args = vec!["list".to_string(), "avd".to_string()];
    let list_probe =
        run_path_command_streamed_with_input(avdmanager, &list_args, &envs, None, task_id, sender)
            .await;
    if list_probe.success && avd_list_contains(&list_probe.stdout, avd_name) {
        emit_log(
            sender,
            task_id,
            format!("Android virtual device already exists: {avd_name}"),
        );
        return Ok(());
    }

    let mut create_args = vec![
        "create".to_string(),
        "avd".to_string(),
        "--force".to_string(),
        "--name".to_string(),
        avd_name.to_string(),
        "--package".to_string(),
        system_image.to_string(),
    ];
    if let Some(device) = requested_device.filter(|device| !device.trim().is_empty()) {
        create_args.push("--device".to_string());
        create_args.push(device.to_string());
    }

    let create_probe = run_path_command_streamed_with_input(
        avdmanager,
        &create_args,
        &envs,
        Some("no\n".to_string()),
        task_id,
        sender,
    )
    .await;
    if create_probe.success {
        emit_log(
            sender,
            task_id,
            format!("Created Android virtual device {avd_name}"),
        );
        return Ok(());
    }

    if requested_device.is_some() {
        emit_log(
            sender,
            task_id,
            "avdmanager rejected selected Android device; retrying without explicit device",
        );
        let fallback_args = vec![
            "create".to_string(),
            "avd".to_string(),
            "--force".to_string(),
            "--name".to_string(),
            avd_name.to_string(),
            "--package".to_string(),
            system_image.to_string(),
        ];
        let fallback_probe = run_path_command_streamed_with_input(
            avdmanager,
            &fallback_args,
            &envs,
            Some("no\n".to_string()),
            task_id,
            sender,
        )
        .await;
        if fallback_probe.success {
            emit_log(
                sender,
                task_id,
                format!("Created Android virtual device {avd_name}"),
            );
            return Ok(());
        }

        bail!(
            "avdmanager failed to create Android virtual device: {}",
            fallback_probe.summary()
        );
    }

    bail!(
        "avdmanager failed to create Android virtual device: {}",
        create_probe.summary()
    )
}

/// 判断`avdmanager list avd`输出里是否已有目标AVD。
fn avd_list_contains(output: &str, avd_name: &str) -> bool {
    let expected = format!("Name: {avd_name}");
    output.lines().map(str::trim).any(|line| {
        line == avd_name || line == expected || line.strip_prefix("Name:") == Some(avd_name)
    })
}

/// 生成足够多的`y`输入，用于sdkmanager license prompt。
fn sdkmanager_license_input() -> String {
    "y\n".repeat(SDKMANAGER_LICENSE_INPUT_REPEATS)
}

/// 检测Android SDK相关工具是否可用。
async fn tool_check(
    name: &str,
    path: Option<PathBuf>,
    args: &[&str],
    envs: &[(&str, &Path)],
) -> DoctorCheck {
    match path {
        Some(path) => {
            let probe = run_path_command_with_env(&path, args, envs).await;
            let detail = if probe.success {
                format!(
                    "{name} available at {} ({})",
                    path.display(),
                    probe.summary()
                )
            } else {
                format!(
                    "{name} probe failed at {}: {}",
                    path.display(),
                    probe.summary()
                )
            };

            DoctorCheck {
                key: name.to_string(),
                ready: probe.success,
                detail,
            }
        }
        None => DoctorCheck {
            key: name.to_string(),
            ready: false,
            detail: format!("{name} was not found in the configured SDK root or PATH"),
        },
    }
}

/// 运行PATH中的命令并收集输出。
async fn run_command(program: &str, args: &[&str]) -> CommandProbe {
    finish_probe({
        let mut command = Command::new(program);
        command.args(args);
        command
    })
    .await
}

/// 运行指定路径的命令并收集输出。
async fn run_path_command(program: &Path, args: &[&str]) -> CommandProbe {
    finish_probe({
        let mut command = Command::new(program);
        command.args(args);
        command
    })
    .await
}

/// 运行指定路径的命令，并附加环境变量。
async fn run_path_command_with_env(
    program: &Path,
    args: &[&str],
    envs: &[(&str, &Path)],
) -> CommandProbe {
    finish_probe({
        let mut command = Command::new(program);
        command.args(args);
        for (key, value) in envs {
            command.env(key, value);
        }
        command
    })
    .await
}

/// 运行系统命令，并把stdout/stderr按行转成实时任务日志。
async fn run_command_streamed(
    program: &str,
    args: &[String],
    envs: &[(&str, &Path)],
    task_id: &str,
    sender: Option<&TaskSender>,
) -> CommandProbe {
    let mut command = Command::new(program);
    command.args(args);

    for (key, value) in envs {
        command.env(key, value);
    }

    finish_probe_streamed(command, program, args, None, task_id, sender).await
}

/// 运行指定路径的命令，必要时向stdin写入输入，并实时转发输出行。
async fn run_path_command_streamed_with_input(
    program: &Path,
    args: &[String],
    envs: &[(&str, &Path)],
    input: Option<String>,
    task_id: &str,
    sender: Option<&TaskSender>,
) -> CommandProbe {
    let mut command = Command::new(program);
    command.args(args);

    for (key, value) in envs {
        command.env(key, value);
    }

    let program_label = program.display().to_string();
    finish_probe_streamed(command, &program_label, args, input, task_id, sender).await
}

/// 等待命令结束并转换成诊断probe。
async fn finish_probe(mut command: Command) -> CommandProbe {
    match command.output().await {
        Ok(output) => CommandProbe {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        },
        Err(error) => CommandProbe {
            success: false,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

/// 等待命令结束，同时实时收集输出行。
async fn finish_probe_streamed(
    mut command: Command,
    program_label: &str,
    args: &[String],
    input: Option<String>,
    task_id: &str,
    sender: Option<&TaskSender>,
) -> CommandProbe {
    emit_log(
        sender,
        task_id,
        format!("Running: {program_label} {}", args.join(" ")),
    );

    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return CommandProbe {
                success: false,
                stdout: String::new(),
                stderr: error.to_string(),
            };
        }
    };

    if let Some(input) = input {
        if let Some(mut stdin) = child.stdin.take() {
            tokio::spawn(async move {
                let _ = stdin.write_all(input.as_bytes()).await;
            });
        }
    }

    let stdout_task = child.stdout.take().map(|stdout| {
        tokio::spawn(collect_output_lines(
            stdout,
            "stdout",
            task_id.to_string(),
            sender.cloned(),
        ))
    });
    let stderr_task = child.stderr.take().map(|stderr| {
        tokio::spawn(collect_output_lines(
            stderr,
            "stderr",
            task_id.to_string(),
            sender.cloned(),
        ))
    });

    let status = match child.wait().await {
        Ok(status) => status,
        Err(error) => {
            return CommandProbe {
                success: false,
                stdout: String::new(),
                stderr: error.to_string(),
            };
        }
    };

    let stdout = match stdout_task {
        Some(task) => task.await.unwrap_or_default(),
        None => String::new(),
    };
    let stderr = match stderr_task {
        Some(task) => task.await.unwrap_or_default(),
        None => String::new(),
    };

    CommandProbe {
        success: status.success(),
        stdout,
        stderr,
    }
}

/// 持续读取子进程输出，并把每一行转成任务日志。
async fn collect_output_lines<R>(
    stream: R,
    stream_name: &'static str,
    task_id: String,
    sender: Option<TaskSender>,
) -> String
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(stream).lines();
    let mut output = Vec::new();

    loop {
        match lines.next_line().await {
            Ok(Some(raw_line)) => {
                let normalized = raw_line.replace('\r', "\n");
                for line in normalized.lines().map(str::trim) {
                    if line.is_empty() || is_terminal_progress_noise(line) {
                        continue;
                    }

                    let line = line.to_string();
                    if let Some(sender) = &sender {
                        let _ = sender.send(crate::model::TaskEvent::Log {
                            id: task_id.clone(),
                            message: format!("{stream_name}: {line}"),
                        });
                    }
                    output.push(line);
                }
            }
            Ok(None) => break,
            Err(error) => {
                let message = format!("{stream_name} read failed: {error}");
                if let Some(sender) = &sender {
                    let _ = sender.send(crate::model::TaskEvent::Log {
                        id: task_id.clone(),
                        message: message.clone(),
                    });
                }
                output.push(message);
                break;
            }
        }
    }

    output.join("\n")
}

/// 过滤终端工具的无意义进度表头。
///
/// `curl`、`tar`等命令有时会向stderr写入动态表格；这些内容在GUI
/// 里会失去终端的覆盖效果，因此只保留真正有信息量的输出。
fn is_terminal_progress_noise(line: &str) -> bool {
    line.starts_with("% Total")
        || line.starts_with("Dload Upload")
        || line == "Total    % Received % Xferd  Average Speed   Time    Time     Time  Current"
        || line == "Speed"
}

/// 在Android SDK的cmdline-tools目录中查找工具。
fn locate_cmdline_tool(sdk_root: &Path, tool_name: &str) -> Option<PathBuf> {
    let cmdline_tools_dir = sdk_root.join("cmdline-tools");
    let mut candidates = vec![
        cmdline_tools_dir.join("latest/bin").join(tool_name),
        cmdline_tools_dir.join("bin").join(tool_name),
        sdk_root.join("tools/bin").join(tool_name),
    ];

    if let Ok(entries) = fs::read_dir(&cmdline_tools_dir) {
        for entry in entries.filter_map(Result::ok) {
            candidates.push(entry.path().join("bin").join(tool_name));
        }
    }

    candidates.into_iter().find(|path| path.exists())
}

/// 在Android SDK根目录下按相对路径查找工具。
fn locate_sdk_tool(sdk_root: &Path, relative_path: &str) -> Option<PathBuf> {
    let path = sdk_root.join(relative_path);
    path.exists().then_some(path)
}

/// 从PATH中查找可执行文件。
fn find_executable_in_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.exists())
}

/// 返回第一行非空输出。
fn first_meaningful_line(output: &str) -> Option<&str> {
    output.lines().map(str::trim).find(|line| !line.is_empty())
}
