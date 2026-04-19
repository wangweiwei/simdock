use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Simdock当前支持的平台。
pub enum Platform {
    Ios,
    Android,
}

impl Platform {
    /// 返回用于CLI、JSON和日志输出的稳定平台标识。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ios" => Ok(Self::Ios),
            "android" => Ok(Self::Android),
            other => Err(format!("unsupported platform: {other}")),
        }
    }
}
