use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;

#[derive(Debug, Clone)]
/// Simdock在本机使用的目录集合。
///
/// 所有托管下载、日志和Android SDK/AVD数据都应该通过这里解析，
/// 避免各层散落硬编码路径。
pub struct AppPaths {
    pub app_support_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub android_sdk_root: PathBuf,
    pub android_avd_root: PathBuf,
}

impl AppPaths {
    /// 根据macOS应用目录规范探测Simdock的本地目录。
    pub fn detect() -> Result<Self> {
        let project_dirs = ProjectDirs::from("com", "simdock", "Simdock")
            .context("unable to determine application directories")?;
        let app_support_dir = project_dirs.data_local_dir().to_path_buf();
        let cache_dir = project_dirs.cache_dir().to_path_buf();
        let logs_dir = app_support_dir.join("logs");
        let android_sdk_root = app_support_dir.join("android-sdk");
        let android_avd_root = app_support_dir.join("avd");

        Ok(Self {
            app_support_dir,
            cache_dir,
            logs_dir,
            android_sdk_root,
            android_avd_root,
        })
    }
}
