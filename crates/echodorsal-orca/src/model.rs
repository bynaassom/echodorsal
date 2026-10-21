//! Minimal read models. Unknown JSON fields are intentionally ignored.

use serde::{Deserialize, Deserializer};

macro_rules! string_state {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $name {
            $($variant,)+
            Unknown(String),
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Ok(match String::deserialize(deserializer)?.as_str() {
                    $($wire => Self::$variant,)+
                    other => Self::Unknown(other.to_owned()),
                })
            }
        }
    };
}

string_state!(RuntimeState { Ready => "ready", Starting => "starting", Stopped => "stopped" });
string_state!(WorktreeState {
    Todo => "todo", InProgress => "in-progress", InReview => "in-review", Completed => "completed"
});
string_state!(TaskState {
    Pending => "pending", Ready => "ready", Dispatched => "dispatched", Completed => "completed",
    Failed => "failed", Blocked => "blocked"
});
string_state!(GateState { Pending => "pending", Open => "open", Resolved => "resolved" });

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct WorktreeId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct GateId(pub String);

#[derive(Debug, Clone, Deserialize)]
pub struct AppStatus {
    pub running: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub state: RuntimeState,
    pub reachable: bool,
    pub app_version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrcaStatus {
    pub app: AppStatus,
    pub runtime: RuntimeStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub worktree_id: WorktreeId,
    pub display_name: Option<String>,
    pub workspace_status: Option<WorktreeState>,
    pub host_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostScope {
    pub host_ids: Vec<String>,
    pub omitted_host_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktrees {
    pub worktrees: Vec<Worktree>,
    pub total_count: usize,
    pub truncated: bool,
    pub host_scope: HostScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct RunId(pub String);

/// Minimal task fields verified against a real explicit Run read.
#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    #[serde(alias = "taskId")]
    pub id: TaskId,
    pub status: TaskState,
    #[serde(rename = "task_title", alias = "title")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Gate {
    #[serde(alias = "gateId")]
    pub id: GateId,
    pub status: GateState,
    pub question: Option<String>,
}
