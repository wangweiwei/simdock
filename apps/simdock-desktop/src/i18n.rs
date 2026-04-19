use std::fmt;

use simdock_core::Platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppLanguage {
    Chinese,
    English,
}

impl AppLanguage {
    pub(crate) fn is_english(self) -> bool {
        matches!(self, Self::English)
    }
}

impl fmt::Display for AppLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chinese => f.write_str("中文"),
            Self::English => f.write_str("English"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeMode {
    System,
    Light,
    Dark,
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => f.write_str("System"),
            Self::Light => f.write_str("Light"),
            Self::Dark => f.write_str("Dark"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThemeModeOption {
    pub(crate) mode: ThemeMode,
    label: &'static str,
}

impl fmt::Display for ThemeModeOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WindowTitleState {
    Loading,
    Ready,
    Failed,
}

pub(crate) fn theme_mode_options(language: AppLanguage) -> [ThemeModeOption; 3] {
    [
        theme_mode_option(ThemeMode::System, language),
        theme_mode_option(ThemeMode::Light, language),
        theme_mode_option(ThemeMode::Dark, language),
    ]
}

pub(crate) fn theme_mode_option(mode: ThemeMode, language: AppLanguage) -> ThemeModeOption {
    ThemeModeOption {
        mode,
        label: theme_mode_label(mode, language),
    }
}

pub(crate) fn theme_mode_label(mode: ThemeMode, language: AppLanguage) -> &'static str {
    match (mode, language) {
        (ThemeMode::System, AppLanguage::Chinese) => "跟随系统",
        (ThemeMode::Light, AppLanguage::Chinese) => "浅色",
        (ThemeMode::Dark, AppLanguage::Chinese) => "深色",
        (ThemeMode::System, AppLanguage::English) => "Follow system",
        (ThemeMode::Light, AppLanguage::English) => "Light",
        (ThemeMode::Dark, AppLanguage::English) => "Dark",
    }
}

pub(crate) fn window_title(
    language: AppLanguage,
    platform: Platform,
    state: WindowTitleState,
) -> String {
    let platform = platform_label(platform);

    match (language, state) {
        (AppLanguage::English, WindowTitleState::Loading) => {
            format!("Simdock | {platform} | Refreshing")
        }
        (AppLanguage::English, WindowTitleState::Ready) => format!("Simdock | {platform}"),
        (AppLanguage::English, WindowTitleState::Failed) => {
            format!("Simdock | {platform} | Attention needed")
        }
        (AppLanguage::Chinese, WindowTitleState::Loading) => {
            format!("Simdock | {platform} | 刷新中")
        }
        (AppLanguage::Chinese, WindowTitleState::Ready) => format!("Simdock | {platform}"),
        (AppLanguage::Chinese, WindowTitleState::Failed) => {
            format!("Simdock | {platform} | 需要处理")
        }
    }
}

pub(crate) fn app_title() -> &'static str {
    "Simdock"
}

pub(crate) fn header_subtitle(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => {
            "Readiness checks for iOS Simulator and Android Emulator on this Mac."
        }
        AppLanguage::Chinese => "检查这台 Mac 是否已准备好运行 iOS 模拟器和 Android 模拟器。",
    }
}

pub(crate) fn language_field_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Language",
        AppLanguage::Chinese => "语言",
    }
}

pub(crate) fn theme_field_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Theme",
        AppLanguage::Chinese => "主题",
    }
}

pub(crate) fn refresh_button_label(is_loading: bool, language: AppLanguage) -> &'static str {
    match (is_loading, language) {
        (true, AppLanguage::English) => "Checking...",
        (true, AppLanguage::Chinese) => "检测中...",
        (false, AppLanguage::English) => "Check again",
        (false, AppLanguage::Chinese) => "重新检测",
    }
}

pub(crate) fn status_loading_title(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Current run in progress",
        AppLanguage::Chinese => "正在运行诊断",
    }
}

pub(crate) fn status_loading_detail(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => {
            "Simdock is probing Xcode, simulator runtimes, Android SDK tools, and Java availability."
        }
        AppLanguage::Chinese => {
            "Simdock 正在检查 Xcode、模拟器运行时、Android SDK 工具和 Java 可用性。"
        }
    }
}

pub(crate) fn selected_ready_title(platform: Platform, language: AppLanguage) -> &'static str {
    match (platform, language) {
        (Platform::Ios, AppLanguage::English) => "iOS Simulator is ready",
        (Platform::Android, AppLanguage::English) => "Android Emulator is ready",
        (Platform::Ios, AppLanguage::Chinese) => "iOS 模拟器已就绪",
        (Platform::Android, AppLanguage::Chinese) => "Android 模拟器已就绪",
    }
}

pub(crate) fn selected_ready_detail(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => {
            "The selected platform passed every environment check. Switch tabs to inspect the other platform."
        }
        AppLanguage::Chinese => "当前平台已经通过全部诊断项。你可以切换 Tab 查看另一个平台。",
    }
}

pub(crate) fn selected_attention_title(platform: Platform, language: AppLanguage) -> &'static str {
    match (platform, language) {
        (Platform::Ios, AppLanguage::English) => "iOS setup needs attention",
        (Platform::Android, AppLanguage::English) => "Android setup needs attention",
        (Platform::Ios, AppLanguage::Chinese) => "iOS 环境需要处理",
        (Platform::Android, AppLanguage::Chinese) => "Android 环境需要处理",
    }
}

pub(crate) fn selected_attention_detail(platform: Platform, language: AppLanguage) -> &'static str {
    match (platform, language) {
        (Platform::Ios, AppLanguage::English) => {
            "Review Xcode availability, license acceptance, and installed iOS simulator runtimes."
        }
        (Platform::Android, AppLanguage::English) => {
            "Review Java, Android SDK tools, emulator binaries, and Android system images."
        }
        (Platform::Ios, AppLanguage::Chinese) => {
            "请检查 Xcode 是否可用、许可证是否已接受，以及是否已安装 iOS 模拟器运行时。"
        }
        (Platform::Android, AppLanguage::Chinese) => {
            "请检查 Java、Android SDK 工具、模拟器二进制文件和 Android 系统镜像。"
        }
    }
}

pub(crate) fn no_diagnostics_title(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "No diagnostics loaded for this platform",
        AppLanguage::Chinese => "当前平台没有诊断结果",
    }
}

pub(crate) fn no_diagnostics_detail(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Refresh the doctor run to repopulate platform diagnostics.",
        AppLanguage::Chinese => "刷新诊断后会重新加载当前平台的环境信息。",
    }
}

pub(crate) fn doctor_failed_title(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Environment check failed",
        AppLanguage::Chinese => "环境检测失败",
    }
}

pub(crate) fn action_opening_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Opening...",
        AppLanguage::Chinese => "打开中...",
    }
}

pub(crate) fn action_installing_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Installing...",
        AppLanguage::Chinese => "安装中...",
    }
}

pub(crate) fn action_open_simulator_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Open simulator",
        AppLanguage::Chinese => "打开模拟器",
    }
}

pub(crate) fn action_one_click_install_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "One-click install",
        AppLanguage::Chinese => "一键安装",
    }
}

pub(crate) fn install_panel_title(platform: Platform, language: AppLanguage) -> String {
    match language {
        AppLanguage::English => format!("{} install", platform_label(platform)),
        AppLanguage::Chinese => format!("{} 安装", platform_label(platform)),
    }
}

pub(crate) fn install_hint(platform: Platform, language: AppLanguage) -> &'static str {
    match (platform, language) {
        (Platform::Ios, AppLanguage::English) => {
            "Runs the iOS simulator dependency workflow. Xcode itself still needs to come from Apple."
        }
        (Platform::Android, AppLanguage::English) => {
            "Runs the managed Android emulator dependency workflow under Simdock paths."
        }
        (Platform::Ios, AppLanguage::Chinese) => {
            "执行 iOS 模拟器依赖流程。Xcode 本体仍需要来自 Apple 官方。"
        }
        (Platform::Android, AppLanguage::Chinese) => {
            "在 Simdock 托管目录下执行 Android 模拟器依赖流程。"
        }
    }
}

pub(crate) fn install_stage_titles(platform: Platform, language: AppLanguage) -> [&'static str; 4] {
    match (platform, language) {
        (Platform::Ios, AppLanguage::English) => ["Xcode", "Tools", "Auth", "Simulator"],
        (Platform::Android, AppLanguage::English) => ["SDK", "Java", "Tools", "Emulator"],
        (Platform::Ios, AppLanguage::Chinese) => ["Xcode", "工具", "授权", "模拟器"],
        (Platform::Android, AppLanguage::Chinese) => ["SDK", "Java", "工具", "模拟器"],
    }
}

pub(crate) fn live_logs_title(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Live logs",
        AppLanguage::Chinese => "实时日志",
    }
}

pub(crate) fn empty_install_logs(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "No install logs yet. Start an install task to see live output.",
        AppLanguage::Chinese => "还没有安装日志。点击一键安装后会实时显示。",
    }
}

pub(crate) fn report_title(platform: Platform, language: AppLanguage) -> &'static str {
    match (platform, language) {
        (Platform::Ios, AppLanguage::English) => "iOS Simulator",
        (Platform::Android, AppLanguage::English) => "Android Emulator",
        (Platform::Ios, AppLanguage::Chinese) => "iOS 模拟器",
        (Platform::Android, AppLanguage::Chinese) => "Android 模拟器",
    }
}

pub(crate) fn install_ready_to_start(platform: Platform, language: AppLanguage) -> String {
    match language {
        AppLanguage::English => format!("Ready to install {}", report_title(platform, language)),
        AppLanguage::Chinese => format!("可以开始安装{}", report_title(platform, language)),
    }
}

pub(crate) fn install_starting_message(platform: Platform, language: AppLanguage) -> String {
    match language {
        AppLanguage::English => {
            format!(
                "Starting one-click install for {}",
                report_title(platform, language)
            )
        }
        AppLanguage::Chinese => format!("正在启动{}一键安装", report_title(platform, language)),
    }
}

pub(crate) fn opening_simulator_message(platform: Platform, language: AppLanguage) -> String {
    match language {
        AppLanguage::English => format!("Opening {}", report_title(platform, language)),
        AppLanguage::Chinese => format!("正在打开{}", report_title(platform, language)),
    }
}

pub(crate) fn simulator_opened_message(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Simulator opened",
        AppLanguage::Chinese => "模拟器已打开",
    }
}

pub(crate) fn xcode_license_waiting_message(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => {
            "Xcode license requires confirmation. Waiting for macOS authorization..."
        }
        AppLanguage::Chinese => "Xcode 许可证需要确认。正在等待 macOS 授权...",
    }
}

pub(crate) fn xcode_license_cancelled_message(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => {
            "Xcode license confirmation was cancelled; iOS installation cannot continue yet."
        }
        AppLanguage::Chinese => "已取消 Xcode 许可证确认，暂时无法继续 iOS 安装。",
    }
}

pub(crate) fn xcode_license_command_failed(error: &str, language: AppLanguage) -> String {
    match language {
        AppLanguage::English => format!("Xcode license command failed: {error}"),
        AppLanguage::Chinese => format!("Xcode 许可证命令执行失败：{error}"),
    }
}

pub(crate) fn install_completed_message(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Install task completed",
        AppLanguage::Chinese => "安装任务已完成",
    }
}

pub(crate) fn install_message(message: &str, language: AppLanguage) -> String {
    if language.is_english() {
        message.to_string()
    } else {
        localize_install_message(message)
    }
}

pub(crate) fn xcode_license_dialog_text(shell_command: &str, language: AppLanguage) -> String {
    match language {
        AppLanguage::English => format!(
            "Simdock will run this command to accept the Xcode license:\n\n{shell_command}\n\nContinue only if you agree to Apple's Xcode license. macOS will ask for administrator authorization next."
        ),
        AppLanguage::Chinese => format!(
            "Simdock 将执行以下命令来接受 Xcode 许可证：\n\n{shell_command}\n\n只有在你同意 Apple 的 Xcode 许可证时才继续。下一步 macOS 会请求管理员授权。"
        ),
    }
}

pub(crate) fn continue_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Continue",
        AppLanguage::Chinese => "继续",
    }
}

pub(crate) fn cancel_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Cancel",
        AppLanguage::Chinese => "取消",
    }
}

pub(crate) fn platform_label(platform: Platform) -> &'static str {
    match platform {
        Platform::Ios => "iOS",
        Platform::Android => "Android",
    }
}

fn localize_install_message(message: &str) -> String {
    if let Some(version) = message
        .strip_prefix("Prepare iOS simulator dependencies (")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("准备 iOS 模拟器依赖（{version}）");
    }

    if let Some(api) = message
        .strip_prefix("Prepare Android emulator dependencies (API ")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("准备 Android 模拟器依赖（API {api}）");
    }

    if let Some(version) = message.strip_prefix("Preparing runtime request for ") {
        return format!("正在准备运行时请求：{version}");
    }

    if let Some(api) = message.strip_prefix("Preparing system image request for API ") {
        return format!("正在准备 Android 系统镜像请求：API {api}");
    }

    if let Some(device) = message.strip_prefix("Recommended simulator device: ") {
        return format!("推荐的模拟器设备：{device}");
    }

    if let Some(device) = message.strip_prefix("Recommended virtual device: ") {
        return format!("推荐的虚拟设备：{device}");
    }

    if let Some(path) = message.strip_prefix("Using Xcode developer directory: ") {
        return format!("正在使用 Xcode 开发者目录：{path}");
    }

    if let Some(output) = message.strip_prefix("Xcode license check output: ") {
        return format!("Xcode 许可证检查输出：{output}");
    }

    if let Some(output) =
        message.strip_prefix("xcodebuild -downloadPlatform iOS returned an error: ")
    {
        return format!("xcodebuild -downloadPlatform iOS 返回错误：{output}");
    }

    if let Some(rest) = message.strip_prefix("Requested iOS ") {
        return format!("请求的 iOS {rest}");
    }

    if let Some(rest) = message.strip_prefix("Found iOS runtime ") {
        return format!("已找到 iOS 运行时 {rest}");
    }

    if let Some(device) = message.strip_prefix("Using simulator device type: ") {
        return format!("使用模拟器设备类型：{device}");
    }

    if let Some(device) = message.strip_prefix("Reusing simulator ") {
        return format!("复用模拟器 {device}");
    }

    if let Some(device) = message.strip_prefix("Creating simulator ") {
        return format!("正在创建模拟器 {device}");
    }

    if let Some(udid) = message.strip_prefix("Created simulator ") {
        return format!("已创建模拟器 {udid}");
    }

    if let Some(udid) = message.strip_prefix("Booting simulator ") {
        return format!("正在启动模拟器 {udid}");
    }

    if let Some(udid) = message.strip_prefix("Simulator is already booted: ") {
        return format!("模拟器已启动：{udid}");
    }

    if let Some(udid) = message.strip_prefix("Simulator booted: ") {
        return format!("模拟器已启动：{udid}");
    }

    if let Some(command) = message.strip_prefix("Running: ") {
        return format!("执行命令：{command}");
    }

    if let Some(output) = message.strip_prefix("stdout: ") {
        return format!("输出：{output}");
    }

    if let Some(output) = message.strip_prefix("stderr: ") {
        return format!("错误输出：{output}");
    }

    match message {
        "Checking Xcode.app and developer directory" => "正在检查 Xcode.app 和开发者目录".to_string(),
        "Checking Xcode license and simulator tool availability" => {
            "正在检查 Xcode 许可证和模拟器工具可用性".to_string()
        }
        "Checking Xcode license acceptance" => "正在检查 Xcode 许可证是否已接受".to_string(),
        "Xcode license has been accepted" => "Xcode 许可证已接受".to_string(),
        "Xcode license has not been accepted. Open Terminal and run: sudo xcodebuild -license accept" => {
            "Xcode 许可证尚未接受。请打开终端执行：sudo xcodebuild -license accept".to_string()
        }
        "Running xcodebuild -runFirstLaunch" => {
            "正在执行 xcodebuild -runFirstLaunch".to_string()
        }
        "Downloading iOS simulator platform with xcodebuild -downloadPlatform iOS" => {
            "正在通过 xcodebuild -downloadPlatform iOS 下载 iOS 模拟器平台".to_string()
        }
        "Continuing only if the requested iOS runtime is already available" => {
            "仅当目标 iOS 运行时已存在时继续执行".to_string()
        }
        "Detecting installed iOS simulator runtimes with simctl" => {
            "正在通过 simctl 检测已安装的 iOS 模拟器运行时".to_string()
        }
        "Resolving target iOS simulator device type" => {
            "正在解析目标 iOS 模拟器设备类型".to_string()
        }
        "Creating or reusing target iOS simulator" => {
            "正在创建或复用目标 iOS 模拟器".to_string()
        }
        "No iOS simulator device type is available from simctl" => {
            "simctl 没有返回可用的 iOS 模拟器设备类型".to_string()
        }
        "Booting target iOS simulator" => "正在启动目标 iOS 模拟器".to_string(),
        "Simulator was already booted" => "模拟器已经处于启动状态".to_string(),
        "Opened Simulator.app" => "已打开 Simulator.app".to_string(),
        "iOS simulator is installed and running" => {
            "iOS 模拟器已安装并正在运行".to_string()
        }
        "Waiting for xcodebuild runtime install wiring" => {
            "等待接入 xcodebuild 运行时安装流程".to_string()
        }
        "No simulator device selected yet" => "还没有选择模拟器设备".to_string(),
        "Resolving Android SDK root and managed directories" => {
            "正在解析 Android SDK 根目录和托管目录".to_string()
        }
        "Checking Java runtime and command-line tools" => {
            "正在检查 Java 运行时和命令行工具".to_string()
        }
        "Waiting for sdkmanager / avdmanager install wiring" => {
            "等待接入 sdkmanager / avdmanager 安装流程".to_string()
        }
        "No virtual device selected yet" => "还没有选择虚拟设备".to_string(),
        "iOS runtime installation is not implemented yet; next step is wiring xcodebuild -runFirstLaunch and runtime download commands" => {
            "iOS 运行时安装尚未实现；下一步会接入 xcodebuild -runFirstLaunch 和运行时下载命令。".to_string()
        }
        "Android runtime installation is not implemented yet; next step is wiring cmdline-tools download, sdkmanager packages, and AVD creation" => {
            "Android 运行时安装尚未实现；下一步会接入 cmdline-tools 下载、sdkmanager 包安装和 AVD 创建。".to_string()
        }
        other => other.to_string(),
    }
}
