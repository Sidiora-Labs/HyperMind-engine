use hypermind::HM_EXPORTED_SYMBOLS;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const MODULE_MAP: &str = "sdk/swift/Sources/CHyperMind/module.modulemap";
const SWIFT_SOURCES: &str = "sdk/swift/Sources/HyperMind";
const HEADER: &str = "crates/hm-capi/include/hypermind.h";
const DISPATCHER: &str = "crates/hm-mcp/src/dispatcher.rs";
const MANIFEST: &str = "sdk/swift/Package.swift";
const README: &str = "sdk/swift/README.md";
const VERB_COUNT: usize = 14;
const ARM: &str = " => match serde_json::from_slice";
const CALL_SITE: &str = "verb: \"";

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory sits two levels below the repository root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn read_relative(relative: &str) -> String {
    read(&repository().join(relative))
}

fn swift_sources() -> String {
    let directory = repository().join(SWIFT_SOURCES);
    let entries = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", directory.display()));
    let mut files: Vec<PathBuf> = entries
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("{} lists cleanly: {error}", directory.display()))
                .path()
        })
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "swift")
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "{} holds Swift sources",
        directory.display()
    );
    files
        .iter()
        .map(|path| read(path))
        .collect::<Vec<String>>()
        .join("\n")
}

fn identifiers(text: &str, prefix: &str, continues: fn(char) -> bool) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (index, _) in text.match_indices(prefix) {
        let preceded = text[..index]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        if preceded {
            continue;
        }
        let tail: String = text[index + prefix.len()..]
            .chars()
            .take_while(|character| continues(*character))
            .collect();
        found.insert(format!("{prefix}{tail}"));
    }
    found
}

fn header_status_enumerators(header: &str) -> BTreeSet<String> {
    header
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_end_matches(',');
            let (name, _) = trimmed.split_once('=')?;
            let name = name.trim();
            name.starts_with("HM_STATUS_").then(|| name.to_owned())
        })
        .collect()
}

fn dispatcher_verbs(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix('"')?;
            let end = rest.find('"')?;
            let (verb, tail) = rest.split_at(end);
            tail[1..].starts_with(ARM).then(|| verb.to_owned())
        })
        .collect()
}

fn swift_verbs(sources: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (index, _) in sources.match_indices(CALL_SITE) {
        let rest = &sources[index + CALL_SITE.len()..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        found.insert(rest[..end].to_owned());
    }
    found
}

#[test]
fn module_map_points_at_the_published_header() {
    let root = repository();
    let map_path = root.join(MODULE_MAP);
    let map = read(&map_path);

    let quoted = map
        .lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix("header ")?.strip_prefix('"')?;
            let end = rest.find('"')?;
            Some(rest[..end].to_owned())
        })
        .expect("the module map names a header");

    let directory = map_path
        .parent()
        .expect("the module map lives in a directory");
    let resolved = directory
        .join(&quoted)
        .canonicalize()
        .unwrap_or_else(|error| panic!("{quoted} resolves from the module map: {error}"));
    let published = root
        .join(HEADER)
        .canonicalize()
        .unwrap_or_else(|error| panic!("{HEADER} resolves: {error}"));
    assert_eq!(resolved, published);
    assert!(map.contains("link \"hypermind\""), "{map}");
}

#[test]
fn swift_uses_only_declared_symbols() {
    let sources = swift_sources();
    let used = identifiers(&sources, "hm_", |character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    });
    assert!(!used.is_empty(), "the Swift sources call the C boundary");

    let declared: BTreeSet<String> = HM_EXPORTED_SYMBOLS
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    let undeclared: BTreeSet<&String> = used.difference(&declared).collect();
    assert!(undeclared.is_empty(), "{undeclared:?}");
}

#[test]
fn swift_uses_only_declared_status_values() {
    let sources = swift_sources();
    let used = identifiers(&sources, "HM_STATUS_", |character| {
        character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
    });
    assert!(
        !used.is_empty(),
        "the Swift sources inspect the status tier"
    );

    let declared = header_status_enumerators(&read_relative(HEADER));
    let undeclared: BTreeSet<&String> = used.difference(&declared).collect();
    assert!(undeclared.is_empty(), "{undeclared:?}");
}

#[test]
fn swift_mirrors_the_verb_inventory() {
    let dispatched = dispatcher_verbs(&read_relative(DISPATCHER));
    assert_eq!(dispatched.len(), VERB_COUNT, "{dispatched:?}");

    let wrapped = swift_verbs(&swift_sources());
    assert_eq!(wrapped, dispatched);
    assert_eq!(wrapped.len(), VERB_COUNT, "{wrapped:?}");
}

#[test]
fn package_manifest_wires_the_system_library() {
    let manifest = read_relative(MANIFEST);
    assert!(
        manifest.contains(".systemLibrary(name: \"CHyperMind\", path: \"Sources/CHyperMind\")"),
        "{manifest}"
    );
    assert!(
        manifest.contains(
            ".target(name: \"HyperMind\", dependencies: [\"CHyperMind\"], path: \"Sources/HyperMind\")"
        ),
        "{manifest}"
    );
    assert!(
        manifest.contains(".library(name: \"HyperMind\", targets: [\"HyperMind\"])"),
        "{manifest}"
    );
}

#[test]
fn readme_records_the_apple_platform_gate() {
    let readme = read_relative(README);
    assert!(readme.contains("## Apple-platform gate"), "{readme}");
}
