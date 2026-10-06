use hm_fabric::supervisor::*;
use std::{collections::BTreeMap, net::TcpListener, path::PathBuf, time::{Duration, Instant}};
fn spec(command: &str, args: &[&str]) -> ProcessSpec {
    ProcessSpec { module_id: "worker".into(), command: command.into(), args: args.iter().map(|s| s.to_string()).collect(), env: BTreeMap::new(), cwd: PathBuf::from("/tmp"), readiness: Probe::ProcessAlive, health: Probe::ProcessAlive, readiness_timeout: Duration::from_secs(2), shutdown_timeout: Duration::from_millis(80), stderr_bytes: 8, drain_message: None, restart: RestartPolicy { max_restarts: 1, initial_backoff: Duration::from_millis(20), max_backoff: Duration::from_millis(40) } }
}
#[test]
fn real_child_environment_identity_stderr_and_eof_drain() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("identity");
    let mut declared = spec("/bin/sh", &["-c", "printf '%s|%s|%s|%s|%s' \"$HYPERMIND_MODULE_ID\" \"$HYPERMIND_LAUNCH_ID\" \"$HYPERMIND_GENERATION\" \"${HOME-unset}\" \"$PWD\" > \"$OUTPUT\"; printf 'abcdefghijklmnop' >&2; cat >/dev/null"]);
    declared.cwd = dir.path().to_path_buf(); declared.env.insert("OUTPUT".into(), output.to_string_lossy().into());
    let mut supervisor = Supervisor::new(); let identity = supervisor.start(declared).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while (!output.exists() || supervisor.active().unwrap().stderr_tail().len() < 8) && Instant::now() < deadline { std::thread::sleep(Duration::from_millis(5)); }
    let text = std::fs::read_to_string(output).unwrap();
    assert_eq!(text, format!("worker|{}|1|unset|{}", identity.launch_id, dir.path().display()));
    assert_eq!(supervisor.active().unwrap().stderr_tail(), b"ijklmnop");
    assert!(supervisor.active_mut().unwrap().healthy().unwrap());
    supervisor.shutdown().unwrap(); assert!(supervisor.active().is_none());
}
#[test]
fn replacement_waits_for_real_tcp_service_and_failed_candidate_preserves_old() {
    let socket = TcpListener::bind("127.0.0.1:0").unwrap(); let address = socket.local_addr().unwrap(); drop(socket);
    let mut supervisor = Supervisor::new(); let original = supervisor.start(spec("/bin/cat", &[])).unwrap();
    let mut failed = spec("/bin/cat", &[]); failed.readiness = Probe::Tcp(address); failed.readiness_timeout = Duration::from_millis(50);
    assert!(matches!(supervisor.replace(failed), Err(SupervisorError::ReadinessTimeout)));
    assert_eq!(supervisor.active().unwrap().identity, original);
    let port = address.port().to_string();
    let mut replacement = spec("/usr/bin/python3", &["-m", "http.server", &port, "--bind", "127.0.0.1"]); replacement.readiness = Probe::Tcp(address); replacement.health = Probe::Tcp(address);
    let next = supervisor.replace(replacement).unwrap(); assert_ne!(next.launch_id, original.launch_id); assert_eq!(next.generation, 2);
    assert!(supervisor.active_mut().unwrap().healthy().unwrap()); supervisor.shutdown().unwrap();
    assert!(!containment_capabilities().process_tree_termination); assert!(!containment_capabilities().filesystem_isolation);
}
#[test]
fn real_exit_restart_budget_and_bounded_force_shutdown() {
    let mut supervisor = Supervisor::new(); supervisor.start(spec("/bin/sleep", &["0.1"])).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let before = Instant::now(); assert!(supervisor.restart_if_unhealthy().unwrap().is_some()); assert!(before.elapsed() >= Duration::from_millis(20));
    std::thread::sleep(Duration::from_millis(150)); assert!(matches!(supervisor.restart_if_unhealthy(), Err(SupervisorError::RestartBudget))); supervisor.shutdown().unwrap();
    supervisor.start(spec("/bin/sleep", &["10"])).unwrap(); let before = Instant::now(); supervisor.shutdown().unwrap(); assert!(before.elapsed() < Duration::from_secs(1));
}
