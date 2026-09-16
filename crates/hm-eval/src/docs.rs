#![allow(clippy::missing_errors_doc)]

use std::error::Error;
use std::path::Path;
use std::process::Command;

pub fn gate() -> Result<(), Box<dyn Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = Command::new("node")
        .arg("docs/tools/catalog.mjs")
        .arg("--check")
        .current_dir(repository)
        .status()?;
    if !status.success() {
        return Err("documentation coverage or local-link gate failed".into());
    }
    Ok(())
}
