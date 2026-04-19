pub mod android;
pub mod ios;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::model::{
    CreateProfileRequest, DeviceTemplate, DoctorReport, InstallRequest, Instance, Platform,
    Profile, Runtime, TaskEvent,
};

pub type TaskSender = mpsc::UnboundedSender<TaskEvent>;

/// 平台能力抽象。
///
/// iOS 和 Android 的安装、诊断、启动方式差异很大，但上层 CLI / GUI
/// 只需要依赖这组统一能力。Provider 负责把平台细节转换成稳定的领域模型。
#[async_trait]
pub trait PlatformProvider: Send + Sync {
    /// 返回当前 Provider 对应的平台。
    fn platform(&self) -> Platform;

    /// 执行平台环境诊断。
    ///
    /// 该方法只做探测，不应该修改用户环境；需要安装或授权的动作放到
    /// `install_runtime` 或显式确认流程里执行。
    async fn doctor(&self) -> anyhow::Result<DoctorReport>;

    /// 列出当前平台可识别的运行时版本。
    async fn list_runtimes(&self) -> anyhow::Result<Vec<Runtime>>;

    /// 列出当前平台可创建模拟器的设备模板。
    async fn list_device_templates(&self) -> anyhow::Result<Vec<DeviceTemplate>>;

    /// 安装或准备目标运行时。
    ///
    /// `task_sender` 用于向 GUI / CLI 发送进度、日志和失败原因；核心流程
    /// 不直接依赖任何具体界面。
    async fn install_runtime(
        &self,
        request: InstallRequest,
        task_sender: Option<TaskSender>,
    ) -> anyhow::Result<()>;

    /// 创建一个可复用的模拟器启动配置。
    async fn create_profile(&self, request: CreateProfileRequest) -> anyhow::Result<Profile>;

    /// 根据 profile 启动模拟器实例。
    async fn start(
        &self,
        profile: &Profile,
        task_sender: Option<TaskSender>,
    ) -> anyhow::Result<Instance>;

    /// 停止正在运行的模拟器实例。
    async fn stop(&self, instance: &Instance) -> anyhow::Result<()>;
}
