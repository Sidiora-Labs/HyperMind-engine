#![allow(clippy::too_many_lines)]

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn call(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hm"))
        .args(args)
        .env("NO_COLOR", "1")
        .env_remove("HM_CONSOLIDATION_PROVIDER")
        .env_remove("HM_RECONSTRUCTION_PROVIDER")
        .env_remove("HM_EMBEDDING_PROVIDER")
        .output()
        .unwrap()
}

fn hm(args: &[&str]) -> Value {
    let output = call(args);
    assert!(
        output.status.success(),
        "hm {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn replace_byte(text: &str, index: usize, first: u8, second: u8) -> String {
    let mut bytes = text.as_bytes().to_vec();
    bytes[index] = if bytes[index] == first { second } else { first };
    String::from_utf8(bytes).unwrap()
}

fn rejects(archive: &Path, mutate: impl Fn(String) -> String) {
    let mutated = archive.with_extension("tampered");
    let _ = fs::remove_file(&mutated);
    fs::write(&mutated, mutate(fs::read_to_string(archive).unwrap())).unwrap();
    let output = call(&[
        "archive",
        "verify",
        "--input",
        mutated.to_str().unwrap(),
        "--json",
    ]);
    assert!(
        !output.status.success(),
        "tampered archive was accepted: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn packed_archive_verifies_without_keys_and_rejects_tampering() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    hm(&[
        "init",
        "--path",
        root.to_str().unwrap(),
        "--actor",
        "7",
        "--json",
    ]);
    let config = root.join("hypermind.conf");
    for content in ["first archived memory", "second archived memory"] {
        hm(&[
            "remember",
            "--config",
            config.to_str().unwrap(),
            "--conversation",
            "archive-test",
            "--content",
            content,
            "--embedded",
            "--json",
        ]);
    }

    let output = root.join("out.hma");
    let packed = hm(&[
        "archive",
        "pack",
        "--config",
        config.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(packed["ok"], true);
    assert_eq!(packed["format"], "hypermind.archive.v1");
    assert_eq!(packed["archive_version"], 1);
    assert_eq!(packed["actor"], 7);
    assert_eq!(packed["events"], 2);
    let names = packed["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|member| member["name"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(
        names
            .iter()
            .all(|name| ["checkpoint.bin", "events.jsonl"].contains(&name.as_str())),
        "unexpected member names: {names:?}"
    );
    assert!(names.iter().any(|name| name == "events.jsonl"));
    assert!(is_hex(packed["public_key"].as_str().unwrap(), 64));
    assert_eq!(
        fs::metadata(&output).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let text = fs::read_to_string(&output).unwrap();
    let manifest: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(manifest["format"], "hypermind.archive.v1");
    assert!(is_hex(manifest["public_key"].as_str().unwrap(), 64));
    assert!(is_hex(manifest["signature"].as_str().unwrap(), 128));

    let verified = hm(&[
        "archive",
        "verify",
        "--input",
        output.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(verified["ok"], true);
    assert_eq!(verified["verified"], true);
    let displayed = call(&["archive", "verify", "--input", output.to_str().unwrap()]);
    assert!(displayed.status.success());
    assert!(
        String::from_utf8_lossy(&displayed.stdout).starts_with("ARCHIVE VERIFIED"),
        "unexpected display: {}",
        String::from_utf8_lossy(&displayed.stdout)
    );

    rejects(&output, |text| {
        let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
        let last = lines.len() - 1;
        let marker = "\"content_base64\":\"";
        let index = lines[last].find(marker).unwrap() + marker.len() + 4;
        lines[last] = replace_byte(&lines[last], index, b'A', b'B');
        format!("{}\n", lines.join("\n"))
    });
    rejects(&output, |text| {
        let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
        let marker = "\"digest\":\"";
        let index = lines[0].find(marker).unwrap() + marker.len();
        lines[0] = replace_byte(&lines[0], index, b'0', b'1');
        format!("{}\n", lines.join("\n"))
    });
    rejects(&output, |text| {
        let mut manifest: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let bytes = manifest["members"][0]["bytes"].as_u64().unwrap();
        manifest["members"][0]["bytes"] = Value::from(bytes - 1);
        let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
        lines[0] = serde_json::to_string(&manifest).unwrap();
        format!("{}\n", lines.join("\n"))
    });
    rejects(&output, |text| {
        let mut manifest: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let signature = manifest["signature"].as_str().unwrap().to_string();
        let first = if signature.starts_with('0') { '1' } else { '0' };
        manifest["signature"] = Value::from(format!("{first}{}", &signature[1..]));
        let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
        lines[0] = serde_json::to_string(&manifest).unwrap();
        format!("{}\n", lines.join("\n"))
    });

    let pinned = call(&[
        "archive",
        "verify",
        "--input",
        output.to_str().unwrap(),
        "--public-key",
        &"ab".repeat(32),
        "--json",
    ]);
    assert!(
        !pinned.status.success(),
        "a mismatched pinned public key was accepted"
    );
}
