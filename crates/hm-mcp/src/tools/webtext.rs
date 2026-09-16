//! Bounded tag stripper for fetched textual payloads; it is not an HTML parser and renders nothing.
#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};

pub const MAXIMUM_EXTRACTED_BYTES: usize = 1024 * 1024;

const TEXTUAL_MEDIA_TYPES: [&str; 6] = [
    "application/json",
    "application/xhtml+xml",
    "text/csv",
    "text/html",
    "text/markdown",
    "text/plain",
];

const MARKUP_MEDIA_TYPES: [&str; 2] = ["application/xhtml+xml", "text/html"];

const BLOCK_ELEMENTS: [&str; 41] = [
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "br",
    "dd",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "li",
    "main",
    "nav",
    "ol",
    "option",
    "p",
    "pre",
    "section",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];

const MAXIMUM_ENTITY_BYTES: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedText {
    pub media_type: String,
    pub title: Option<String>,
    pub text: String,
}

#[must_use]
pub fn is_textual(media_type: &str) -> bool {
    TEXTUAL_MEDIA_TYPES.contains(&media_type)
}

pub fn extract_text(media_type: &str, bytes: &[u8]) -> Result<ExtractedText, Error> {
    if !is_textual(media_type) {
        return Err(Error::new(ErrorCode::OperationUnavailable));
    }
    let source = core::str::from_utf8(bytes).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    let (title, text) = if MARKUP_MEDIA_TYPES.contains(&media_type) {
        strip_html(source)
    } else {
        (None, source.trim().to_owned())
    };
    if text.len() > MAXIMUM_EXTRACTED_BYTES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    Ok(ExtractedText {
        media_type: media_type.to_owned(),
        title,
        text,
    })
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Separator {
    None,
    Space,
    Line,
}

struct Sink {
    text: String,
    pending: Separator,
}

impl Sink {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            text: String::with_capacity(capacity),
            pending: Separator::None,
        }
    }

    fn separate(&mut self, separator: Separator) {
        if !self.text.is_empty() && separator > self.pending {
            self.pending = separator;
        }
    }

    fn push_run(&mut self, run: &str) {
        for character in run.chars() {
            if character.is_ascii_whitespace() {
                self.separate(Separator::Space);
                continue;
            }
            match self.pending {
                Separator::None => {}
                Separator::Space => self.text.push(' '),
                Separator::Line => self.text.push('\n'),
            }
            self.pending = Separator::None;
            self.text.push(character);
        }
    }
}

struct Tag<'a> {
    name: &'a str,
    closing: bool,
    end: usize,
}

fn strip_html(source: &str) -> (Option<String>, String) {
    let bytes = source.as_bytes();
    let mut sink = Sink::with_capacity(source.len());
    let mut title: Option<String> = None;
    let mut index = 0;
    while index < bytes.len() {
        let Some(relative) = source[index..].find('<') else {
            sink.push_run(&decode_entities(&source[index..]));
            break;
        };
        if relative > 0 {
            sink.push_run(&decode_entities(&source[index..index + relative]));
        }
        let open = index + relative;
        if source[open..].starts_with("<!--") {
            index = source[open + 4..]
                .find("-->")
                .map_or(bytes.len(), |offset| open + 4 + offset + 3);
            continue;
        }
        let tag = read_tag(source, open);
        if !tag.closing
            && (tag.name.eq_ignore_ascii_case("script") || tag.name.eq_ignore_ascii_case("style"))
        {
            sink.separate(Separator::Line);
            index = skip_element(source, tag.name, tag.end);
            continue;
        }
        if !tag.closing && tag.name.eq_ignore_ascii_case("title") {
            let closing = find_ignore_ascii_case(source, "</title", tag.end);
            let body = &source[tag.end..closing.unwrap_or(bytes.len())];
            if title.is_none() {
                let mut heading = Sink::with_capacity(body.len());
                heading.push_run(&decode_entities(body));
                if !heading.text.is_empty() {
                    title = Some(heading.text);
                }
            }
            sink.separate(Separator::Line);
            index = closing.map_or(bytes.len(), |position| read_tag(source, position).end);
            continue;
        }
        if is_block_element(tag.name) {
            sink.separate(Separator::Line);
        }
        index = tag.end;
    }
    (title, sink.text)
}

fn is_block_element(name: &str) -> bool {
    BLOCK_ELEMENTS
        .iter()
        .any(|element| element.eq_ignore_ascii_case(name))
}

fn read_tag(source: &str, open: usize) -> Tag<'_> {
    let bytes = source.as_bytes();
    let mut cursor = open + 1;
    let closing = bytes.get(cursor) == Some(&b'/');
    if closing {
        cursor += 1;
    }
    let start = cursor;
    while cursor < bytes.len()
        && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'-' || bytes[cursor] == b':')
    {
        cursor += 1;
    }
    let name = &source[start..cursor];
    let mut quote: Option<u8> = None;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if quote.is_some_and(|open_quote| byte == open_quote) {
            quote = None;
        } else if quote.is_none() && (byte == b'"' || byte == b'\'') {
            quote = Some(byte);
        } else if quote.is_none() && byte == b'>' {
            return Tag {
                name,
                closing,
                end: cursor + 1,
            };
        }
        cursor += 1;
    }
    Tag {
        name,
        closing,
        end: bytes.len(),
    }
}

fn skip_element(source: &str, name: &str, from: usize) -> usize {
    let needle = format!("</{name}");
    let Some(closing) = find_ignore_ascii_case(source, &needle, from) else {
        return source.len();
    };
    read_tag(source, closing).end
}

fn find_ignore_ascii_case(source: &str, needle: &str, from: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let target = needle.as_bytes();
    if target.is_empty() || from > bytes.len() || bytes.len() - from < target.len() {
        return None;
    }
    (from..=bytes.len() - target.len())
        .find(|start| bytes[*start..*start + target.len()].eq_ignore_ascii_case(target))
}

fn decode_entities(source: &str) -> String {
    let mut decoded = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(offset) = rest.find('&') {
        decoded.push_str(&rest[..offset]);
        let after = &rest[offset + 1..];
        let terminator = after
            .char_indices()
            .take(MAXIMUM_ENTITY_BYTES)
            .find(|(_, character)| *character == ';')
            .map(|(position, _)| position);
        let Some(terminator) = terminator else {
            decoded.push('&');
            rest = after;
            continue;
        };
        let body = &after[..terminator];
        if let Some(character) = resolve_entity(body) {
            decoded.push(character);
        } else {
            decoded.push('&');
            decoded.push_str(body);
            decoded.push(';');
        }
        rest = &after[terminator + 1..];
    }
    decoded.push_str(rest);
    decoded
}

fn resolve_entity(body: &str) -> Option<char> {
    if let Some(digits) = body.strip_prefix('#') {
        let code = if let Some(hex) = digits.strip_prefix(['x', 'X']) {
            u32::from_str_radix(hex, 16).ok()?
        } else {
            digits.parse::<u32>().ok()?
        };
        return char::from_u32(code);
    }
    match body {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => None,
    }
}
