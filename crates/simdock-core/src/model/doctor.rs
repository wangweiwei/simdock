use serde::{Deserialize, Serialize};

use crate::model::Platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 单个环境检查项。
///
/// `key` 是机器可读的稳定标识，`detail` 是给用户或日志展示的说明。
pub struct DoctorCheck {
    pub key: String,
    pub ready: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 某个平台的一次完整诊断结果。
pub struct DoctorReport {
    pub platform: Platform,
    pub ready: bool,
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    /// 根据检查项生成诊断报告。
    ///
    /// 只要有一个检查项未通过，整个平台就视为未就绪。
    pub fn from_checks(platform: Platform, checks: Vec<DoctorCheck>) -> Self {
        let ready = checks.iter().all(|check| check.ready);

        Self {
            platform,
            ready,
            checks,
        }
    }

    /// 生成一个占位的未就绪报告。
    ///
    /// 用于诊断尚未执行、平台仍在初始化，或需要给 UI 一个可展示状态时。
    pub fn pending(platform: Platform, detail: impl Into<String>) -> Self {
        Self::from_checks(
            platform,
            vec![DoctorCheck {
                key: "bootstrap".to_string(),
                ready: false,
                detail: detail.into(),
            }],
        )
    }
}
