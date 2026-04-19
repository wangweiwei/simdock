use serde::{Deserialize, Serialize};

use crate::model::Platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 可用于创建模拟器的设备模板。
pub struct DeviceTemplate {
    pub id: String,
    pub platform: Platform,
    pub name: String,
    pub arch: String,
}
