//! Simdock 的核心领域层。
//!
//! 这个 crate 不关心 CLI 或桌面 UI，只负责描述模拟器领域模型、
//! 平台 Provider 接口，以及 iOS / Android 的诊断和安装工作流。

pub mod model;
pub mod provider;
pub mod service;

pub use model::{
    CreateProfileRequest, DeviceTemplate, DoctorCheck, DoctorReport, InstallRequest, Instance,
    InstanceState, Platform, Profile, Runtime, TaskEvent, TaskState,
};
