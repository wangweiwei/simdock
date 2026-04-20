use serde::{Deserialize, Serialize};

use crate::model::Platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 一个已存在的模拟器设备。
///
/// 该模型描述系统当前可见的设备实例，不区分是否由Simdock创建。
pub struct SimulatorDevice {
    pub id: String,
    pub platform: Platform,
    pub name: String,
    pub runtime_id: String,
    pub runtime_name: String,
    pub runtime_version: String,
    pub state: String,
    pub available: bool,
}

impl SimulatorDevice {
    /// 返回用于UI选择列表的紧凑展示名。
    ///
    /// 示例：`iPhone 16/iOS 26.4/Shutdown`。
    pub fn display_label(&self) -> String {
        format!(
            "{}/{}/{}",
            self.normalized_device_name(),
            self.runtime_name,
            self.state
        )
    }

    /// 规范化设备名，避免Simdock早期创建的设备重复展示runtime。
    fn normalized_device_name(&self) -> String {
        self.name
            .strip_prefix("Simdock ")
            .unwrap_or(&self.name)
            .strip_suffix(&format!(" {}", self.runtime_name))
            .unwrap_or_else(|| self.name.strip_prefix("Simdock ").unwrap_or(&self.name))
            .to_string()
    }
}
