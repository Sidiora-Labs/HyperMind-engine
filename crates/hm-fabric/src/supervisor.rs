use std::{collections::{BTreeMap, VecDeque}, io::{Read, Write}, net::{SocketAddr, TcpStream}, path::PathBuf, process::{Child, Command, Stdio}, sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use thiserror::Error;
use crate::containment::{ContainmentPolicy, ContainmentError};
#[cfg(target_os = "linux")]
use crate::containment::OwnedProcessTree;

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
    #[error(transparent)] Containment(#[from] ContainmentError),
    #[error("invalid process declaration: {0}")] Invalid(String),
    #[error("process I/O: {0}")] Io(#[from] std::io::Error),
    #[error("readiness deadline exceeded")] ReadinessTimeout,
    #[error("contained launcher exited before readiness: {0}")] ContainedLaunch(String),
    #[error("child exited before readiness")] EarlyExit,
    #[error("restart budget exhausted")] RestartBudget,
    #[error("process is not ready")] NotReady,
    #[error("replacement module identity differs")] IdentityMismatch,
}
pub struct ManagedProcess { pub identity: LaunchIdentity, pub state: ProcessState, child: Child, stderr: Arc<Mutex<VecDeque<u8>>>, spec: ProcessSpec, policy: Option<ContainmentPolicy>, #[cfg(target_os = "linux")] tree: Option<OwnedProcessTree>, stderr_reader: Option<thread::JoinHandle<()>> }
impl ManagedProcess {
    fn finish_pipe(&mut self, timeout: Duration) -> Result<(), SupervisorError> {
        let deadline = Instant::now() + timeout;
        while self.stderr_reader.as_ref().is_some_and(|reader| !reader.is_finished()) {
            if Instant::now() >= deadline { return Err(ContainmentError::Unsupported("owned stderr pipe did not close within deadline".into()).into()); }
            thread::sleep(Duration::from_millis(5));
        }
        if let Some(reader) = self.stderr_reader.take() { let _ = reader.join(); }
        Ok(())
    }
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
        #[cfg(target_os = "linux")]
        if let Some(tree) = &self.tree { tree.terminate()?; }
        if self.child.try_wait()?.is_none() { self.child.kill()?; }
        self.child.wait()?;
        #[cfg(target_os = "linux")]
        if let Some(tree) = &self.tree { tree.reap_init(self.spec.shutdown_timeout.max(Duration::from_secs(2)))?; self.finish_pipe(Duration::from_secs(2))?; }
        self.state = ProcessState::Exited;
        drain_result
    }
}
impl Drop for ManagedProcess { fn drop(&mut self) {
    #[cfg(target_os = "linux")]
    if let Some(tree) = &self.tree { let _ = tree.terminate(); }
    let _ = self.child.kill(); let _ = self.child.wait();
    #[cfg(target_os = "linux")]
    if let Some(tree) = &self.tree { let _ = tree.reap_init(Duration::from_secs(2)); let _ = self.finish_pipe(Duration::from_secs(2)); }
} }
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
    pub fn start_contained(&mut self, spec: ProcessSpec, policy: ContainmentPolicy) -> Result<LaunchIdentity, SupervisorError> {
        if self.active.is_some() { return Err(SupervisorError::Invalid("already active".into())); }
        let generation = self.generation.checked_add(1).ok_or_else(|| SupervisorError::Invalid("generation exhausted".into()))?;
        let process = launch_candidate(spec, generation, policy)?;
        let identity = process.identity.clone(); self.active = Some(process); self.generation = generation; Ok(identity)
    }
    pub fn adopt_candidate(&mut self, candidate: ManagedProcess) -> Result<LaunchIdentity, SupervisorError> {
        if candidate.state != ProcessState::Ready { return Err(SupervisorError::NotReady); }
        if let Some(old) = self.active.as_mut() { if old.identity.module_id != candidate.identity.module_id { return Err(SupervisorError::IdentityMismatch); } old.shutdown()?; }
        let identity = candidate.identity.clone(); self.generation = identity.generation; self.active = Some(candidate); Ok(identity)
    }
    pub fn replace(&mut self, spec: ProcessSpec) -> Result<LaunchIdentity, SupervisorError> {
        let old = self.active.as_ref().ok_or(SupervisorError::NotReady)?;
        if old.identity.module_id != spec.module_id { return Err(SupervisorError::IdentityMismatch); }
        let generation = self.generation.checked_add(1).ok_or_else(|| SupervisorError::Invalid("generation exhausted".into()))?;
        let policy = old.policy.clone();
        let candidate = launch_with_policy(spec, generation, policy)?;
        self.active.as_mut().unwrap().shutdown()?;
        let identity = candidate.identity.clone(); self.active = Some(candidate); self.generation = generation; Ok(identity)
    }
    pub fn restart_if_unhealthy(&mut self) -> Result<Option<LaunchIdentity>, SupervisorError> {
        let current = self.active.as_mut().ok_or(SupervisorError::NotReady)?;
        if current.healthy()? { return Ok(None); }
        let spec = current.spec.clone();
        let policy = current.policy.clone();
        if self.restarts >= spec.restart.max_restarts { return Err(SupervisorError::RestartBudget); }
        let factor = 1u32.checked_shl(self.restarts.min(31)).unwrap_or(u32::MAX);
        let backoff = spec.restart.initial_backoff.saturating_mul(factor).min(spec.restart.max_backoff);
        if let Some(last) = self.last_restart { let elapsed = last.elapsed(); if elapsed < backoff { thread::sleep(backoff - elapsed); } } else { thread::sleep(backoff); }
        self.restarts += 1; self.last_restart = Some(Instant::now());
        current.shutdown()?; self.active.take(); match policy { Some(policy) => self.start_contained(spec, policy).map(Some), None => self.start(spec).map(Some) }
    }
    pub fn shutdown(&mut self) -> Result<(), SupervisorError> { if let Some(process) = self.active.as_mut() { process.shutdown()?; } self.active.take(); Ok(()) }
}
fn probe(probe: &Probe) -> bool { match probe { Probe::ProcessAlive => true, Probe::Tcp(address) => TcpStream::connect_timeout(address, Duration::from_millis(25)).is_ok() } }
pub fn launch_candidate(spec: ProcessSpec, generation: u64, policy: ContainmentPolicy) -> Result<ManagedProcess, SupervisorError> {
    if generation == 0 { return Err(SupervisorError::Invalid("positive launch generation required".into())); }
    launch_with_policy(spec, generation, Some(policy))
}
fn launch(spec: ProcessSpec, generation: u64) -> Result<ManagedProcess, SupervisorError> { launch_with_policy(spec, generation, None) }
fn launch_with_policy(spec: ProcessSpec, generation: u64, policy: Option<ContainmentPolicy>) -> Result<ManagedProcess, SupervisorError> {
    if spec.module_id.is_empty() || !spec.command.is_absolute() || !spec.cwd.is_absolute() || spec.readiness_timeout.is_zero() || spec.stderr_bytes > 16 * 1024 * 1024 || spec.restart.initial_backoff > spec.restart.max_backoff || spec.env.keys().any(|k| k.starts_with("HYPERMIND_") || k.is_empty() || k.contains(['=', '\0'])) { return Err(SupervisorError::Invalid("invalid command, environment or bounds".into())); }
    let sequence = LAUNCH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let launch_id = format!("{}-{}-{sequence}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos());
    if policy.is_some() && spec.env.keys().any(|key| key.starts_with("LD_") || key.starts_with("MALLOC_") || ["GLIBC_TUNABLES", "GCONV_PATH", "LOCPATH", "ENV", "BASH_ENV", "SHELLOPTS", "IFS"].contains(&key.as_str())) { return Err(SupervisorError::Invalid("privileged launcher environment refused".into())); }
    let mut command = if let Some(policy) = &policy {
        #[cfg(target_os = "linux")]
        { crate::containment::prepare_command(policy, &spec.command, &spec.args, &spec.cwd)? }
        #[cfg(not(target_os = "linux"))]
        { return Err(ContainmentError::Unsupported("Linux containment is not available on this platform".into()).into()); }
    } else { let mut command = Command::new(&spec.command); command.args(&spec.args); command };
    command.env_clear().envs(&spec.env).env("HYPERMIND_MODULE_ID", &spec.module_id).env("HYPERMIND_LAUNCH_ID", &launch_id).env("HYPERMIND_GENERATION", generation.to_string()).current_dir(if policy.is_some() { std::path::Path::new("/") } else { &spec.cwd }).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    #[cfg(target_os = "linux")]
    let tree = if policy.is_some() { match OwnedProcessTree::attach(child.id()) { Ok(tree) => Some(tree), Err(error) => { let _ = child.kill(); let _ = child.wait(); return Err(error.into()); } } } else { None };
    let stderr = Arc::new(Mutex::new(VecDeque::new())); let captured = stderr.clone(); let capacity = spec.stderr_bytes;
    let stderr_reader = child.stderr.take().map(|mut pipe| thread::spawn(move || { let mut buffer = [0u8; 4096]; while let Ok(count) = pipe.read(&mut buffer) { if count == 0 { break; } let mut tail = captured.lock().unwrap_or_else(|e| e.into_inner()); for byte in &buffer[..count] { if capacity > 0 { if tail.len() == capacity { tail.pop_front(); } tail.push_back(*byte); } } } }));
    let identity = LaunchIdentity { module_id: spec.module_id.clone(), launch_id, generation, pid: child.id() };
    let mut process = ManagedProcess { identity, state: ProcessState::Starting, child, stderr, spec, policy: policy.clone(), #[cfg(target_os = "linux")] tree, stderr_reader };
    let deadline = Instant::now() + process.spec.readiness_timeout;
    loop { if process.child.try_wait()?.is_some() { if policy.is_some() { let _ = process.finish_pipe(Duration::from_millis(100)); return Err(SupervisorError::ContainedLaunch(String::from_utf8_lossy(&process.stderr_tail()).into_owned())); } return Err(SupervisorError::EarlyExit); } let confined = if let Some(policy) = &policy {
        #[cfg(target_os = "linux")]
        { process.tree.as_mut().unwrap().confirm_restricted(policy)? }
        #[cfg(not(target_os = "linux"))]
        { false }
    } else { true };
    if confined && probe(&process.spec.readiness) { process.state = ProcessState::Ready; return Ok(process); } if Instant::now() >= deadline { return Err(SupervisorError::ReadinessTimeout); } thread::sleep(Duration::from_millis(5)); }
}
