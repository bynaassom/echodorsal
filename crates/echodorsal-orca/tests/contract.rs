use echodorsal_orca::{
    CliOrca, GateState, OrcaError, OrcaPort, OrcaStatus, RunId, Task, TaskState, WorktreeState,
    Worktrees, parse_response,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const STATUS: &[u8] = include_bytes!("../../../fixtures/orca/1.4.237/status.json");
const WORKTREES: &[u8] = include_bytes!("../../../fixtures/orca/1.4.237/worktrees.json");

fn fake(mode: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/support/fake-orca")
        .join(mode)
}

#[test]
fn real_sanitized_contract() {
    let status: OrcaStatus = parse_response(STATUS).unwrap();
    assert_eq!(status.runtime.app_version, "1.4.237");
    assert!(status.app.running);
    let worktrees: Worktrees = parse_response(WORKTREES).unwrap();
    assert_eq!(worktrees.worktrees.len(), 32);
    assert_eq!(worktrees.total_count, 32);
    assert!(!worktrees.truncated);
    assert_eq!(worktrees.host_scope.host_ids, ["local"]);
    assert_eq!(worktrees.host_scope.omitted_host_ids, Vec::<String>::new());
    assert!(
        worktrees
            .worktrees
            .iter()
            .any(|w| w.workspace_status == Some(WorktreeState::Completed))
    );
    for bytes in [
        include_bytes!("../../../fixtures/orca/1.4.237/tasks.json").as_slice(),
        include_bytes!("../../../fixtures/orca/1.4.237/gates.json").as_slice(),
    ] {
        assert!(matches!(
            parse_response::<serde_json::Value>(bytes),
            Err(OrcaError::RunRequired)
        ));
    }
}

#[test]
fn real_explicit_run_contract() {
    #[derive(serde::Deserialize)]
    struct Tasks {
        tasks: Vec<Task>,
        count: usize,
    }
    #[derive(serde::Deserialize)]
    struct Gates {
        gates: Vec<echodorsal_orca::Gate>,
        count: usize,
    }
    let tasks: Tasks = parse_response(include_bytes!(
        "../../../fixtures/orca/1.4.237/tasks-run.json"
    ))
    .unwrap();
    assert_eq!(tasks.count, 45);
    assert_eq!(tasks.tasks.len(), tasks.count);
    assert_eq!(tasks.tasks[0].title.as_deref(), Some("Task 1"));
    assert!(
        tasks
            .tasks
            .iter()
            .any(|task| task.status == TaskState::Blocked)
    );
    let gates: Gates = parse_response(include_bytes!(
        "../../../fixtures/orca/1.4.237/gates-run.json"
    ))
    .unwrap();
    assert_eq!(gates.count, 0);
    assert!(gates.gates.is_empty());
}

#[tokio::test]
async fn success_supports_only_the_fixed_read_surface() {
    let client = CliOrca::connect_with_bin(fake("ok")).await.unwrap();
    assert_eq!(client.compatibility().version, "1.4.237");
    assert!(client.compatibility().verified_version);
    assert!(client.status().await.unwrap().app.running);
    assert_eq!(client.worktrees().await.unwrap().total_count, 32);
    let run = RunId("run-demo".into());
    let tasks = client.tasks_for_run(&run).await.unwrap();
    assert_eq!(tasks.len(), 45);
    assert_eq!(tasks[0].title.as_deref(), Some("Task 1"));
    assert!(client.gates_for_run(&run).await.unwrap().is_empty());
    assert!(matches!(client.tasks().await, Err(OrcaError::RunRequired)));
    assert!(matches!(client.gates().await, Err(OrcaError::RunRequired)));
}

#[tokio::test]
async fn not_running_is_typed_even_on_nonzero_exit() {
    assert!(matches!(
        CliOrca::connect_with_bin(fake("not-running")).await,
        Err(OrcaError::NotRunning)
    ));
}

#[tokio::test]
async fn deadline_stops_a_slow_cli() {
    let start = Instant::now();
    assert!(matches!(
        CliOrca::connect_with_bin(fake("slow")).await,
        Err(OrcaError::Timeout)
    ));
    assert!(start.elapsed() >= Duration::from_secs(3));
    assert!(start.elapsed() < Duration::from_secs(6));
}

#[tokio::test]
async fn bad_json_is_rejected() {
    assert!(matches!(
        CliOrca::connect_with_bin(fake("bad-json")).await,
        Err(OrcaError::BadOutput)
    ));
}

#[tokio::test]
async fn new_fields_and_state_values_are_tolerated() {
    let client = CliOrca::connect_with_bin(fake("unknown-fields"))
        .await
        .unwrap();
    assert!(!client.compatibility().verified_version);
    assert_eq!(client.compatibility().version, "9.0.0");
    let run = RunId("run-demo".into());
    assert_eq!(
        client.tasks_for_run(&run).await.unwrap()[0].status,
        TaskState::Unknown("future-task-state".into())
    );
    assert_eq!(
        client.gates_for_run(&run).await.unwrap()[0].status,
        GateState::Unknown("future-gate-state".into())
    );
    let snapshot = client.worktrees().await.unwrap();
    assert_eq!(
        snapshot.worktrees[0].workspace_status,
        Some(WorktreeState::Unknown("future-state".into()))
    );
    assert!(snapshot.worktrees[0].display_name.is_none());
}

#[test]
fn missing_fields_and_unstructured_errors_are_not_empty_success() {
    for raw in [
        b"{}".as_slice(),
        b"{\"ok\":true,\"result\":{}}",
        b"{\"ok\":true}",
        b"{\"ok\":false}",
    ] {
        assert!(matches!(
            parse_response::<Worktrees>(raw),
            Err(OrcaError::BadOutput)
        ));
    }
    assert!(matches!(
        parse_response::<Worktrees>(b"{\"ok\":false,\"error\":{\"code\":\"unknown_command\"}}"),
        Err(OrcaError::Unsupported { .. })
    ));
}
