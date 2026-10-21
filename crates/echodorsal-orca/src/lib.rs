//! Read-only adapter for the stablyai Orca CLI.
//!
//! Connect once at startup to validate the command registry and record the version.
//! All subprocess calls use fixed argument lists and a three-second deadline.

mod model;

pub use model::*;
use serde::{Deserialize, de::DeserializeOwned};
use std::{future::Future, path::PathBuf, process::Stdio, time::Duration};
use thiserror::Error;
use tokio::{io::AsyncReadExt, process::Command, time::timeout};

pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_OUTPUT: u64 = 4 * 1024 * 1024;
const REQUIRED_COMMANDS: [&str; 5] = [
    "status",
    "agent-context",
    "worktree ps",
    "orchestration task-list",
    "orchestration gate-list",
];

/// Errors intentionally exclude raw stdout/stderr, which may contain private data.
#[derive(Debug, Error)]
pub enum OrcaError {
    #[error("Orca runtime is not running or reachable")]
    NotRunning,
    #[error("unsupported Orca CLI: {reason}")]
    Unsupported { reason: String },
    #[error("Orca command exceeded the three-second deadline")]
    Timeout,
    #[error("Orca returned invalid output")]
    BadOutput,
    #[error("Orca orchestration requires a bound Run")]
    RunRequired,
    #[error("Orca executable could not be started or read")]
    Io(#[source] std::io::Error),
    #[error("Orca command failed")]
    CommandFailed,
}

pub trait OrcaPort: Send + Sync {
    fn status(&self) -> impl Future<Output = Result<OrcaStatus, OrcaError>> + Send;
    fn worktrees(&self) -> impl Future<Output = Result<Worktrees, OrcaError>> + Send;
    fn tasks(&self) -> impl Future<Output = Result<Vec<Task>, OrcaError>> + Send;
    fn gates(&self) -> impl Future<Output = Result<Vec<Gate>, OrcaError>> + Send;
    fn tasks_for_run(
        &self,
        run: &RunId,
    ) -> impl Future<Output = Result<Vec<Task>, OrcaError>> + Send;
    fn gates_for_run(
        &self,
        run: &RunId,
    ) -> impl Future<Output = Result<Vec<Gate>, OrcaError>> + Send;
}

#[derive(Debug, Clone)]
pub struct Compatibility {
    pub version: String,
    /// False should be presented as a warning; it does not prevent reads.
    pub verified_version: bool,
}

#[derive(Debug, Clone)]
pub struct CliOrca {
    executable: PathBuf,
    compatibility: Compatibility,
}

impl CliOrca {
    /// Uses `ORCA_BIN` when configured, otherwise `orca` from PATH.
    ///
    /// # Errors
    /// Returns a typed error if runtime/identity discovery fails.
    pub async fn connect() -> Result<Self, OrcaError> {
        let executable =
            std::env::var_os("ORCA_BIN").map_or_else(|| PathBuf::from("orca"), PathBuf::from);
        Self::connect_with_bin(executable).await
    }

    /// # Errors
    /// Returns a typed error if runtime/identity discovery fails.
    pub async fn connect_with_bin(executable: impl Into<PathBuf>) -> Result<Self, OrcaError> {
        let mut client = Self {
            executable: executable.into(),
            compatibility: Compatibility {
                version: String::new(),
                verified_version: false,
            },
        };
        let status = client.status().await?;
        validate_registry(&client.execute(&["agent-context", "--json"]).await?)?;
        client.compatibility = Compatibility {
            verified_version: status.runtime.app_version == "1.4.237",
            version: status.runtime.app_version,
        };
        Ok(client)
    }

    #[must_use]
    pub fn compatibility(&self) -> &Compatibility {
        &self.compatibility
    }

    async fn execute(&self, args: &[&str]) -> Result<Vec<u8>, OrcaError> {
        // Drop/timeout kills the child. Stderr is deliberately discarded and stdout
        // is bounded so a broken/wrong CLI cannot exhaust memory or leak diagnostics.
        let mut child = Command::new(&self.executable)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(OrcaError::Io)?;
        let stdout = child.stdout.take().ok_or(OrcaError::BadOutput)?;
        let operation = async {
            let mut bytes = Vec::new();
            stdout
                .take(MAX_OUTPUT + 1)
                .read_to_end(&mut bytes)
                .await
                .map_err(OrcaError::Io)?;
            if bytes.len() as u64 > MAX_OUTPUT {
                return Err(OrcaError::BadOutput);
            }
            let status = child.wait().await.map_err(OrcaError::Io)?;
            // Orca emits structured errors even with a non-zero exit status.
            if !status.success() {
                return match parse_failure(&bytes) {
                    Some(error) => Err(error),
                    None => Err(OrcaError::CommandFailed),
                };
            }
            Ok(bytes)
        };
        timeout(COMMAND_TIMEOUT, operation)
            .await
            .map_err(|_| OrcaError::Timeout)?
    }

    async fn read<T: DeserializeOwned>(&self, args: &[&str]) -> Result<T, OrcaError> {
        parse_response(&self.execute(args).await?)
    }
}

impl OrcaPort for CliOrca {
    async fn status(&self) -> Result<OrcaStatus, OrcaError> {
        let result: serde_json::Value = self.read(&["status", "--json"]).await?;
        validate_status(result)
    }

    async fn worktrees(&self) -> Result<Worktrees, OrcaError> {
        self.read(&["worktree", "ps", "--json"]).await
    }

    async fn tasks(&self) -> Result<Vec<Task>, OrcaError> {
        let result: Tasks = self.read(&["orchestration", "task-list", "--json"]).await?;
        Ok(result.tasks)
    }

    async fn gates(&self) -> Result<Vec<Gate>, OrcaError> {
        let result: Gates = self.read(&["orchestration", "gate-list", "--json"]).await?;
        Ok(result.gates)
    }
    async fn tasks_for_run(&self, run: &RunId) -> Result<Vec<Task>, OrcaError> {
        let result: Tasks = self
            .read(&["orchestration", "task-list", "--run", &run.0, "--json"])
            .await?;
        Ok(result.tasks)
    }

    async fn gates_for_run(&self, run: &RunId) -> Result<Vec<Gate>, OrcaError> {
        let result: Gates = self
            .read(&["orchestration", "gate-list", "--run", &run.0, "--json"])
            .await?;
        Ok(result.gates)
    }
}

#[derive(Deserialize)]
struct Registry {
    commands: Vec<RegistryCommand>,
}
#[derive(Deserialize)]
struct RegistryCommand {
    command: String,
    flags: Vec<String>,
}
#[derive(Deserialize)]
struct Tasks {
    tasks: Vec<Task>,
}
#[derive(Deserialize)]
struct Gates {
    gates: Vec<Gate>,
}
#[derive(Deserialize)]
struct Failure {
    code: String,
}
#[derive(Deserialize)]
struct Envelope<T> {
    ok: bool,
    result: Option<T>,
    error: Option<Failure>,
}

fn failure(error: &Failure) -> OrcaError {
    match error.code.as_str() {
        "run_required" => OrcaError::RunRequired,
        "not_running"
        | "app_not_running"
        | "runtime_not_running"
        | "runtime_unavailable"
        | "runtime_not_reachable" => OrcaError::NotRunning,
        "unknown_command" | "unsupported_command" => OrcaError::Unsupported {
            reason: "required command is unavailable".into(),
        },
        _ => OrcaError::CommandFailed,
    }
}

fn parse_failure(bytes: &[u8]) -> Option<OrcaError> {
    let envelope: Envelope<serde_json::Value> = serde_json::from_slice(bytes).ok()?;
    if envelope.ok {
        return None;
    }
    envelope.error.as_ref().map(failure)
}

/// Parses the CLI envelope without depending on process execution.
///
/// # Errors
/// Returns `BadOutput` for malformed or incomplete success responses, or the
/// structured runtime error for unsuccessful responses.
pub fn parse_response<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, OrcaError> {
    let envelope: Envelope<serde_json::Value> =
        serde_json::from_slice(bytes).map_err(|_| OrcaError::BadOutput)?;
    if !envelope.ok {
        return Err(envelope
            .error
            .as_ref()
            .map_or(OrcaError::BadOutput, failure));
    }
    serde_json::from_value(envelope.result.ok_or(OrcaError::BadOutput)?)
        .map_err(|_| OrcaError::BadOutput)
}

fn validate_status(result: serde_json::Value) -> Result<OrcaStatus, OrcaError> {
    let running = result
        .pointer("/app/running")
        .and_then(serde_json::Value::as_bool)
        .ok_or(OrcaError::BadOutput)?;
    if !running {
        return Err(OrcaError::NotRunning);
    }
    let reachable = result
        .pointer("/runtime/reachable")
        .and_then(serde_json::Value::as_bool)
        .ok_or(OrcaError::BadOutput)?;
    if !reachable {
        return Err(OrcaError::NotRunning);
    }
    let status: OrcaStatus = serde_json::from_value(result).map_err(|_| OrcaError::BadOutput)?;
    if status.runtime.app_version.is_empty() {
        return Err(OrcaError::BadOutput);
    }
    Ok(status)
}

fn validate_registry(bytes: &[u8]) -> Result<(), OrcaError> {
    let registry: Registry = serde_json::from_slice(bytes).map_err(|_| OrcaError::Unsupported {
        reason: "missing command registry".into(),
    })?;
    let missing: Vec<_> = REQUIRED_COMMANDS
        .iter()
        .filter(|name| {
            !registry.commands.iter().any(|command| {
                command.command == **name
                    && command.flags.iter().any(|flag| flag == "json")
                    && (!name.starts_with("orchestration ")
                        || command.flags.iter().any(|flag| flag == "run"))
            })
        })
        .copied()
        .collect();
    if !missing.is_empty() {
        return Err(OrcaError::Unsupported {
            reason: format!(
                "missing commands or required JSON/run flags: {}",
                missing.join(", ")
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_status_does_not_require_live_version_fields() {
        assert!(matches!(
            validate_status(serde_json::json!({"app":{"running":false}})),
            Err(OrcaError::NotRunning)
        ));
        assert!(matches!(
            validate_status(
                serde_json::json!({"app":{"running":true}, "runtime":{"reachable":false}})
            ),
            Err(OrcaError::NotRunning)
        ));
        assert!(matches!(
            validate_status(
                serde_json::json!({"app":{"running":true}, "runtime":{"reachable":true}})
            ),
            Err(OrcaError::BadOutput)
        ));
    }

    #[test]
    fn registry_requires_exact_read_commands_and_json_support() {
        let original: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/orca/1.4.237/agent-context.json"
        ))
        .unwrap();
        assert!(validate_registry(&serde_json::to_vec(&original).unwrap()).is_ok());
        let mut missing = original.clone();
        missing["commands"].as_array_mut().unwrap().pop();
        assert!(matches!(
            validate_registry(&serde_json::to_vec(&missing).unwrap()),
            Err(OrcaError::Unsupported { .. })
        ));
        let mut wrong_command = original.clone();
        wrong_command["commands"][0]["command"] = "status-other".into();
        assert!(matches!(
            validate_registry(&serde_json::to_vec(&wrong_command).unwrap()),
            Err(OrcaError::Unsupported { .. })
        ));
        let mut no_run = original.clone();
        for command in no_run["commands"].as_array_mut().unwrap() {
            command["flags"] = serde_json::json!(["json"]);
        }
        assert!(matches!(
            validate_registry(&serde_json::to_vec(&no_run).unwrap()),
            Err(OrcaError::Unsupported { .. })
        ));
        let mut no_json = original;
        no_json["commands"][0]["flags"] = serde_json::json!([]);
        assert!(matches!(
            validate_registry(&serde_json::to_vec(&no_json).unwrap()),
            Err(OrcaError::Unsupported { .. })
        ));
        assert!(matches!(
            validate_registry(b"{}"),
            Err(OrcaError::Unsupported { .. })
        ));
    }
}
