use std::{collections::{BTreeMap, VecDeque}, io::{Read, Write}, net::{SocketAddr, TcpStream}, path::PathBuf, process::{Child, Command, Stdio}, sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use thiserror::Error;

static LAUNCH_SEQUENCE: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchIdentity { pub module_id: String, pub launch_id: String, pub generation: u64, pub pid: u32 }
#[derive(Clone, Debug)]
pub enum Probe { ProcessAlive, Tcp(SocketAddr) }
#[derive(Clone, Debug)]
pub struct ProcessSpec {
    pub module_id: String, pub command: PathBuf, pub args: Vec<String>, pub env: BTreeMap<String, String>, pub cwd: PathBuf,
    pub readiness: Probe, pub health: Probe, pub readiness_timeout: Duration, pub shutdown_timeout: Duration,
    pub stderr_bytes: usize, pub drain_message: Option<Vec<u8>>, pub restart: RestartPolicy,
}
#[derive(Clone, Debug)]
pub struct RestartPolicy { pub max_restarts: u32, pub initial_backoff: Duration, pub max_backoff: Duration }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessState { Starting, Ready, Draining, Exited }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentCapabilities { pub clean_environment: bool, pub direct_child_termination: bool, pub process_tree_termination: bool, pub resource_limits: bool, pub filesystem_isolation: bool }
pub fn containment_capabilities() -> ContainmentCapabilities { ContainmentCapabilities { clean_environment: true, direct_child_termination: true, process_tree_termination: false, resource_limits: false, filesystem_isolation: false } }
#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("invalid process declaration: {0}")] Invalid(String),
    #[error("process I/O: {0}")] Io(#[from] std::io::Error),
    #[error("readiness deadline exceeded")] ReadinessTimeout,
    #[error("child exited before readiness")] EarlyExit,
    #[error("restart budget exhausted")] RestartBudget,
    #[error("process is not ready")] NotReady,
    #[error("replacement module identity differs")] IdentityMismatch,
}
pub struct ManagedProcess { pub identity: LaunchIdentity, pub state: ProcessState, child: Child, stderr: Arc<Mutex<VecDeque<u8>>>, spec: ProcessSpec }
impl ManagedProcess {
    pub fn stderr_tail(&self) -> Vec<u8> { self.stderr.lock().unwrap_or_else(|e| e.into_inner()).iter().copied().collect() }
    pub fn healthy(&mut self) -> Result<bool, SupervisorError> { if self.state != ProcessState::Ready || self.child.try_wait()?.is_some() { return Ok(false); } Ok(probe(&self.spec.health)) }
    pub fn drain(&mut self) -> Result<(), SupervisorError> {
        if self.state == ProcessState::Exited { return Ok(()); }
        self.state = ProcessState::Draining;
        if let Some(message) = &self.spec.drain_message { if let Some(stdin) = self.child.stdin.as_mut() { stdin.write_all(message)?; stdin.flush()?; } }
        self.child.stdin.take();
        Ok(())
    }
    pub fn shutdown(&mut self) -> Result<(), SupervisorError> {
        let drain_result = self.drain();
        let deadline = Instant::now() + self.spec.shutdown_timeout;
        while self.child.try_wait()?.is_none() && Instant::now() < deadline { thread::sleep(Duration::from_millis(5)); }
        if self.child.try_wait()?.is_none() { self.child.kill()?; }
        self.child.wait()?; self.state = ProcessState::Exited;
        drain_result
    }
}
impl Drop for ManagedProcess { fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); } }
pub struct Supervisor { active: Option<ManagedProcess>, generation: u64, restarts: u32, last_restart: Option<Instant> }
impl Default for Supervisor { fn default() -> Self { Self::new() } }
impl Supervisor {
    pub fn new() -> Self { Self { active: None, generation: 0, restarts: 0, last_restart: None } }
    pub fn active(&self) -> Option<&ManagedProcess> { self.active.as_ref() }
    pub fn active_mut(&mut self) -> Option<&mut ManagedProcess> { self.active.as_mut() }
    pub fn start(&mut self, spec: ProcessSpec) -> Result<LaunchIdentity, SupervisorError> {
        if self.active.is_some() { return Err(SupervisorError::Invalid("already active".into())); }
        self.generation = self.generation.checked_add(1).ok_or_else(|| SupervisorError::Invalid("generation exhausted".into()))?;
        let process = launch(spec, self.generation)?; let identity = process.identity.clone(); self.active = Some(process); Ok(identity)
    }
    pub fn replace(&mut self, spec: ProcessSpec) -> Result<LaunchIdentity, SupervisorError> {
        let old = self.active.as_ref().ok_or(SupervisorError::NotReady)?;
        if old.identity.module_id != spec.module_id { return Err(SupervisorError::IdentityMismatch); }
        let generation = self.generation.checked_add(1).ok_or_else(|| SupervisorError::Invalid("generation exhausted".into()))?;
        let candidate = launch(spec, generation)?;
        self.active.as_mut().unwrap().shutdown()?;
        let identity = candidate.identity.clone(); self.active = Some(candidate); self.generation = generation; Ok(identity)
    }
    pub fn restart_if_unhealthy(&mut self) -> Result<Option<LaunchIdentity>, SupervisorError> {
        let current = self.active.as_mut().ok_or(SupervisorError::NotReady)?;
        if current.healthy()? { return Ok(None); }
        let spec = current.spec.clone();
        if self.restarts >= spec.restart.max_restarts { return Err(SupervisorError::RestartBudget); }
        let factor = 1u32.checked_shl(self.restarts.min(31)).unwrap_or(u32::MAX);
        let backoff = spec.restart.initial_backoff.saturating_mul(factor).min(spec.restart.max_backoff);
        if let Some(last) = self.last_restart { let elapsed = last.elapsed(); if elapsed < backoff { thread::sleep(backoff - elapsed); } } else { thread::sleep(backoff); }
        self.restarts += 1; self.last_restart = Some(Instant::now());
        current.shutdown()?; self.active.take(); self.start(spec).map(Some)
    }
    pub fn shutdown(&mut self) -> Result<(), SupervisorError> { if let Some(process) = self.active.as_mut() { process.shutdown()?; } self.active.take(); Ok(()) }
}
fn probe(probe: &Probe) -> bool { match probe { Probe::ProcessAlive => true, Probe::Tcp(address) => TcpStream::connect_timeout(address, Duration::from_millis(25)).is_ok() } }
fn launch(spec: ProcessSpec, generation: u64) -> Result<ManagedProcess, SupervisorError> {
    if spec.module_id.is_empty() || !spec.command.is_absolute() || !spec.cwd.is_absolute() || spec.readiness_timeout.is_zero() || spec.stderr_bytes > 16 * 1024 * 1024 || spec.restart.initial_backoff > spec.restart.max_backoff || spec.env.keys().any(|k| k.starts_with("HYPERMIND_") || k.is_empty() || k.contains(['=', '\0'])) { return Err(SupervisorError::Invalid("invalid command, environment or bounds".into())); }
    let sequence = LAUNCH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let launch_id = format!("{}-{}-{sequence}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos());
    let mut command = Command::new(&spec.command);
    command.args(&spec.args).env_clear().envs(&spec.env).env("HYPERMIND_MODULE_ID", &spec.module_id).env("HYPERMIND_LAUNCH_ID", &launch_id).env("HYPERMIND_GENERATION", generation.to_string()).current_dir(&spec.cwd).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stderr = Arc::new(Mutex::new(VecDeque::new())); let captured = stderr.clone(); let capacity = spec.stderr_bytes;
    if let Some(mut pipe) = child.stderr.take() { thread::spawn(move || { let mut buffer = [0u8; 4096]; while let Ok(count) = pipe.read(&mut buffer) { if count == 0 { break; } let mut tail = captured.lock().unwrap_or_else(|e| e.into_inner()); for byte in &buffer[..count] { if capacity > 0 { if tail.len() == capacity { tail.pop_front(); } tail.push_back(*byte); } } } }); }
    let identity = LaunchIdentity { module_id: spec.module_id.clone(), launch_id, generation, pid: child.id() };
    let mut process = ManagedProcess { identity, state: ProcessState::Starting, child, stderr, spec };
    let deadline = Instant::now() + process.spec.readiness_timeout;
    loop { if process.child.try_wait()?.is_some() { return Err(SupervisorError::EarlyExit); } if probe(&process.spec.readiness) { process.state = ProcessState::Ready; return Ok(process); } if Instant::now() >= deadline { return Err(SupervisorError::ReadinessTimeout); } thread::sleep(Duration::from_millis(5)); }
}
