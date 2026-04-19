//! Simdock 的基础设施层。
//!
//! 这里放置与操作系统交互但不属于业务领域的能力，例如应用目录探测和命令执行。

pub mod paths;
pub mod shell;

pub use paths::AppPaths;
pub use shell::{CommandOutput, CommandSpec, ShellRunner, TokioShellRunner};
