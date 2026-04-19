use serde::{Deserialize, Serialize};

use crate::model::Platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 一个可复用的模拟器启动配置。
pub struct Profile {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub runtime_id: String,
    pub device_template_id: String,
    pub extra: serde_json::Value,
}
