#![allow(clippy::missing_errors_doc)]
//! Playbook v1 documents are parsed by hand and line by line: an opening `%%`
//! fence, `key: value` header lines, a closing `%%` fence, then the instruction
//! body verbatim. Anything else is rejected, never repaired.

use hm_core::{Error, ErrorCode};
use hm_schema::event::MAXIMUM_IDENTIFIER_BYTES;
use hm_schema::events::ProcedureImported;

pub const MAXIMUM_DOCUMENT_BYTES: usize = 65_536;

const FENCE: &str = "%%";
const DEFAULT_PLAYBOOK_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeaderKey {
    Name,
    Outcomes,
    Preconditions,
    Strategy,
    Tools,
    Version,
}

const HEADER_KEYS: [(&str, HeaderKey); 6] = [
    ("name", HeaderKey::Name),
    ("outcomes", HeaderKey::Outcomes),
    ("preconditions", HeaderKey::Preconditions),
    ("strategy", HeaderKey::Strategy),
    ("tools", HeaderKey::Tools),
    ("version", HeaderKey::Version),
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Header {
    name: Option<String>,
    strategy: Option<String>,
    outcomes: Option<Vec<String>>,
    preconditions: Option<Vec<String>>,
    tools: Option<Vec<String>>,
    version: Option<u16>,
}

#[must_use]
pub fn procedure_id(source_uri: &str, name: &str) -> Vec<u8> {
    let mut hash = blake3::Hasher::new();
    hash.update(b"hypermind.playbook.v1\0");
    hash.update(&(source_uri.len() as u64).to_le_bytes());
    hash.update(source_uri.as_bytes());
    hash.update(name.as_bytes());
    hash.finalize().as_bytes().to_vec()
}

pub fn parse(source_uri: &str, document: &[u8]) -> Result<ProcedureImported, Error> {
    if document.len() > MAXIMUM_DOCUMENT_BYTES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    if source_uri.is_empty() {
        return Err(malformed());
    }
    if source_uri.len() > MAXIMUM_IDENTIFIER_BYTES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let text = std::str::from_utf8(document).map_err(|_| malformed())?;
    let (fenced, body) = split(text)?;
    let header = headers(fenced)?;
    let name = header.name.ok_or_else(malformed)?;
    let strategy = header.strategy.ok_or_else(malformed)?;
    let expected_outcomes = header.outcomes.ok_or_else(malformed)?;
    if body.trim().is_empty() {
        return Err(malformed());
    }
    if name.len() > MAXIMUM_IDENTIFIER_BYTES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    Ok(ProcedureImported {
        procedure_id: procedure_id(source_uri, &name),
        name,
        strategy,
        expected_outcomes,
        preconditions: header.preconditions.unwrap_or_default(),
        instructions: body.as_bytes().to_vec(),
        declared_tools: header.tools.unwrap_or_default(),
        source_uri: source_uri.to_owned(),
        source_digest: blake3::hash(document).as_bytes().to_vec(),
        playbook_version: header.version.unwrap_or(DEFAULT_PLAYBOOK_VERSION),
    })
}

fn split(text: &str) -> Result<(&str, &str), Error> {
    let (opening, after_opening) = take_line(text);
    if opening != FENCE {
        return Err(malformed());
    }
    let mut cursor = 0;
    while cursor < after_opening.len() {
        let (line, rest) = take_line(&after_opening[cursor..]);
        if line == FENCE {
            let body = after_opening.len() - rest.len();
            return Ok((&after_opening[..cursor], &after_opening[body..]));
        }
        cursor = after_opening.len() - rest.len();
    }
    Err(malformed())
}

fn take_line(text: &str) -> (&str, &str) {
    match text.find('\n') {
        Some(index) => (
            text[..index].trim_end_matches('\r'),
            &text[index.saturating_add(1)..],
        ),
        None => (text.trim_end_matches('\r'), ""),
    }
}

fn headers(fenced: &str) -> Result<Header, Error> {
    let mut header = Header::default();
    for line in fenced.lines() {
        let (key, value) = line.split_once(':').ok_or_else(malformed)?;
        let value = value.trim();
        if value.is_empty() {
            return Err(malformed());
        }
        match recognised(key.trim()).ok_or_else(malformed)? {
            HeaderKey::Name => claim(&mut header.name, value.to_owned())?,
            HeaderKey::Strategy => claim(&mut header.strategy, value.to_owned())?,
            HeaderKey::Outcomes => claim(&mut header.outcomes, entries(value)?)?,
            HeaderKey::Preconditions => claim(&mut header.preconditions, entries(value)?)?,
            HeaderKey::Tools => claim(&mut header.tools, entries(value)?)?,
            HeaderKey::Version => {
                claim(&mut header.version, value.parse().map_err(|_| malformed())?)?;
            }
        }
    }
    Ok(header)
}

fn recognised(key: &str) -> Option<HeaderKey> {
    HEADER_KEYS
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, recognised)| *recognised)
}

fn claim<T>(slot: &mut Option<T>, value: T) -> Result<(), Error> {
    if slot.is_some() {
        return Err(malformed());
    }
    *slot = Some(value);
    Ok(())
}

fn entries(value: &str) -> Result<Vec<String>, Error> {
    let mut entries = Vec::new();
    for entry in value.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            return Err(malformed());
        }
        entries.push(entry.to_owned());
    }
    Ok(entries)
}

fn malformed() -> Error {
    Error::new(ErrorCode::InvalidArgument)
}
