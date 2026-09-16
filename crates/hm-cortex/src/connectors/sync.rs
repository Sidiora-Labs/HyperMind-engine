#![allow(clippy::missing_errors_doc)]

use super::decode_hex;
use hm_core::{Error, ErrorCode};
use hm_schema::event::MAXIMUM_IDENTIFIER_BYTES;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

pub const MAXIMUM_SOURCE_PAGES: usize = 8;
pub const MAXIMUM_SOURCE_IDS: usize = 256;

const CONTENT_DIGEST_BYTES: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRequest {
    pub url: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

pub trait SourceTransport: Send + Sync {
    fn fetch(&self, request: &SourceRequest) -> Result<SourceResponse, Error>;
}

pub struct RecordedSourceTransport {
    exchanges: Mutex<Vec<(SourceRequest, SourceResponse)>>,
}

impl RecordedSourceTransport {
    #[must_use]
    pub fn new(exchanges: Vec<(SourceRequest, SourceResponse)>) -> Self {
        Self {
            exchanges: Mutex::new(exchanges),
        }
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.exchanges.lock().map_or(0, |exchanges| exchanges.len())
    }
}

impl SourceTransport for RecordedSourceTransport {
    fn fetch(&self, request: &SourceRequest) -> Result<SourceResponse, Error> {
        let mut exchanges = self
            .exchanges
            .lock()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
        if exchanges.is_empty() {
            return Err(Error::new(ErrorCode::BackendUnavailable));
        }
        let (recorded, response) = exchanges.remove(0);
        if recorded != *request {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        Ok(response)
    }
}

pub struct HttpSourceTransport {
    client: reqwest::blocking::Client,
}

impl HttpSourceTransport {
    pub fn new(timeout: Duration) -> Result<Self, Error> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
        Ok(Self { client })
    }
}

impl SourceTransport for HttpSourceTransport {
    fn fetch(&self, request: &SourceRequest) -> Result<SourceResponse, Error> {
        let mut call = self.client.get(&request.url);
        for (name, value) in &request.headers {
            call = call.header(name, value);
        }
        let response = call
            .send()
            .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
        let status = response.status().as_u16();
        let body = response
            .bytes()
            .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?
            .to_vec();
        Ok(SourceResponse { status, body })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRevisionListing {
    pub source_id: String,
    pub revision: Vec<u8>,
    pub content_digest: [u8; 32],
}

pub fn list_revisions(
    transport: &dyn SourceTransport,
    base_url: &str,
    access_token: &str,
) -> Result<Vec<SourceRevisionListing>, Error> {
    if base_url.is_empty() || access_token.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut listings: Vec<SourceRevisionListing> = Vec::new();
    let mut visited: Vec<String> = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAXIMUM_SOURCE_PAGES {
        let response = transport.fetch(&page_request(base_url, access_token, cursor.as_deref()))?;
        if !(200..300).contains(&response.status) {
            return Err(Error::new(ErrorCode::BackendUnavailable));
        }
        let page = parse_page(&response.body)?;
        if listings.len().saturating_add(page.listings.len()) > MAXIMUM_SOURCE_IDS {
            return Err(Error::new(ErrorCode::CapacityExceeded));
        }
        listings.extend(page.listings);
        let Some(next) = page.next_cursor else {
            return Ok(listings);
        };
        if visited.contains(&next) {
            return Err(Error::new(ErrorCode::SchemaInvalid));
        }
        visited.push(next.clone());
        cursor = Some(next);
    }
    Err(Error::new(ErrorCode::CapacityExceeded))
}

struct RevisionPage {
    listings: Vec<SourceRevisionListing>,
    next_cursor: Option<String>,
}

fn page_request(base_url: &str, access_token: &str, cursor: Option<&str>) -> SourceRequest {
    let base = base_url.trim_end_matches('/');
    let url = cursor.map_or_else(
        || format!("{base}/sources"),
        |cursor| format!("{base}/sources?cursor={cursor}"),
    );
    SourceRequest {
        url,
        headers: BTreeMap::from([
            ("accept".to_owned(), "application/json".to_owned()),
            ("authorization".to_owned(), format!("Bearer {access_token}")),
        ]),
    }
}

fn parse_page(body: &[u8]) -> Result<RevisionPage, Error> {
    let invalid = Error::new(ErrorCode::SchemaInvalid);
    let document = serde_json::from_slice::<Value>(body).map_err(|_| invalid)?;
    let Some(object) = document.as_object() else {
        return Err(invalid);
    };
    let Some(sources) = object.get("sources").and_then(Value::as_array) else {
        return Err(invalid);
    };
    let mut listings = Vec::with_capacity(sources.len());
    for source in sources {
        let Some(entry) = source.as_object() else {
            return Err(invalid);
        };
        let (Some(source_id), Some(revision), Some(content_digest)) = (
            entry.get("source_id").and_then(Value::as_str),
            entry.get("revision").and_then(Value::as_str),
            entry.get("content_digest").and_then(Value::as_str),
        ) else {
            return Err(invalid);
        };
        if !is_identifier(source_id) || !is_identifier(revision) {
            return Err(invalid);
        }
        let Some(content_digest) = decode_hex::<CONTENT_DIGEST_BYTES>(content_digest) else {
            return Err(invalid);
        };
        listings.push(SourceRevisionListing {
            source_id: source_id.to_owned(),
            revision: revision.as_bytes().to_vec(),
            content_digest,
        });
    }
    let next_cursor = match object.get("next_cursor") {
        None | Some(Value::Null) => None,
        Some(Value::String(cursor)) => {
            if cursor.is_empty()
                || cursor.len() > MAXIMUM_IDENTIFIER_BYTES
                || !cursor.bytes().all(is_cursor_byte)
            {
                return Err(invalid);
            }
            Some(cursor.clone())
        }
        Some(_) => return Err(invalid),
    };
    Ok(RevisionPage {
        listings,
        next_cursor,
    })
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAXIMUM_IDENTIFIER_BYTES
}

const fn is_cursor_byte(byte: u8) -> bool {
    matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}
