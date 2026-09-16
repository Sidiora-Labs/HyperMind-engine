#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::bench::execution::repository_root;
use crate::bench::gateway::DynError;
use crate::contract::CONTRACT_ACTOR;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const READY_DEADLINE: Duration = Duration::from_secs(15);
pub const PROJECTION_MAP_BYTES: usize = 67_108_864;

const POLL_INTERVAL: Duration = Duration::from_millis(20);

pub struct ContractDaemon {
    child: Child,
    root: PathBuf,
    socket: PathBuf,
    config: PathBuf,
    executable: PathBuf,
}

pub fn binary() -> Result<PathBuf, DynError> {
    if let Some(value) = std::env::var_os("HM_BINARY") {
        let path = PathBuf::from(value);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "HM_BINARY names {}, which is not a file; build it with `cargo build -p hm-cli`",
            path.display()
        )
        .into());
    }
    let mut candidates = Vec::new();
    if let Ok(current) = std::env::current_exe() {
        for directory in current.ancestors().skip(1).take(2) {
            candidates.push(directory.join("hm"));
        }
    }
    if let Some(target) = std::env::var_os("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(target).join("debug/hm"));
    }
    candidates.push(repository_root().join("target/debug/hm"));
    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "the hm binary is missing at {}; build it with `cargo build -p hm-cli`",
        candidates
            .last()
            .map(|path| path.display().to_string())
            .unwrap_or_default()
    )
    .into())
}

impl ContractDaemon {
    pub fn start(root: &Path) -> Result<Self, DynError> {
        let socket = root.join("hypermind.sock");
        let data = root.join("data");
        let config = root.join("hypermind.conf");
        fs::create_dir_all(&data)?;
        write_configuration(&config, &socket, &data)?;
        let executable = binary()?;
        let mut child = spawn(&executable, &config)?;
        if let Err(error) = wait_until_ready(&socket, &mut child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(Self {
            child,
            root: root.to_path_buf(),
            socket,
            config,
            executable,
        })
    }

    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn config(&self) -> &Path {
        &self.config
    }

    #[must_use]
    pub fn token_hex() -> String {
        "44".repeat(32)
    }

    pub fn restart(&mut self) -> Result<(), DynError> {
        self.child.kill()?;
        let status = self.child.wait()?;
        if status.success() {
            return Err("the daemon exited successfully instead of being killed".into());
        }
        self.child = spawn(&self.executable, &self.config)?;
        wait_until_ready(&self.socket, &mut self.child)
    }

    pub fn stop(&mut self) -> Result<(), DynError> {
        self.child.kill()?;
        self.child.wait()?;
        Ok(())
    }
}

impl Drop for ContractDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn write_configuration(config: &Path, socket: &Path, data: &Path) -> Result<(), DynError> {
    let contents = format!(
        "socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor={CONTRACT_ACTOR}:{}\nprojection_map_bytes={PROJECTION_MAP_BYTES}\n",
        socket.display(),
        data.display(),
        "11".repeat(16),
        "22".repeat(32),
        "33".repeat(32),
        ContractDaemon::token_hex(),
    );
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(config)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn spawn(executable: &Path, config: &Path) -> Result<Child, DynError> {
    let mut command = Command::new(executable);
    command
        .arg("serve")
        .arg("--config")
        .arg(config)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for key in runtime_variables() {
        command.env_remove(key);
    }
    Ok(command.spawn()?)
}

pub(crate) fn runtime_variables() -> Vec<OsString> {
    std::env::vars_os()
        .map(|(key, _)| key)
        .filter(|key| key.to_string_lossy().starts_with("HM_"))
        .collect()
}

fn wait_until_ready(socket: &Path, child: &mut Child) -> Result<(), DynError> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(format!(
                "the daemon exited with {status} before {} accepted a connection",
                socket.display()
            )
            .into());
        }
        if UnixStream::connect(socket).is_ok() {
            return Ok(());
        }
        if started.elapsed() > READY_DEADLINE {
            return Err(format!(
                "{} did not accept a connection within {} seconds",
                socket.display(),
                READY_DEADLINE.as_secs()
            )
            .into());
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}
