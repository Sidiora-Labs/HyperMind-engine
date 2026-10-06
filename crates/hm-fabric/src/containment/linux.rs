use super::*;
use rustix::process::{Pid, PidfdFlags, Signal, pidfd_open, pidfd_send_signal, set_child_subreaper, waitpid, WaitOptions};
use std::{fs, os::fd::OwnedFd, process::Command, thread, time::{Duration, Instant}};

const SCRIPT: &str = r#"set -eu
root=$1; uid=$2; gid=$3; address=$4; cpu=$5; files=$6; size=$7; processes=$8; count=$9
shift 9
/usr/bin/mount --bind "$root" "$root"
/usr/bin/mount -o remount,bind,ro,nosuid,nodev "$root"
/usr/bin/mount -t tmpfs -o rw,size=65536,nr_inodes=8,mode=755,nosuid,noexec,dev tmpfs "$root/dev"
/usr/bin/mknod -m 666 "$root/dev/null" c 1 3
/usr/bin/mknod -m 666 "$root/dev/zero" c 1 5
/usr/bin/mknod -m 666 "$root/dev/random" c 1 8
/usr/bin/mknod -m 666 "$root/dev/urandom" c 1 9
/usr/bin/mount -o remount,ro,nosuid,noexec,dev "$root/dev"
while [ "$count" -gt 0 ]; do
    /usr/bin/mount --bind "$1" "$root$2"
    /usr/bin/mount -o remount,bind,rw,nosuid,nodev,noexec "$root$2"
    shift 2; count=$((count - 1))
done
cwd=$1; shift
exec /usr/bin/prlimit --as="$address" --cpu="$cpu" --nofile="$files" --fsize="$size" --nproc="$processes" /usr/bin/unshare --root "$root" --wd "$cwd" --setgid "$gid" --setuid "$uid" -- "$@"
"#;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessIdentity { pub pid: u32, pub start_ticks: u64 }
pub struct OwnedProcessTree { pub identity: ProcessIdentity, pidfd: OwnedFd, init: Option<ProcessIdentity>, init_pidfd: Option<OwnedFd> }
fn errno(error: rustix::io::Errno) -> std::io::Error { std::io::Error::from_raw_os_error(error.raw_os_error()) }
fn identity(pid: u32) -> Result<ProcessIdentity, ContainmentError> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let rest = stat.rsplit_once(") ").ok_or_else(|| ContainmentError::Invalid("invalid proc identity".into()))?.1;
    let ticks = rest.split_whitespace().nth(19).and_then(|value| value.parse().ok()).ok_or_else(|| ContainmentError::Invalid("invalid proc start time".into()))?;
    Ok(ProcessIdentity { pid, start_ticks: ticks })
}
pub fn capabilities() -> LinuxCapabilities {
    let pid = Pid::from_raw(std::process::id() as i32).unwrap();
    let stable = pidfd_open(pid, PidfdFlags::empty()).is_ok();
    let outcome = Command::new("/usr/bin/unshare").args(["--mount", "--net", "--pid", "--fork", "--kill-child=KILL", "/bin/true"]).env_clear().output();
    let (namespaces, detail) = match outcome { Ok(output) => (output.status.success(), String::from_utf8_lossy(&output.stderr).into_owned()), Err(error) => (false, error.to_string()) };
    let tools = ["/usr/bin/mount", "/usr/bin/mknod", "/usr/bin/prlimit", "/usr/bin/unshare", "/bin/sh"].iter().all(|path| fs::metadata(path).is_ok());
    LinuxCapabilities { namespaces, stable_pid_handles: stable, inherited_resource_limits: tools, readonly_rootfs: namespaces && tools, denied_ip_network: namespaces, detail }
}
impl ContainmentPolicy {
    pub fn validate(&self) -> Result<(), ContainmentError> {
        if !self.rootfs.is_absolute() || fs::canonicalize(&self.rootfs)? != self.rootfs || !self.rootfs.is_dir() || self.uid == 0 || self.gid == 0 { return Err(ContainmentError::Invalid("canonical rootfs and nonzero UID/GID required".into())); }
        let device_path = self.rootfs.join("dev");
        let device_metadata = match fs::symlink_metadata(&device_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(ContainmentError::Invalid("rootfs requires a private device mountpoint directory".into())),
            Err(error) => return Err(error.into()),
        };
        if !device_metadata.file_type().is_dir() || fs::canonicalize(&device_path)? != device_path {
            return Err(ContainmentError::Invalid("rootfs device mountpoint must be a canonical directory".into()));
        }
        let limits = &self.limits;
        if [limits.address_space_bytes, limits.cpu_seconds, limits.open_files, limits.file_size_bytes, limits.processes].contains(&0) || self.writable_binds.len() > 32 { return Err(ContainmentError::Invalid("positive resource limits and bounded binds required".into())); }
        for binding in &self.writable_binds {
            if binding.guest_path.starts_with(std::path::Path::new("/dev")) {
                return Err(ContainmentError::Invalid("writable bind overlaps private devices".into()));
            }
            if !binding.host_path.is_absolute() || fs::canonicalize(&binding.host_path)? != binding.host_path || !binding.host_path.is_dir() || !binding.guest_path.is_absolute() || binding.guest_path == std::path::Path::new("/") || binding.guest_path.components().any(|part| matches!(part, std::path::Component::ParentDir)) { return Err(ContainmentError::Invalid("canonical directory bind required".into())); }
            let destination = self.rootfs.join(binding.guest_path.strip_prefix("/").unwrap());
            let canonical = fs::canonicalize(destination)?;
            if !canonical.starts_with(&self.rootfs) || !canonical.is_dir() { return Err(ContainmentError::Invalid("bind destination escapes rootfs".into())); }
        }
        Ok(())
    }
}
pub fn prepare_command(policy: &ContainmentPolicy, executable: &std::path::Path, arguments: &[String], cwd: &std::path::Path) -> Result<Command, ContainmentError> {
    policy.validate()?;
    if !executable.is_absolute() || !cwd.is_absolute() || executable.components().chain(cwd.components()).any(|part| matches!(part, std::path::Component::ParentDir)) { return Err(ContainmentError::Invalid("absolute guest command and working directory required".into())); }
    let caps = capabilities();
    if !caps.namespaces || !caps.stable_pid_handles || !caps.inherited_resource_limits || !caps.readonly_rootfs { return Err(ContainmentError::Unsupported(format!("Linux namespace/pidfd/tools unavailable: {}", caps.detail))); }
    set_child_subreaper(Pid::from_raw(1)).map_err(errno)?;
    let mut command = Command::new("/usr/bin/unshare");
    command.args(["--mount", "--net", "--pid", "--fork", "--kill-child=KILL", "--propagation", "private", "/bin/sh", "-c", SCRIPT, "hypermind-containment"]);
    command.arg(&policy.rootfs).arg(policy.uid.to_string()).arg(policy.gid.to_string()).arg(policy.limits.address_space_bytes.to_string()).arg(policy.limits.cpu_seconds.to_string()).arg(policy.limits.open_files.to_string()).arg(policy.limits.file_size_bytes.to_string()).arg(policy.limits.processes.to_string()).arg(policy.writable_binds.len().to_string());
    for binding in &policy.writable_binds { command.arg(&binding.host_path).arg(&binding.guest_path); }
    command.arg(cwd).arg(executable).args(arguments);
    Ok(command)
}
impl OwnedProcessTree {
    pub fn attach(pid: u32) -> Result<Self, ContainmentError> {
        let before = identity(pid)?;
        let pidfd = pidfd_open(Pid::from_raw(pid as i32).ok_or_else(|| ContainmentError::Invalid("invalid process identifier".into()))?, PidfdFlags::empty()).map_err(errno)?;
        if identity(pid)? != before { return Err(ContainmentError::Invalid("process identity changed during acquisition".into())); }
        Ok(Self { identity: before, pidfd, init: None, init_pidfd: None })
    }
    pub fn confirm_restricted(&mut self, policy: &ContainmentPolicy) -> Result<bool, ContainmentError> {
        let children = fs::read_to_string(format!("/proc/{0}/task/{0}/children", self.identity.pid))?;
        for pid in children.split_whitespace().filter_map(|p| p.parse::<u32>().ok()) {
            let before = identity(pid)?;
            let descriptor = pidfd_open(Pid::from_raw(pid as i32).ok_or_else(|| ContainmentError::Invalid("invalid namespace init identifier".into()))?, PidfdFlags::empty()).map_err(errno)?;
            if identity(pid)? != before { return Err(ContainmentError::Invalid("namespace init identity changed during acquisition".into())); }
            self.init = Some(before.clone());
            self.init_pidfd = Some(descriptor);
            let status = fs::read_to_string(format!("/proc/{pid}/status"))?;
            let effective = status.lines().find(|line| line.starts_with("Uid:")).and_then(|line| line.split_whitespace().nth(2)).and_then(|value| value.parse::<u32>().ok());
            if effective == Some(policy.uid) {
                if identity(pid)? != before { return Err(ContainmentError::Invalid("namespace init identity changed before restriction confirmation".into())); }
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn signal(&self, signal: Signal) -> Result<(), ContainmentError> {
        match pidfd_send_signal(&self.pidfd, signal) { Ok(()) => Ok(()), Err(rustix::io::Errno::SRCH) => Ok(()), Err(error) => Err(errno(error).into()) }
    }
    pub fn terminate(&self) -> Result<(), ContainmentError> {
        if let Some(init_pidfd) = &self.init_pidfd {
            match pidfd_send_signal(init_pidfd, Signal::KILL) {
                Ok(()) | Err(rustix::io::Errno::SRCH) => {},
                Err(error) => return Err(errno(error).into()),
            }
        }
        self.signal(Signal::KILL)
    }
    pub fn reap_init(&self, timeout: Duration) -> Result<(), ContainmentError> {
        let Some(init) = &self.init else { return Ok(()); };
        let deadline = Instant::now() + timeout;
        loop {
            if !std::path::Path::new(&format!("/proc/{}", init.pid)).exists() { return Ok(()); }
            if identity(init.pid).is_ok_and(|current| current != *init) { return Ok(()); }
            match waitpid(Pid::from_raw(init.pid as i32), WaitOptions::NOHANG) {
                Ok(Some(_)) => return Ok(()), Ok(None) | Err(rustix::io::Errno::CHILD) => {}, Err(error) => return Err(errno(error).into()),
            }
            if Instant::now() >= deadline { return Err(ContainmentError::Unsupported("owned namespace init did not reap within deadline".into())); }
            thread::sleep(Duration::from_millis(5));
        }
    }
}
