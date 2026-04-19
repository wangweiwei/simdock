use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::process::Command;

#[derive(Debug, Clone)]
/// 一次命令执行请求。
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
/// 一次命令执行结果。
pub struct CommandOutput {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub struct TokioShellRunner;

#[async_trait]
/// 命令执行抽象。
///
/// 通过 trait 隔离真实 shell，后续测试可以替换为 fake runner。
pub trait ShellRunner {
    /// 执行命令并返回 stdout / stderr。
    async fn run(&self, spec: &CommandSpec) -> Result<CommandOutput>;
}

impl TokioShellRunner {
    /// 创建基于 Tokio 的命令执行器。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ShellRunner for TokioShellRunner {
    async fn run(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        let output = Command::new(&spec.program)
            .args(&spec.args)
            .output()
            .await
            .with_context(|| format!("failed to run {}", spec.program))?;

        Ok(CommandOutput {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
