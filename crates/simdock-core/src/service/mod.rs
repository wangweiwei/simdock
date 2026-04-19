use crate::{
    model::{DoctorReport, Platform},
    provider::PlatformProvider,
};

/// Simdock 的应用服务门面。
///
/// 这一层把 iOS / Android Provider 组合起来，给 CLI 和桌面端提供稳定入口。
/// 它不持有 UI 状态，也不直接执行用户交互。
pub struct SimdockService<I, A> {
    ios: I,
    android: A,
}

impl<I, A> SimdockService<I, A>
where
    I: PlatformProvider,
    A: PlatformProvider,
{
    /// 创建服务实例。
    pub fn new(ios: I, android: A) -> Self {
        Self { ios, android }
    }

    /// 同时运行 iOS 和 Android 的诊断。
    ///
    /// 当前实现按顺序执行，后续如果诊断耗时增加，可以在这里改成并发。
    pub async fn doctor_all(&self) -> anyhow::Result<Vec<DoctorReport>> {
        Ok(vec![self.ios.doctor().await?, self.android.doctor().await?])
    }

    /// 根据平台获取对应 Provider。
    pub fn provider_for(&self, platform: Platform) -> &dyn PlatformProvider {
        match platform {
            Platform::Ios => &self.ios,
            Platform::Android => &self.android,
        }
    }
}
