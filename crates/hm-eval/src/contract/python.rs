#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::bench::execution::repository_root;
use crate::bench::gateway::DynError;
use crate::contract::daemon::{self, ContractDaemon, PROJECTION_MAP_BYTES, READY_DEADLINE, binary};
use crate::contract::{
    CONTRACT_ACTOR, ContractObservation, Phase, RawDocument, Unavailable, observe_raw,
    write_scenario,
};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const SDK: &str = "python";
pub const TRANSPORT: &str = "grpc";
pub const PACKAGE_DIRECTORY: &str = "sdk/python";
pub const LEG_SCRIPT: &str = "eval/contract/python-leg.py";
pub const IMPORT_PROBE: &str = "import hypermind";

const SCENARIO_FILE: &str = "scenario.v1.json";
const POLL_INTERVAL: Duration = Duration::from_millis(30);
const CERTIFICATE_SUBJECT: &str = "/CN=hypermind-cross-sdk-contract";

fn unavailable(reason: String) -> Unavailable {
    Unavailable {
        sdk: SDK.to_owned(),
        reason,
    }
}

pub fn availability() -> Result<(), Unavailable> {
    availability_in(&repository_root())
}

pub fn availability_in(root: &Path) -> Result<(), Unavailable> {
    let script = root.join(LEG_SCRIPT);
    if !script.is_file() {
        return Err(unavailable(format!(
            "the leg script is missing at {}",
            script.display()
        )));
    }
    let probe = Command::new("python3")
        .arg("-c")
        .arg(IMPORT_PROBE)
        .env("PYTHONPATH", root.join(PACKAGE_DIRECTORY))
        .stdin(Stdio::null())
        .output()
        .map_err(|error| unavailable(format!("python3 is not on PATH: {error}")))?;
    if !probe.status.success() {
        return Err(unavailable(format!(
            "`python3 -c \"{IMPORT_PROBE}\"` with PYTHONPATH={} exited with {}: {}",
            root.join(PACKAGE_DIRECTORY).display(),
            probe.status,
            String::from_utf8_lossy(&probe.stderr).trim()
        )));
    }
    match Command::new("openssl")
        .arg("version")
        .stdin(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            return Err(unavailable(format!(
                "`openssl version` exited with {}, so the leg has no mutual tls material",
                output.status
            )));
        }
        Err(error) => {
            return Err(unavailable(format!(
                "openssl is not on PATH, so the leg has no mutual tls material: {error}"
            )));
        }
    }
    binary().map_err(|error| unavailable(error.to_string()))?;
    Ok(())
}

pub fn tls_material(root: &Path) -> Result<(), DynError> {
    fs::create_dir_all(root)?;
    openssl(
        root,
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "ca.key",
            "-out",
            "ca.pem",
            "-subj",
            CERTIFICATE_SUBJECT,
            "-days",
            "1",
        ],
    )?;
    for (name, purpose) in [("server", "serverAuth"), ("client", "clientAuth")] {
        openssl(
            root,
            &[
                "req",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-keyout",
                &format!("{name}.key"),
                "-out",
                &format!("{name}.csr"),
                "-subj",
                &format!("/CN={name}"),
            ],
        )?;
        fs::write(
            root.join(format!("{name}.ext")),
            format!(
                "basicConstraints=CA:FALSE\nextendedKeyUsage={purpose}\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n"
            ),
        )?;
        openssl(
            root,
            &[
                "x509",
                "-req",
                "-in",
                &format!("{name}.csr"),
                "-CA",
                "ca.pem",
                "-CAkey",
                "ca.key",
                "-CAcreateserial",
                "-out",
                &format!("{name}.pem"),
                "-days",
                "1",
                "-extfile",
                &format!("{name}.ext"),
            ],
        )?;
    }
    Ok(())
}

fn openssl(root: &Path, arguments: &[&str]) -> Result<(), DynError> {
    let output = Command::new("openssl")
        .args(arguments)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "`openssl {}` exited with {}: {}",
            arguments.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(())
}

pub struct GrpcDaemon {
    child: Child,
    root: PathBuf,
    address: SocketAddr,
    config: PathBuf,
    executable: PathBuf,
}

impl GrpcDaemon {
    pub fn start(root: &Path) -> Result<Self, DynError> {
        let data = root.join("data");
        let config = root.join("hypermind.conf");
        fs::create_dir_all(&data)?;
        tls_material(root)?;
        write_configuration(&config, &root.join("hypermind.sock"), &data)?;
        let address = free_address()?;
        let executable = binary()?;
        let mut child = spawn(&executable, &config, root, address)?;
        if let Err(error) = wait_until_ready(address, &mut child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(Self {
            child,
            root: root.to_path_buf(),
            address,
            config,
            executable,
        })
    }

    #[must_use]
    pub fn endpoint(&self) -> String {
        format!("localhost:{}", self.address.port())
    }

    #[must_use]
    pub fn tls_root(&self) -> &Path {
        &self.root
    }

    pub fn restart(&mut self) -> Result<(), DynError> {
        self.child.kill()?;
        let status = self.child.wait()?;
        if status.success() {
            return Err("the grpc daemon exited successfully instead of being killed".into());
        }
        self.child = spawn(&self.executable, &self.config, &self.root, self.address)?;
        wait_until_ready(self.address, &mut self.child)
    }

    pub fn stop(&mut self) -> Result<(), DynError> {
        self.child.kill()?;
        self.child.wait()?;
        Ok(())
    }
}

impl Drop for GrpcDaemon {
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

fn free_address() -> Result<SocketAddr, DynError> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?)
}

fn spawn(
    executable: &Path,
    config: &Path,
    tls_root: &Path,
    address: SocketAddr,
) -> Result<Child, DynError> {
    let mut command = Command::new(executable);
    command
        .arg("serve")
        .arg("--config")
        .arg(config)
        .arg("--grpc-bind")
        .arg(address.to_string())
        .arg("--tls-cert")
        .arg(tls_root.join("server.pem"))
        .arg("--tls-key")
        .arg(tls_root.join("server.key"))
        .arg("--tls-client-ca")
        .arg(tls_root.join("ca.pem"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for key in daemon::runtime_variables() {
        command.env_remove(key);
    }
    Ok(command.spawn()?)
}

fn wait_until_ready(address: SocketAddr, child: &mut Child) -> Result<(), DynError> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(
                format!("the grpc daemon exited with {status} before {address} accepted").into(),
            );
        }
        if TcpStream::connect_timeout(&address, POLL_INTERVAL).is_ok() {
            return Ok(());
        }
        if started.elapsed() > READY_DEADLINE {
            return Err(format!(
                "{address} did not accept a connection within {} seconds",
                READY_DEADLINE.as_secs()
            )
            .into());
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

pub fn run_leg(
    endpoint: &str,
    tls_root: &Path,
    conversation: &str,
    phase: Phase,
    out: &Path,
) -> Result<Vec<u8>, DynError> {
    let root = repository_root();
    let directory = out.parent().ok_or("the leg output path has no parent")?;
    let scenario = directory.join(SCENARIO_FILE);
    write_scenario(&scenario, conversation)?;
    let phase_name = serde_json::to_value(phase)?
        .as_str()
        .ok_or("the scenario phase is not a string")?
        .to_owned();
    let mut command = Command::new("python3");
    command
        .arg(root.join(LEG_SCRIPT))
        .arg("--endpoint")
        .arg(endpoint)
        .arg("--tls-root")
        .arg(tls_root)
        .arg("--token")
        .arg(ContractDaemon::token_hex())
        .arg("--scenario")
        .arg(&scenario)
        .arg("--phase")
        .arg(&phase_name)
        .arg("--out")
        .arg(out)
        .current_dir(&root)
        .stdin(Stdio::null());
    for key in daemon::runtime_variables() {
        command.env_remove(key);
    }
    command.env("PYTHONPATH", root.join(PACKAGE_DIRECTORY));
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "the python leg exited with {} during {phase_name}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(std::fs::read(out)?)
}

pub fn observation(root: &Path, conversation: &str) -> Result<ContractObservation, DynError> {
    let mut daemon = GrpcDaemon::start(root)?;
    let endpoint = daemon.endpoint();
    let observed = collect(&mut daemon, &endpoint, root, conversation);
    let stopped = daemon.stop();
    let observed = observed?;
    stopped?;
    Ok(observed)
}

fn collect(
    daemon: &mut GrpcDaemon,
    endpoint: &str,
    root: &Path,
    conversation: &str,
) -> Result<ContractObservation, DynError> {
    let before = run_leg(
        endpoint,
        root,
        conversation,
        Phase::BeforeRestart,
        &root.join("python-before-restart.json"),
    )?;
    daemon.restart()?;
    let after = run_leg(
        endpoint,
        root,
        conversation,
        Phase::AfterRestart,
        &root.join("python-after-restart.json"),
    )?;
    let mut document: RawDocument = serde_json::from_slice(&before)?;
    let tail: RawDocument = serde_json::from_slice(&after)?;
    if document.format != tail.format
        || document.sdk != tail.sdk
        || document.transport != tail.transport
    {
        return Err("the python leg changed its identity across the restart".into());
    }
    if document.sdk != SDK || document.transport != TRANSPORT {
        return Err(format!(
            "the python leg reported sdk {} over {}",
            document.sdk, document.transport
        )
        .into());
    }
    document.calls.extend(tail.calls);
    observe_raw(&serde_json::to_vec(&document)?)
}
