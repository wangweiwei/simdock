//! Simdock的核心领域层。
//!
//! 这个crate不关心CLI或桌面UI，只负责描述模拟器领域模型、
//! 平台Provider接口，以及iOS/Android的诊断和安装工作流。

pub mod model;
pub mod provider;
pub mod service;

pub use model::{
    CreateProfileRequest, DeviceTemplate, DoctorCheck, DoctorReport, InstallRequest, Instance,
    InstanceState, Platform, Profile, Runtime, SimulatorDevice, TaskEvent, TaskState,
};
