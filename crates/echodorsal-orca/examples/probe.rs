//! Read-only manual verification. Prints counts and version only, never project data.
use echodorsal_orca::{CliOrca, OrcaPort, RunId};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orca = CliOrca::connect().await?;
    let worktrees = orca.worktrees().await?;
    println!(
        "version={} verified={} returned={} total={} truncated={} omitted_hosts={}",
        orca.compatibility().version,
        orca.compatibility().verified_version,
        worktrees.worktrees.len(),
        worktrees.total_count,
        worktrees.truncated,
        worktrees.host_scope.omitted_host_ids.len()
    );
    let run = std::env::var("ORCA_RUN_ID").ok().map(RunId);
    let tasks = match &run {
        Some(run) => orca.tasks_for_run(run).await,
        None => orca.tasks().await,
    };
    let gates = match &run {
        Some(run) => orca.gates_for_run(run).await,
        None => orca.gates().await,
    };
    match tasks {
        Ok(tasks) => println!("tasks={}", tasks.len()),
        Err(error) => println!("tasks: {error}"),
    }
    match gates {
        Ok(gates) => println!("gates={}", gates.len()),
        Err(error) => println!("gates: {error}"),
    }
    Ok(())
}
