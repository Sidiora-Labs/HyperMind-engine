use hypermind::{
    HM_ABI_VERSION_MAJOR, HM_ABI_VERSION_MINOR, HM_EXPORTED_SYMBOLS, HM_STATUS_TABLE, HmStatus,
    hm_abi_version,
};
use std::path::PathBuf;

const SOURCES: [&str; 5] = [
    "src/lib.rs",
    "src/status.rs",
    "src/engine.rs",
    "src/call.rs",
    "src/waiter.rs",
];
const HEADER: &str = "include/hypermind.h";
const VARIANTS: [(HmStatus, &str); 8] = [
    (HmStatus::Ok, "HM_STATUS_OK"),
    (HmStatus::NullPointer, "HM_STATUS_NULL_POINTER"),
    (HmStatus::InvalidUtf8, "HM_STATUS_INVALID_UTF8"),
    (HmStatus::InvalidArgument, "HM_STATUS_INVALID_ARGUMENT"),
    (HmStatus::HandleClosed, "HM_STATUS_HANDLE_CLOSED"),
    (HmStatus::Runtime, "HM_STATUS_RUNTIME"),
    (HmStatus::Panic, "HM_STATUS_PANIC"),
    (HmStatus::Kernel, "HM_STATUS_KERNEL"),
];

fn read(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{relative} is readable: {error}"))
}

fn identifier(text: &str) -> String {
    text.chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect()
}

fn source_exports(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        let declaration = trimmed
            .strip_prefix("pub extern \"C\" fn ")
            .or_else(|| trimmed.strip_prefix("pub unsafe extern \"C\" fn "));
        if let Some(declaration) = declaration {
            names.push(identifier(declaration));
        }
    }
    names
}

fn header_declarations(header: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in header.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("/*") || trimmed.starts_with('*') || trimmed.starts_with("//") {
            continue;
        }
        if !trimmed.ends_with(");") {
            continue;
        }
        let Some(open) = trimmed.find('(') else {
            continue;
        };
        let prefix = &trimmed[..open];
        let start = prefix
            .rfind(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .map_or(0, |index| index + 1);
        let name = &prefix[start..];
        if name.starts_with("hm_") {
            names.push(name.to_owned());
        }
    }
    names
}

fn expected_symbols() -> Vec<String> {
    let mut expected: Vec<String> = HM_EXPORTED_SYMBOLS
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    expected.sort();
    expected
}

#[test]
fn source_exports_match_the_table() {
    let mut found: Vec<String> = SOURCES
        .iter()
        .flat_map(|source| source_exports(&read(source)))
        .collect();
    found.sort();
    assert_eq!(found, expected_symbols());

    let sorted = expected_symbols();
    assert_eq!(
        HM_EXPORTED_SYMBOLS.to_vec(),
        sorted.iter().map(String::as_str).collect::<Vec<&str>>()
    );
}

#[test]
fn header_declares_exactly_the_exported_symbols() {
    let mut declared = header_declarations(&read(HEADER));
    declared.sort();
    assert_eq!(declared, expected_symbols());
}

#[test]
fn status_values_match_the_header() {
    let header = read(HEADER);
    let mut declared: Vec<(String, i32)> = Vec::new();
    for line in header.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if !trimmed.starts_with("HM_STATUS_") {
            continue;
        }
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };
        let value = value
            .trim()
            .parse::<i32>()
            .unwrap_or_else(|error| panic!("{trimmed} carries an integer value: {error}"));
        declared.push((name.trim().to_owned(), value));
    }

    let table: Vec<(String, i32)> = HM_STATUS_TABLE
        .iter()
        .map(|(name, value)| ((*name).to_owned(), *value))
        .collect();
    assert_eq!(declared, table);

    for (index, (variant, name)) in VARIANTS.iter().enumerate() {
        assert_eq!(HM_STATUS_TABLE[index], (*name, *variant as i32));
    }
}

#[test]
fn header_version_macros_match_hm_abi_version() {
    let header = read(HEADER);
    let macro_value = |wanted: &str| -> u32 {
        header
            .lines()
            .find_map(|line| {
                let rest = line.trim().strip_prefix("#define ")?;
                let (name, value) = rest.split_once(' ')?;
                (name == wanted).then(|| {
                    value
                        .trim()
                        .parse::<u32>()
                        .unwrap_or_else(|error| panic!("{wanted} is an integer: {error}"))
                })
            })
            .unwrap_or_else(|| panic!("the header defines {wanted}"))
    };

    let major = macro_value("HM_ABI_VERSION_MAJOR");
    let minor = macro_value("HM_ABI_VERSION_MINOR");
    assert_eq!(major, HM_ABI_VERSION_MAJOR);
    assert_eq!(minor, HM_ABI_VERSION_MINOR);
    assert_eq!(hm_abi_version(), (major << 16) | minor);
}
