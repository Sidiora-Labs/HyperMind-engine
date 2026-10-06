use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkPolicy { DenyIp }
#[derive(Clone, Debug)]
pub struct WritableBind { pub host_path: PathBuf, pub guest_path: PathBuf }
#[derive(Clone, Debug)]
pub struct ResourceLimits { pub address_space_bytes: u64, pub cpu_seconds: u64, pub open_files: u64, pub file_size_bytes: u64, pub processes: u64 }
#[derive(Clone, Debug)]
pub struct ContainmentPolicy { pub rootfs: PathBuf, pub writable_binds: Vec<WritableBind>, pub uid: u32, pub gid: u32, pub limits: ResourceLimits, pub network: NetworkPolicy }
#[derive(Clone, Debug)]
pub struct LinuxCapabilities { pub namespaces: bool, pub stable_pid_handles: bool, pub inherited_resource_limits: bool, pub readonly_rootfs: bool, pub denied_ip_network: bool, pub detail: String }
#[derive(Debug, thiserror::Error)]
pub enum ContainmentError {
    #[error("containment unavailable: {0}")] Unsupported(String),
    #[error("invalid containment policy: {0}")] Invalid(String),
    #[error("containment I/O: {0}")] Io(#[from] std::io::Error),
}
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux::{OwnedProcessTree, ProcessIdentity, capabilities, prepare_command};
