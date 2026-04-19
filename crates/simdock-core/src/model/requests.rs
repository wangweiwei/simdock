use crate::model::Platform;

#[derive(Debug, Clone)]
/// 安装或准备模拟器运行时的请求。
pub struct InstallRequest {
    pub platform: Platform,
    pub runtime_version: String,
    pub device_name: Option<String>,
}

#[derive(Debug, Clone)]
/// 创建模拟器 profile 的请求。
///
/// Profile 是运行时、设备模板和用户命名配置的组合。
pub struct CreateProfileRequest {
    pub name: String,
    pub platform: Platform,
    pub runtime_id: String,
    pub device_template_id: String,
}
