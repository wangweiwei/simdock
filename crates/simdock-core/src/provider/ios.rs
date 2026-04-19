use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
};

use crate::{
    model::{
        CreateProfileRequest, DeviceTemplate, DoctorCheck, DoctorReport, InstallRequest, Instance,
        Platform, Profile, Runtime,
    },
    provider::{PlatformProvider, TaskSender},
};

#[derive(Debug, Clone, Default)]
/// iOS Simulator 平台 Provider。
///
/// 该 Provider 只面向 macOS，通过 Apple 官方工具链（Xcode、xcodebuild、
/// xcrun、simctl）完成诊断、运行时检测、模拟器创建和启动。
pub struct IosProvider;

impl IosProvider {
    /// 创建 iOS Provider。
    pub fn new() -> Self {
        Self
    }

    /// 在 `/Applications` 下查找 Xcode.app。
    ///
    /// 优先选择标准名称 `Xcode.app`，如果用户安装了 `Xcode-beta.app`
    /// 这类变体，则退而选择第一个以 Xcode 开头的应用。
    fn discover_xcode_app(&self) -> Option<PathBuf> {
        let applications_dir = Path::new("/Applications");
        let entries = fs::read_dir(applications_dir).ok()?;
        let mut candidates = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension() == Some(OsStr::new("app"))
                    && path
                        .file_stem()
                        .and_then(OsStr::to_str)
                        .is_some_and(|name| name.starts_with("Xcode"))
            })
            .collect::<Vec<_>>();

        candidates.sort();
        candidates
            .into_iter()
            .find(|path| path.file_name() == Some(OsStr::new("Xcode.app")))
            .or_else(|| {
                fs::read_dir(applications_dir)
                    .ok()?
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .find(|path| {
                        path.extension() == Some(OsStr::new("app"))
                            && path
                                .file_stem()
                                .and_then(OsStr::to_str)
                                .is_some_and(|name| name.starts_with("Xcode"))
                    })
            })
    }

    /// 返回 Xcode 的 Developer 目录。
    ///
    /// 后续调用 `xcodebuild`、`xcrun` 时会显式设置 `DEVELOPER_DIR`，
    /// 避免受到全局 `xcode-select` 配置漂移的影响。
    fn developer_dir(&self) -> Option<PathBuf> {
        self.discover_xcode_app()
            .map(|path| path.join("Contents/Developer"))
            .filter(|path| path.exists())
    }

    /// 获取必须存在的 iOS 工具链。
    ///
    /// 安装和列举运行时属于真实操作，缺少 Xcode 时直接返回可读错误。
    fn require_toolchain(&self) -> Result<IosToolchain> {
        let developer_dir = self.developer_dir().ok_or_else(|| {
            anyhow!("No Xcode.app installation found in /Applications. Install Xcode first.")
        })?;
        let xcodebuild_path = developer_dir.join("usr/bin/xcodebuild");

        if !xcodebuild_path.exists() {
            bail!(
                "xcodebuild was not found at {}. Reinstall or repair Xcode.",
                xcodebuild_path.display()
            );
        }

        Ok(IosToolchain {
            developer_dir,
            xcodebuild_path,
        })
    }
}

#[async_trait]
impl PlatformProvider for IosProvider {
    fn platform(&self) -> Platform {
        Platform::Ios
    }

    async fn doctor(&self) -> Result<DoctorReport> {
        let active_dir_probe = run_command("xcode-select", &["-p"], &[]).await;
        let developer_dir = self.developer_dir();
        let xcode_app_check = match &developer_dir {
            Some(path) => {
                let mut detail = format!("Found Xcode developer directory at {}", path.display());
                if active_dir_probe.success {
                    let active_dir = first_meaningful_line(&active_dir_probe.stdout)
                        .unwrap_or("unknown developer directory");
                    if active_dir != path.display().to_string() {
                        detail
                            .push_str(&format!("; current xcode-select target is {}", active_dir));
                    }
                }

                DoctorCheck {
                    key: "xcode_app".to_string(),
                    ready: true,
                    detail,
                }
            }
            None => DoctorCheck {
                key: "xcode_app".to_string(),
                ready: false,
                detail: "No Xcode.app installation found in /Applications".to_string(),
            },
        };

        let xcodebuild_check = if let Some(developer_dir) = &developer_dir {
            let xcodebuild_path = developer_dir.join("usr/bin/xcodebuild");
            let probe = run_path_command(
                &xcodebuild_path,
                &["-version"],
                &[("DEVELOPER_DIR", developer_dir)],
            )
            .await;
            let detail = if probe.success {
                let version =
                    first_meaningful_line(&probe.stdout).unwrap_or("xcodebuild is available");
                format!("{version} via {}", xcodebuild_path.display())
            } else {
                format!(
                    "xcodebuild probe failed via {}: {}",
                    xcodebuild_path.display(),
                    probe.summary()
                )
            };

            DoctorCheck {
                key: "xcodebuild".to_string(),
                ready: probe.success,
                detail,
            }
        } else {
            DoctorCheck {
                key: "xcodebuild".to_string(),
                ready: false,
                detail: "xcodebuild unavailable because no Xcode developer directory was found"
                    .to_string(),
            }
        };

        let license_check = if let Some(developer_dir) = &developer_dir {
            let xcodebuild_path = developer_dir.join("usr/bin/xcodebuild");
            let probe = run_path_command(
                &xcodebuild_path,
                &["-license", "check"],
                &[("DEVELOPER_DIR", developer_dir)],
            )
            .await;
            let detail = if probe.success {
                "Xcode license has been accepted".to_string()
            } else {
                format!(
                    "Xcode license is not accepted; run sudo xcodebuild -license accept ({})",
                    probe.summary()
                )
            };

            DoctorCheck {
                key: "xcode_license".to_string(),
                ready: probe.success,
                detail,
            }
        } else {
            DoctorCheck {
                key: "xcode_license".to_string(),
                ready: false,
                detail:
                    "Xcode license check skipped because no Xcode developer directory was found"
                        .to_string(),
            }
        };

        let simctl_check = if let Some(developer_dir) = &developer_dir {
            let probe = run_command(
                "xcrun",
                &["simctl", "help"],
                &[("DEVELOPER_DIR", developer_dir)],
            )
            .await;
            let detail = if probe.success {
                format!("simctl is available through {}", developer_dir.display())
            } else {
                format!("simctl probe failed: {}", probe.summary())
            };

            DoctorCheck {
                key: "simctl".to_string(),
                ready: probe.success,
                detail,
            }
        } else {
            DoctorCheck {
                key: "simctl".to_string(),
                ready: false,
                detail: "simctl unavailable because no Xcode developer directory was found"
                    .to_string(),
            }
        };

        let runtime_check = if let Some(developer_dir) = &developer_dir {
            let probe = run_command(
                "xcrun",
                &["simctl", "list", "runtimes"],
                &[("DEVELOPER_DIR", developer_dir)],
            )
            .await;
            let has_ios_runtime = probe.success
                && (probe.stdout.contains("iOS ")
                    || probe
                        .stdout
                        .contains("com.apple.CoreSimulator.SimRuntime.iOS"));
            let detail = if probe.success && has_ios_runtime {
                "At least one iOS simulator runtime is installed".to_string()
            } else if probe.success {
                "simctl is available, but no installed iOS runtime was detected".to_string()
            } else {
                format!("runtime probe failed: {}", probe.summary())
            };

            DoctorCheck {
                key: "ios_runtime".to_string(),
                ready: has_ios_runtime,
                detail,
            }
        } else {
            DoctorCheck {
                key: "ios_runtime".to_string(),
                ready: false,
                detail: "iOS runtime probe skipped because no Xcode developer directory was found"
                    .to_string(),
            }
        };

        Ok(DoctorReport::from_checks(
            Platform::Ios,
            vec![
                xcode_app_check,
                xcodebuild_check,
                license_check,
                simctl_check,
                runtime_check,
            ],
        ))
    }

    async fn list_runtimes(&self) -> Result<Vec<Runtime>> {
        let toolchain = self.require_toolchain()?;
        let runtimes = list_ios_runtimes(&toolchain.developer_dir).await?;

        Ok(runtimes
            .into_iter()
            .map(|runtime| Runtime {
                id: runtime.identifier,
                platform: Platform::Ios,
                version: runtime.version,
                arch: std::env::consts::ARCH.to_string(),
                installed: runtime.available,
            })
            .collect())
    }

    async fn list_device_templates(&self) -> Result<Vec<DeviceTemplate>> {
        let toolchain = self.require_toolchain()?;
        let device_types = list_ios_device_types(&toolchain.developer_dir).await?;

        Ok(device_types
            .into_iter()
            .map(|device_type| DeviceTemplate {
                id: device_type.identifier,
                platform: Platform::Ios,
                name: device_type.name,
                arch: std::env::consts::ARCH.to_string(),
            })
            .collect())
    }

    async fn install_runtime(
        &self,
        request: InstallRequest,
        task_sender: Option<TaskSender>,
    ) -> Result<()> {
        let task_id = "ios-install".to_string();

        emit_started(
            task_sender.as_ref(),
            &task_id,
            format!(
                "Prepare iOS simulator dependencies ({})",
                request.runtime_version
            ),
        );

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            5.0,
            "Checking Xcode.app and developer directory",
        );
        let toolchain = match self.require_toolchain() {
            Ok(toolchain) => toolchain,
            Err(error) => return fail_install(task_sender.as_ref(), &task_id, error.to_string()),
        };
        emit_log(
            task_sender.as_ref(),
            &task_id,
            format!(
                "Using Xcode developer directory: {}",
                toolchain.developer_dir.display()
            ),
        );

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            15.0,
            "Checking Xcode license acceptance",
        );
        let license_probe = run_path_command(
            &toolchain.xcodebuild_path,
            &["-license", "check"],
            &[("DEVELOPER_DIR", &toolchain.developer_dir)],
        )
        .await;
        if !license_probe.success {
            emit_log(
                task_sender.as_ref(),
                &task_id,
                format!("Xcode license check output: {}", license_probe.summary()),
            );
            return fail_install(
                task_sender.as_ref(),
                &task_id,
                "Xcode license has not been accepted. Open Terminal and run: sudo xcodebuild -license accept",
            );
        }
        emit_log(
            task_sender.as_ref(),
            &task_id,
            "Xcode license has been accepted",
        );

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            30.0,
            "Running xcodebuild -runFirstLaunch",
        );
        let first_launch_probe = run_path_command_streamed(
            &toolchain.xcodebuild_path,
            &["-runFirstLaunch"],
            &[("DEVELOPER_DIR", &toolchain.developer_dir)],
            &task_id,
            task_sender.as_ref(),
        )
        .await;
        if !first_launch_probe.success {
            return fail_install(
                task_sender.as_ref(),
                &task_id,
                format!(
                    "xcodebuild -runFirstLaunch failed: {}",
                    first_launch_probe.summary()
                ),
            );
        }

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            52.0,
            "Downloading iOS simulator platform with xcodebuild -downloadPlatform iOS",
        );
        let download_probe = run_path_command_streamed(
            &toolchain.xcodebuild_path,
            &["-downloadPlatform", "iOS"],
            &[("DEVELOPER_DIR", &toolchain.developer_dir)],
            &task_id,
            task_sender.as_ref(),
        )
        .await;
        if !download_probe.success {
            emit_log(
                task_sender.as_ref(),
                &task_id,
                format!(
                    "xcodebuild -downloadPlatform iOS returned an error: {}",
                    download_probe.summary()
                ),
            );
            emit_log(
                task_sender.as_ref(),
                &task_id,
                "Continuing only if the requested iOS runtime is already available",
            );
        }

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            68.0,
            "Detecting installed iOS simulator runtimes with simctl",
        );
        let runtimes = match list_ios_runtimes(&toolchain.developer_dir).await {
            Ok(runtimes) => runtimes,
            Err(error) => return fail_install(task_sender.as_ref(), &task_id, error.to_string()),
        };
        let runtime = match select_runtime(&runtimes, &request.runtime_version) {
            Some(runtime) => runtime,
            None if download_probe.success => {
                return fail_install(
                    task_sender.as_ref(),
                    &task_id,
                    format!(
                        "No available iOS simulator runtime was detected after downloading platform iOS"
                    ),
                );
            }
            None => {
                return fail_install(
                    task_sender.as_ref(),
                    &task_id,
                    format!(
                        "xcodebuild -downloadPlatform iOS failed and no iOS {} runtime is available: {}",
                        request.runtime_version,
                        download_probe.summary()
                    ),
                );
            }
        };
        if runtime.version != request.runtime_version {
            emit_log(
                task_sender.as_ref(),
                &task_id,
                format!(
                    "Requested iOS {}, using available runtime {} ({})",
                    request.runtime_version, runtime.version, runtime.identifier
                ),
            );
        } else {
            emit_log(
                task_sender.as_ref(),
                &task_id,
                format!(
                    "Found iOS runtime {} ({})",
                    runtime.version, runtime.identifier
                ),
            );
        }

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            78.0,
            "Resolving target iOS simulator device type",
        );
        let device_types = match list_ios_device_types(&toolchain.developer_dir).await {
            Ok(device_types) => device_types,
            Err(error) => return fail_install(task_sender.as_ref(), &task_id, error.to_string()),
        };
        let device_type = match select_device_type(&device_types, request.device_name.as_deref()) {
            Some(device_type) => device_type,
            None => {
                return fail_install(
                    task_sender.as_ref(),
                    &task_id,
                    "No iOS simulator device type is available from simctl",
                );
            }
        };
        emit_log(
            task_sender.as_ref(),
            &task_id,
            format!("Using simulator device type: {}", device_type.name),
        );

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            86.0,
            "Creating or reusing target iOS simulator",
        );
        let device = match find_or_create_ios_device(
            &toolchain.developer_dir,
            &runtime,
            &device_type,
            task_sender.as_ref(),
            &task_id,
        )
        .await
        {
            Ok(device) => device,
            Err(error) => return fail_install(task_sender.as_ref(), &task_id, error.to_string()),
        };

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            94.0,
            "Booting target iOS simulator",
        );
        if device.state == "Booted" {
            emit_log(
                task_sender.as_ref(),
                &task_id,
                format!("Simulator is already booted: {}", device.udid),
            );
        } else if let Err(error) = boot_ios_device(
            &toolchain.developer_dir,
            &device.udid,
            task_sender.as_ref(),
            &task_id,
        )
        .await
        {
            return fail_install(task_sender.as_ref(), &task_id, error.to_string());
        }

        open_simulator_app(
            &toolchain.developer_dir,
            &device.udid,
            task_sender.as_ref(),
            &task_id,
        )
        .await;

        emit_progress(
            task_sender.as_ref(),
            &task_id,
            100.0,
            "iOS simulator is installed and running",
        );
        emit_finished(task_sender.as_ref(), &task_id);

        Ok(())
    }

    async fn create_profile(&self, request: CreateProfileRequest) -> Result<Profile> {
        Ok(Profile {
            id: format!("ios-{}", request.name),
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
        bail!("ios simulator start is not implemented yet")
    }

    async fn stop(&self, _instance: &Instance) -> Result<()> {
        bail!("ios simulator stop is not implemented yet")
    }
}

#[derive(Debug, Clone)]
struct IosToolchain {
    developer_dir: PathBuf,
    xcodebuild_path: PathBuf,
}

#[derive(Debug, Clone)]
struct IosRuntimeInfo {
    identifier: String,
    name: String,
    version: String,
    available: bool,
}

#[derive(Debug, Clone)]
struct IosDeviceTypeInfo {
    identifier: String,
    name: String,
}

#[derive(Debug, Clone)]
struct IosDeviceInfo {
    udid: String,
    name: String,
    state: String,
    available: bool,
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
/// 这样 Provider 能保证返回错误的同时，也向 GUI / CLI 发出失败事件。
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

/// 通过 `simctl list runtimes --json` 获取 iOS 运行时。
async fn list_ios_runtimes(developer_dir: &Path) -> Result<Vec<IosRuntimeInfo>> {
    let probe = run_command(
        "xcrun",
        &["simctl", "list", "runtimes", "-j"],
        &[("DEVELOPER_DIR", developer_dir)],
    )
    .await;

    if !probe.success {
        bail!("simctl runtime probe failed: {}", probe.summary());
    }

    parse_ios_runtimes(&probe.stdout)
}

/// 通过 `simctl list devicetypes --json` 获取可创建设备类型。
async fn list_ios_device_types(developer_dir: &Path) -> Result<Vec<IosDeviceTypeInfo>> {
    let probe = run_command(
        "xcrun",
        &["simctl", "list", "devicetypes", "-j"],
        &[("DEVELOPER_DIR", developer_dir)],
    )
    .await;

    if !probe.success {
        bail!("simctl device type probe failed: {}", probe.summary());
    }

    parse_ios_device_types(&probe.stdout)
}

/// 通过 `simctl list devices --json` 获取已存在的模拟器设备。
async fn list_ios_devices(developer_dir: &Path) -> Result<Vec<(String, IosDeviceInfo)>> {
    let probe = run_command(
        "xcrun",
        &["simctl", "list", "devices", "-j"],
        &[("DEVELOPER_DIR", developer_dir)],
    )
    .await;

    if !probe.success {
        bail!("simctl device probe failed: {}", probe.summary());
    }

    parse_ios_devices(&probe.stdout)
}

/// 解析 simctl runtime JSON。
fn parse_ios_runtimes(json: &str) -> Result<Vec<IosRuntimeInfo>> {
    let value: serde_json::Value =
        serde_json::from_str(json).context("failed to parse simctl runtimes JSON")?;
    let runtimes = value
        .get("runtimes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("simctl runtimes JSON did not contain a runtimes array"))?;

    Ok(runtimes
        .iter()
        .filter_map(|runtime| {
            let identifier = runtime.get("identifier")?.as_str()?.to_string();
            let name = runtime.get("name")?.as_str()?.to_string();
            let platform = runtime
                .get("platform")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();

            let is_ios = platform == "iOS"
                || name.starts_with("iOS")
                || identifier.contains("SimRuntime.iOS");
            if !is_ios {
                return None;
            }

            let version = runtime
                .get("version")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| name.trim_start_matches("iOS").trim().to_string());
            let available = runtime
                .get("isAvailable")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or_else(|| {
                    !runtime
                        .get("availability")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .contains("unavailable")
                });

            Some(IosRuntimeInfo {
                identifier,
                name,
                version,
                available,
            })
        })
        .collect())
}

/// 解析 simctl devicetype JSON。
fn parse_ios_device_types(json: &str) -> Result<Vec<IosDeviceTypeInfo>> {
    let value: serde_json::Value =
        serde_json::from_str(json).context("failed to parse simctl device types JSON")?;
    let device_types = value
        .get("devicetypes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("simctl devicetypes JSON did not contain a devicetypes array"))?;

    Ok(device_types
        .iter()
        .filter_map(|device_type| {
            let identifier = device_type.get("identifier")?.as_str()?.to_string();
            let name = device_type.get("name")?.as_str()?.to_string();
            let product_family = device_type
                .get("productFamily")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let is_ios_device = product_family == "iPhone"
                || product_family == "iPad"
                || identifier.contains("SimDeviceType.iPhone")
                || identifier.contains("SimDeviceType.iPad");

            is_ios_device.then_some(IosDeviceTypeInfo { identifier, name })
        })
        .collect())
}

/// 解析 simctl device JSON。
fn parse_ios_devices(json: &str) -> Result<Vec<(String, IosDeviceInfo)>> {
    let value: serde_json::Value =
        serde_json::from_str(json).context("failed to parse simctl devices JSON")?;
    let devices = value
        .get("devices")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| anyhow!("simctl devices JSON did not contain a devices object"))?;

    let mut result = Vec::new();
    for (runtime_id, device_list) in devices {
        let Some(device_list) = device_list.as_array() else {
            continue;
        };

        for device in device_list {
            let Some(udid) = device.get("udid").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let Some(name) = device.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };

            let state = device
                .get("state")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Unknown")
                .to_string();
            let available = device
                .get("isAvailable")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);

            result.push((
                runtime_id.clone(),
                IosDeviceInfo {
                    udid: udid.to_string(),
                    name: name.to_string(),
                    state,
                    available,
                },
            ));
        }
    }

    Ok(result)
}

/// 选择目标 iOS 运行时。
///
/// 优先匹配用户请求的版本；如果没有精确命中，则选择已安装且版本最高的
/// 可用 iOS runtime，保证“一键安装”尽量能继续走到可启动状态。
fn select_runtime(runtimes: &[IosRuntimeInfo], requested_version: &str) -> Option<IosRuntimeInfo> {
    let requested = requested_version.trim();
    let requested_identifier_suffix = requested.replace('.', "-");

    runtimes
        .iter()
        .filter(|runtime| runtime.available)
        .find(|runtime| {
            runtime.version == requested
                || runtime.name == format!("iOS {requested}")
                || runtime
                    .identifier
                    .ends_with(&format!("iOS-{requested_identifier_suffix}"))
        })
        .cloned()
        .or_else(|| {
            runtimes
                .iter()
                .filter(|runtime| runtime.available)
                .max_by(|left, right| version_key(&left.version).cmp(&version_key(&right.version)))
                .cloned()
        })
}

/// 选择目标设备类型。
///
/// 如果用户指定设备名则优先模糊匹配；否则默认选择较新的 iPhone 类型。
fn select_device_type(
    device_types: &[IosDeviceTypeInfo],
    requested_name: Option<&str>,
) -> Option<IosDeviceTypeInfo> {
    if let Some(requested_name) = requested_name.filter(|name| !name.trim().is_empty()) {
        if let Some(device_type) = device_types
            .iter()
            .find(|device_type| device_type.name.eq_ignore_ascii_case(requested_name))
        {
            return Some(device_type.clone());
        }

        if let Some(device_type) = device_types
            .iter()
            .find(|device_type| device_type.name.contains(requested_name))
        {
            return Some(device_type.clone());
        }
    }

    ["iPhone 16", "iPhone 15", "iPhone 14"]
        .iter()
        .find_map(|preferred| {
            device_types
                .iter()
                .find(|device_type| device_type.name == *preferred)
                .cloned()
        })
        .or_else(|| {
            device_types
                .iter()
                .find(|device_type| device_type.name.starts_with("iPhone"))
                .cloned()
        })
        .or_else(|| device_types.first().cloned())
}

/// 查找或创建目标 iOS 模拟器。
///
/// 复用已有设备可以避免重复创建大量模拟器；只有找不到同名同运行时时才创建。
async fn find_or_create_ios_device(
    developer_dir: &Path,
    runtime: &IosRuntimeInfo,
    device_type: &IosDeviceTypeInfo,
    sender: Option<&TaskSender>,
    task_id: &str,
) -> Result<IosDeviceInfo> {
    let device_name = format!("Simdock {} {}", device_type.name, runtime.name);
    let devices = list_ios_devices(developer_dir).await?;

    if let Some((_, device)) = devices.into_iter().find(|(runtime_id, device)| {
        runtime_id == &runtime.identifier && device.name == device_name && device.available
    }) {
        emit_log(
            sender,
            task_id,
            format!("Reusing simulator {} ({})", device.name, device.udid),
        );
        return Ok(device);
    }

    emit_log(sender, task_id, format!("Creating simulator {device_name}"));
    let args = [
        "simctl",
        "create",
        device_name.as_str(),
        device_type.identifier.as_str(),
        runtime.identifier.as_str(),
    ];
    let probe = run_command("xcrun", &args, &[("DEVELOPER_DIR", developer_dir)]).await;
    if !probe.success {
        bail!("simctl create failed: {}", probe.summary());
    }

    let udid = first_meaningful_line(&probe.stdout)
        .ok_or_else(|| anyhow!("simctl create succeeded but did not return a device UDID"))?
        .to_string();
    emit_log(sender, task_id, format!("Created simulator {udid}"));

    Ok(IosDeviceInfo {
        udid,
        name: device_name,
        state: "Shutdown".to_string(),
        available: true,
    })
}

/// 启动指定 UDID 的 iOS 模拟器。
async fn boot_ios_device(
    developer_dir: &Path,
    udid: &str,
    sender: Option<&TaskSender>,
    task_id: &str,
) -> Result<()> {
    emit_log(sender, task_id, format!("Booting simulator {udid}"));
    let boot_args = ["simctl", "boot", udid];
    let boot_probe = run_command("xcrun", &boot_args, &[("DEVELOPER_DIR", developer_dir)]).await;
    if !boot_probe.success {
        let summary = boot_probe.summary();
        if summary.contains("current state: Booted") || summary.contains("already booted") {
            emit_log(sender, task_id, "Simulator was already booted");
        } else {
            bail!("simctl boot failed: {summary}");
        }
    }

    let bootstatus_args = ["simctl", "bootstatus", udid, "-b"];
    let bootstatus_probe = run_command_streamed(
        "xcrun",
        &bootstatus_args,
        &[("DEVELOPER_DIR", developer_dir)],
        task_id,
        sender,
    )
    .await;
    if !bootstatus_probe.success {
        bail!("simctl bootstatus failed: {}", bootstatus_probe.summary());
    }

    emit_log(sender, task_id, format!("Simulator booted: {udid}"));
    Ok(())
}

/// 打开 Simulator.app 并尽量聚焦到目标设备。
async fn open_simulator_app(
    developer_dir: &Path,
    udid: &str,
    sender: Option<&TaskSender>,
    task_id: &str,
) {
    let simulator_app = developer_dir.join("Applications/Simulator.app");
    let simulator_app = simulator_app.to_string_lossy();
    let args = [simulator_app.as_ref(), "--args", "-CurrentDeviceUDID", udid];
    let probe = run_command("open", &args, &[]).await;

    if probe.success {
        emit_log(sender, task_id, "Opened Simulator.app");
    } else {
        emit_log(
            sender,
            task_id,
            format!(
                "Simulator booted, but opening Simulator.app failed: {}",
                probe.summary()
            ),
        );
    }
}

/// 将版本号转换成可排序的数字 key。
fn version_key(version: &str) -> Vec<u32> {
    version
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u32>().unwrap_or_default())
        .collect()
}

/// 运行 PATH 中的命令并收集输出。
async fn run_command(program: &str, args: &[&str], envs: &[(&str, &Path)]) -> CommandProbe {
    let mut command = Command::new(program);
    command.args(args);

    for (key, value) in envs {
        command.env(key, value);
    }

    finish_probe(command).await
}

/// 运行指定路径的命令并收集输出。
async fn run_path_command(program: &Path, args: &[&str], envs: &[(&str, &Path)]) -> CommandProbe {
    let mut command = Command::new(program);
    command.args(args);

    for (key, value) in envs {
        command.env(key, value);
    }

    finish_probe(command).await
}

/// 运行 PATH 中的命令，并把 stdout / stderr 按行转成任务日志。
async fn run_command_streamed(
    program: &str,
    args: &[&str],
    envs: &[(&str, &Path)],
    task_id: &str,
    sender: Option<&TaskSender>,
) -> CommandProbe {
    let mut command = Command::new(program);
    command.args(args);

    for (key, value) in envs {
        command.env(key, value);
    }

    finish_probe_streamed(command, program, args, task_id, sender).await
}

/// 运行指定路径的命令，并把 stdout / stderr 按行转成任务日志。
async fn run_path_command_streamed(
    program: &Path,
    args: &[&str],
    envs: &[(&str, &Path)],
    task_id: &str,
    sender: Option<&TaskSender>,
) -> CommandProbe {
    let mut command = Command::new(program);
    command.args(args);

    for (key, value) in envs {
        command.env(key, value);
    }

    let program_label = program.display().to_string();
    finish_probe_streamed(command, &program_label, args, task_id, sender).await
}

/// 等待命令结束并收集完整输出。
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
    args: &[&str],
    task_id: &str,
    sender: Option<&TaskSender>,
) -> CommandProbe {
    emit_log(
        sender,
        task_id,
        format!("Running: {program_label} {}", args.join(" ")),
    );

    command.stdout(Stdio::piped()).stderr(Stdio::piped());

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
            Ok(Some(line)) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                if let Some(sender) = &sender {
                    let _ = sender.send(crate::model::TaskEvent::Log {
                        id: task_id.clone(),
                        message: format!("{stream_name}: {line}"),
                    });
                }
                output.push(line);
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

/// 返回第一行非空输出。
fn first_meaningful_line(output: &str) -> Option<&str> {
    output.lines().map(str::trim).find(|line| !line.is_empty())
}
