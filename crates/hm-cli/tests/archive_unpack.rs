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

type Hostile = (&'static str, fn(&mut Value));

fn entries(directory: &Path) -> Vec<String> {
    let mut names = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn hostile(archive: &Path, destination: &Path, edit: impl Fn(&mut Value)) {
    let text = fs::read_to_string(archive).unwrap();
    let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    let mut manifest: Value = serde_json::from_str(&lines[0]).unwrap();
    edit(&mut manifest);
    lines[0] = serde_json::to_string(&manifest).unwrap();
    fs::write(destination, format!("{}\n", lines.join("\n"))).unwrap();
}

fn events_index(manifest: &Value) -> usize {
    manifest["members"]
        .as_array()
        .unwrap()
        .iter()
        .position(|member| member["name"] == "events.jsonl")
        .unwrap()
}

#[test]
fn unpack_refuses_traversal_and_round_trips_into_a_fresh_actor() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let source = root.join("source");
    fs::create_dir(&source).unwrap();
    hm(&[
        "init",
        "--path",
        source.to_str().unwrap(),
        "--actor",
        "7",
        "--json",
    ]);
    let config = source.join("hypermind.conf");
    hm(&[
        "remember",
        "--config",
        config.to_str().unwrap(),
        "--conversation",
        "archive-unpack",
        "--content",
        "archive heliotrope fact",
        "--embedded",
        "--json",
    ]);
    let archive = root.join("out.hma");
    let packed = hm(&[
        "archive",
        "pack",
        "--config",
        config.to_str().unwrap(),
        "--output",
        archive.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(packed["events"], 1);

    let destination = root.join("dest");
    fs::create_dir(&destination).unwrap();
    let unpacked = hm(&[
        "archive",
        "unpack",
        "--input",
        archive.to_str().unwrap(),
        "--output",
        destination.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(unpacked["ok"], true);
    assert_eq!(unpacked["verified"], true);
    assert_eq!(unpacked["format"], "hypermind.archive.v1");
    assert_eq!(unpacked["actor"], 7);
    let members = unpacked["members"].as_array().unwrap().clone();
    let mut names = members
        .iter()
        .map(|member| member["name"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    names.sort();
    assert!(names.contains(&"events.jsonl".to_string()));
    assert!(
        names
            .iter()
            .all(|name| ["checkpoint.bin", "events.jsonl"].contains(&name.as_str())),
        "unexpected member names: {names:?}"
    );
    for member in &members {
        let path = Path::new(member["path"].as_str().unwrap());
        assert_eq!(path.parent().unwrap(), destination);
        let metadata = fs::metadata(path).unwrap();
        assert_eq!(metadata.len(), member["bytes"].as_u64().unwrap());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }
    assert_eq!(entries(&destination), names);

    let stream = destination.join("events.jsonl");
    let digest = blake3::hash(&fs::read(&stream).unwrap());
    let again = call(&[
        "archive",
        "unpack",
        "--input",
        archive.to_str().unwrap(),
        "--output",
        destination.to_str().unwrap(),
        "--json",
    ]);
    assert!(
        !again.status.success(),
        "a second unpack overwrote an existing member: {}",
        String::from_utf8_lossy(&again.stdout)
    );
    assert_eq!(blake3::hash(&fs::read(&stream).unwrap()), digest);
    assert_eq!(entries(&destination), names);

    let hostiles: [Hostile; 3] = [
        ("traversal.hma", |manifest| {
            let index = events_index(manifest);
            manifest["members"][index]["name"] = Value::from("../escape.jsonl");
        }),
        ("duplicate.hma", |manifest| {
            let index = events_index(manifest);
            let member = manifest["members"][index].clone();
            manifest["members"] = Value::from(vec![member.clone(), member]);
        }),
        ("oversized.hma", |manifest| {
            let index = events_index(manifest);
            manifest["members"][index]["bytes"] = Value::from(999_999_999_999_u64);
        }),
    ];
    let rejected = root.join("dest2");
    fs::create_dir(&rejected).unwrap();
    for (name, edit) in hostiles {
        let path = root.join(name);
        hostile(&archive, &path, edit);
        let output = call(&[
            "archive",
            "unpack",
            "--input",
            path.to_str().unwrap(),
            "--output",
            rejected.to_str().unwrap(),
            "--json",
        ]);
        assert!(
            !output.status.success(),
            "{name} was accepted: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(entries(&rejected).is_empty(), "{name} wrote into dest2");
        assert!(!root.join("escape.jsonl").exists(), "{name} escaped dest2");
    }

    let fresh = root.join("fresh");
    fs::create_dir(&fresh).unwrap();
    hm(&[
        "init",
        "--path",
        fresh.to_str().unwrap(),
        "--actor",
        "7",
        "--json",
    ]);
    let fresh_config = fresh.join("hypermind.conf");
    let imported = hm(&[
        "import",
        "--config",
        fresh_config.to_str().unwrap(),
        "--input",
        archive.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(imported["ok"], true);
    assert_eq!(imported["verified"], true);
    assert_eq!(imported["format"], "hypermind.events.v1");
    assert_eq!(imported["events"], 1);

    let recalled = hm(&[
        "recall",
        "--config",
        fresh_config.to_str().unwrap(),
        "--query",
        "heliotrope",
        "--embedded",
        "--json",
    ]);
    assert_eq!(recalled["items"].as_array().unwrap().len(), 1);
}
