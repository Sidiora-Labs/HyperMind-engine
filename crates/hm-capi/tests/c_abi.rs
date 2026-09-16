use std::path::{Path, PathBuf};
use std::process::Command;

const MARKER: &str = "HM_ABI_SMOKE_OK";
const LIBRARY: &str = "libhypermind.so";

fn library_directory() -> PathBuf {
    let executable = std::env::current_exe().expect("the test binary has a path");
    executable
        .parent()
        .and_then(Path::parent)
        .expect("the test binary lives in target/<profile>/deps")
        .to_path_buf()
}

#[test]
fn c_consumer_drives_the_real_abi() {
    let libraries = library_directory();
    assert!(
        libraries.join(LIBRARY).exists(),
        "{LIBRARY} not found in {}; run `cargo build -p hm-capi` first (cargo test does not build cdylib targets)",
        libraries.display()
    );

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build = tempfile::tempdir().expect("temporary build directory");
    let program = build.path().join("hm_abi_smoke");
    let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".to_owned());

    let compilation = Command::new(&compiler)
        .args(["-std=c11", "-Wall", "-Wextra"])
        .arg("-I")
        .arg(manifest.join("include"))
        .arg("-o")
        .arg(&program)
        .arg(manifest.join("tests").join("c").join("hm_abi_smoke.c"))
        .arg("-L")
        .arg(&libraries)
        .arg("-lhypermind")
        .arg(format!("-Wl,-rpath,{}", libraries.display()))
        .args(["-lpthread", "-ldl", "-lm"])
        .output()
        .unwrap_or_else(|error| panic!("{compiler} could not be started: {error}"));

    let notes = String::from_utf8_lossy(&compilation.stderr).into_owned();
    assert!(
        compilation.status.success(),
        "{compiler} refused the C consumer\nstdout:\n{}\nstderr:\n{notes}",
        String::from_utf8_lossy(&compilation.stdout)
    );
    if !notes.trim().is_empty() {
        println!("{compiler} reported:\n{notes}");
    }

    let state = tempfile::tempdir().expect("temporary state directory");
    let run = Command::new(&program)
        .arg(state.path())
        .env_remove("HM_EMBEDDING_PROVIDER")
        .env_remove("HM_RECONSTRUCTION_PROVIDER")
        .env_remove("HM_CONSOLIDATION_PROVIDER")
        .output()
        .unwrap_or_else(|error| panic!("{} could not be started: {error}", program.display()));

    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&run.stderr).into_owned();
    assert!(
        run.status.success(),
        "the C consumer exited with {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        run.status.code()
    );

    let last = stdout
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or_else(|| panic!("the C consumer printed nothing\nstderr:\n{stderr}"));
    assert_eq!(last, MARKER, "stdout:\n{stdout}\nstderr:\n{stderr}");
}
