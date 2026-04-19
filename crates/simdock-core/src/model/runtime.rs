use serde::{Deserialize, Serialize};

use crate::model::Platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 一个模拟器运行时版本。
pub struct Runtime {
    pub id: String,
    pub platform: Platform,
    pub version: String,
    pub arch: String,
    pub installed: bool,
}
