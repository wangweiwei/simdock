use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use async_trait::async_trait;
use tokio::process::Command;

use crate::{
    model::{
        CreateProfileRequest, DeviceTemplate, DoctorCheck, DoctorReport, InstallRequest, Instance,
        Platform, Profile, Runtime,
    },
    provider::{PlatformProvider, TaskSender},
};

#[derive(Debug, Clone)]
/// Android Emulator 平台 Provider。
///
/// 当前实现优先做环境诊断，并为后续托管 SDK 下载、sdkmanager 安装、
/// AVD 创建和启动保留统一入口。
pub struct AndroidProvider {
    sdk_root: PathBuf,
}

impl AndroidProvider {
    /// 创建 Android Provider。
    ///
    /// `sdk_root` 是 Simdock 托管 SDK 的默认目录；如果用户已有
    /// `ANDROID_SDK_ROOT` 或 `ANDROID_HOME`，诊断会优先识别已有环境。
    pub fn new(sdk_root: PathBuf) -> Self {
        Self { sdk_root }
    }

    /// 返回可能的 Android SDK 根目录。
    ///
    /// 顺序体现优先级：显式环境变量、macOS 默认位置、Simdock 托管目录。
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

    /// 解析当前应该使用的 Android SDK 根目录。
    ///
    /// 返回值中的 bool 表示目录是否已经存在；不存在时安装流程可使用
    /// Simdock 托管目录进行后续初始化。
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
        let java_check = DoctorCheck {
            key: "java_runtime".to_string(),
            ready: java_probe.success,
            detail: if java_probe.success {
                format!("Java runtime available: {}", java_probe.summary())
            } else {
                format!("Java runtime unavailable: {}", java_probe.summary())
            },
        };

        let sdkmanager_check = tool_check(
            "sdkmanager",
            locate_cmdline_tool(&sdk_root, "sdkmanager")
                .or_else(|| find_executable_in_path("sdkmanager")),
            &["--version"],
        )
        .await;
        let avdmanager_check = tool_check(
            "avdmanager",
            locate_cmdline_tool(&sdk_root, "avdmanager")
                .or_else(|| find_executable_in_path("avdmanager")),
            &["list", "device", "-c"],
        )
        .await;
        let emulator_check = tool_check(
            "emulator",
            locate_sdk_tool(&sdk_root, "emulator/emulator")
                .or_else(|| find_executable_in_path("emulator")),
            &["-version"],
        )
        .await;
        let adb_check = tool_check(
            "adb",
            locate_sdk_tool(&sdk_root, "platform-tools/adb")
                .or_else(|| find_executable_in_path("adb")),
            &["version"],
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

        if let Some(sender) = &task_sender {
            let _ = sender.send(crate::model::TaskEvent::Started {
                id: task_id.clone(),
                title: format!(
                    "Prepare Android emulator dependencies (API {})",
                    request.runtime_version
                ),
            });
            let _ = sender.send(crate::model::TaskEvent::Progress {
                id: task_id.clone(),
                pct: 12.0,
                message: "Resolving Android SDK root and managed directories".to_string(),
            });
            let _ = sender.send(crate::model::TaskEvent::Progress {
                id: task_id.clone(),
                pct: 38.0,
                message: "Checking Java runtime and command-line tools".to_string(),
            });
            let _ = sender.send(crate::model::TaskEvent::Progress {
                id: task_id.clone(),
                pct: 62.0,
                message: format!(
                    "Preparing system image request for API {}",
                    request.runtime_version
                ),
            });
            let _ = sender.send(crate::model::TaskEvent::Log {
                id: task_id.clone(),
                message: request
                    .device_name
                    .as_ref()
                    .map(|device| format!("Recommended virtual device: {device}"))
                    .unwrap_or_else(|| "No virtual device selected yet".to_string()),
            });
            let _ = sender.send(crate::model::TaskEvent::Progress {
                id: task_id.clone(),
                pct: 84.0,
                message: "Waiting for sdkmanager / avdmanager install wiring".to_string(),
            });
        }

        let error = "Android runtime installation is not implemented yet; next step is wiring cmdline-tools download, sdkmanager packages, and AVD creation".to_string();

        if let Some(sender) = &task_sender {
            let _ = sender.send(crate::model::TaskEvent::Failed {
                id: task_id,
                error: error.clone(),
            });
        }

        bail!(error)
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

/// 检测 Android SDK 相关工具是否可用。
async fn tool_check(name: &str, path: Option<PathBuf>, args: &[&str]) -> DoctorCheck {
    match path {
        Some(path) => {
            let probe = run_path_command(&path, args).await;
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

/// 运行 PATH 中的命令并收集输出。
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

/// 等待命令结束并转换成诊断 probe。
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

/// 在 Android SDK 的 cmdline-tools 目录中查找工具。
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

/// 在 Android SDK 根目录下按相对路径查找工具。
fn locate_sdk_tool(sdk_root: &Path, relative_path: &str) -> Option<PathBuf> {
    let path = sdk_root.join(relative_path);
    path.exists().then_some(path)
}

/// 从 PATH 中查找可执行文件。
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
