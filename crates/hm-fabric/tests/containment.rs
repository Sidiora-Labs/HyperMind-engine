#![cfg(target_os = "linux")]
use hm_fabric::{containment::*, supervisor::*};
use std::{collections::BTreeMap, fs, net::TcpListener, os::unix::fs::PermissionsExt, path::{Path, PathBuf}, process::Command, thread, time::{Duration, Instant}};
fn image(root: &Path, executable: &str) {
    let canonical = fs::canonicalize(executable).unwrap();
    for target in [PathBuf::from(executable), canonical] {
        let destination = root.join(target.strip_prefix("/").unwrap()); fs::create_dir_all(destination.parent().unwrap()).unwrap(); fs::copy(executable, destination).unwrap();
    }
    let dependencies = Command::new("/usr/bin/ldd").arg(executable).output().unwrap();
    for word in String::from_utf8_lossy(&dependencies.stdout).split_whitespace().filter(|word| word.starts_with('/')) {
        let source = Path::new(word); let destination = root.join(source.strip_prefix("/").unwrap());
        fs::create_dir_all(destination.parent().unwrap()).unwrap(); fs::copy(source, destination).unwrap();
    }
}
fn declaration(script: &str) -> ProcessSpec {
    ProcessSpec { module_id: "contained".into(), command: "/bin/sh".into(), args: vec!["-c".into(), script.into()], env: BTreeMap::new(), cwd: "/".into(), readiness: Probe::ProcessAlive, health: Probe::ProcessAlive, readiness_timeout: Duration::from_secs(5), shutdown_timeout: Duration::from_millis(100), stderr_bytes: 4096, drain_message: None, restart: RestartPolicy { max_restarts: 1, initial_backoff: Duration::from_millis(10), max_backoff: Duration::from_millis(20) } }
}
fn policy(root: &Path, work: &Path) -> ContainmentPolicy {
    ContainmentPolicy { rootfs: root.into(), writable_binds: vec![WritableBind { host_path: work.into(), guest_path: "/work".into() }], uid: 65534, gid: 65534, limits: ResourceLimits { address_space_bytes: 256 * 1024 * 1024, cpu_seconds: 3, open_files: 32, file_size_bytes: 1024 * 1024, processes: 64 }, network: NetworkPolicy::DenyIp }
}
fn await_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() && Instant::now() < deadline { thread::sleep(Duration::from_millis(10)); }
    assert!(path.exists(), "contained program did not produce {}", path.display());
}
#[test]
fn actual_linux_tree_resources_filesystem_network_and_stable_handle() {
    let capabilities = capabilities();
    assert!(capabilities.namespaces && capabilities.stable_pid_handles && capabilities.readonly_rootfs && capabilities.denied_ip_network, "Linux security gate unsupported: {capabilities:?}");
    let temp = tempfile::tempdir().unwrap(); fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o755)).unwrap();
    let root = temp.path().join("image"); let work = temp.path().join("work"); fs::create_dir_all(root.join("work")).unwrap(); fs::create_dir_all(root.join("dev")).unwrap(); fs::create_dir_all(&work).unwrap(); fs::set_permissions(&work, fs::Permissions::from_mode(0o777)).unwrap();
    for executable in ["/bin/sh", "/bin/sleep", "/bin/cat", "/usr/bin/curl", "/usr/bin/dd", "/usr/bin/stat", "/bin/ls"] { image(&root, executable); }
    fs::write(root.join("readonly"), b"original").unwrap(); fs::write(temp.path().join("secret"), b"outside").unwrap();
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let script = format!("/bin/ls -1 /dev > /work/devices; /usr/bin/stat -c '%F:%t:%T' /dev/null /dev/zero /dev/random /dev/urandom > /work/device-types; /usr/bin/dd if=/dev/zero of=/work/zero bs=16 count=1 status=none; /usr/bin/dd if=/dev/random of=/work/random bs=16 count=1 status=none; /usr/bin/dd if=/dev/urandom of=/work/urandom bs=16 count=1 status=none; if echo forbidden > /dev/extra; then echo escape > /work/dev-write; else echo denied > /work/dev-write; fi; if cat '{}' >/dev/null 2>&1; then echo escape > /work/fs; else echo denied > /work/fs; fi; if echo changed > /readonly; then echo escape > /work/ro; else echo denied > /work/ro; fi; if /usr/bin/curl --max-time 1 http://127.0.0.1:{} >/dev/null 2>&1; then echo escape > /work/net; else echo denied > /work/net; fi; ulimit -v > /work/as; ulimit -n > /work/fd; ulimit -f > /work/size; sleep 30 & echo child > /work/ready; wait", temp.path().join("secret").display(), server.local_addr().unwrap().port());
    let mut process = launch_candidate(declaration(&script), 1, policy(&root, &work)).unwrap();
    await_file(&work.join("ready"));
    assert_eq!(fs::read_to_string(work.join("fs")).unwrap().trim(), "denied"); assert_eq!(fs::read_to_string(work.join("ro")).unwrap().trim(), "denied"); assert_eq!(fs::read_to_string(work.join("net")).unwrap().trim(), "denied");
    assert_eq!(fs::read_to_string(work.join("as")).unwrap().trim(), "262144"); assert_eq!(fs::read_to_string(work.join("fd")).unwrap().trim(), "32");
    assert_eq!(fs::read(root.join("readonly")).unwrap(), b"original");
    assert_eq!(fs::read_to_string(work.join("devices")).unwrap(), "null\nrandom\nurandom\nzero\n");
    assert_eq!(fs::read_to_string(work.join("device-types")).unwrap(), "character special file:1:3\ncharacter special file:1:5\ncharacter special file:1:8\ncharacter special file:1:9\n");
    assert_eq!(fs::read(work.join("zero")).unwrap(), vec![0; 16]);
    assert_eq!(fs::read(work.join("random")).unwrap().len(), 16);
    assert_eq!(fs::read(work.join("urandom")).unwrap().len(), 16);
    assert_eq!(fs::read_to_string(work.join("dev-write")).unwrap().trim(), "denied");
    assert_eq!(fs::read_dir(root.join("dev")).unwrap().count(), 0);
    let mut descendant_pids = Vec::new();
    fn descendants(pid: u32, output: &mut Vec<u32>) { if let Ok(text) = fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")) { for child in text.split_whitespace().filter_map(|p| p.parse().ok()) { output.push(child); descendants(child, output); } } }
    descendants(process.identity.pid, &mut descendant_pids); assert!(descendant_pids.len() >= 2);
    let stable = OwnedProcessTree::attach(process.identity.pid).unwrap(); let before = Instant::now(); process.shutdown().unwrap(); assert!(before.elapsed() < Duration::from_secs(4));
    for pid in descendant_pids { assert!(!Path::new(&format!("/proc/{pid}")).exists(), "owned descendant {pid} remains"); }
    let mut unrelated = Command::new("/bin/sleep").arg("5").spawn().unwrap(); stable.terminate().unwrap(); assert!(unrelated.try_wait().unwrap().is_none()); unrelated.kill().unwrap(); unrelated.wait().unwrap();
    let mut invalid = policy(&root, &work); invalid.uid = 0; assert!(launch_candidate(declaration("sleep 5"), 2, invalid).is_err());
    let mut poison = declaration("sleep 5"); poison.env.insert("LD_PRELOAD".into(), "/any".into()); assert!(launch_candidate(poison, 3, policy(&root, &work)).is_err());
}

#[test]
fn device_mountpoint_preflight_refuses_missing_symlink_and_writable_overrides() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("image"); let work = temporary.path().join("work");
    fs::create_dir_all(root.join("work")).unwrap(); fs::create_dir_all(&work).unwrap();
    let absent = policy(&root, &work);
    assert!(matches!(absent.validate(), Err(ContainmentError::Invalid(_))));
    assert!(!root.join("dev").exists());
    std::os::unix::fs::symlink("work", root.join("dev")).unwrap();
    assert!(matches!(absent.validate(), Err(ContainmentError::Invalid(_))));
    fs::remove_file(root.join("dev")).unwrap(); fs::create_dir(root.join("dev")).unwrap();
    absent.validate().unwrap();
    let mut overlap = absent.clone(); overlap.writable_binds.push(WritableBind { host_path: work.clone(), guest_path: "/dev".into() });
    assert!(matches!(overlap.validate(), Err(ContainmentError::Invalid(_))));
    fs::create_dir(root.join("dev/subdirectory")).unwrap();
    overlap.writable_binds[1].guest_path = "/dev/subdirectory".into();
    assert!(matches!(overlap.validate(), Err(ContainmentError::Invalid(_))));
}
