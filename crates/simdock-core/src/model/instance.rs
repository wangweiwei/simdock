use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::Platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 模拟器实例的生命周期状态。
pub enum InstanceState {
    Idle,
    Installing,
    Ready,
    Starting,
    Running,
    Stopping,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 一次实际运行中的模拟器实例。
pub struct Instance {
    pub id: String,
    pub profile_id: String,
    pub platform: Platform,
    pub state: InstanceState,
    pub pid: Option<u32>,
    pub started_at: Option<DateTime<Utc>>,
}
