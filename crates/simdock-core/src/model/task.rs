use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 长任务状态。
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Provider 向上层报告安装或启动进度的事件。
///
/// GUI 使用这些事件更新进度条和实时日志，CLI 后续也可以用它做流式输出。
pub enum TaskEvent {
    Started {
        id: String,
        title: String,
    },
    Progress {
        id: String,
        pct: f32,
        message: String,
    },
    Log {
        id: String,
        message: String,
    },
    Finished {
        id: String,
    },
    Failed {
        id: String,
        error: String,
    },
}
