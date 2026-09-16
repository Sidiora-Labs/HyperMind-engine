#![allow(clippy::missing_errors_doc)]
//! Vocabulary sources are read as line-based RDF triples (N-Triples) only.
//! Turtle, RDF/XML and JSON-LD are not supported and are rejected, not skipped.

use crate::ingest::{IngestResult, IngestSource, ingest};
use hm_core::{Error, ErrorCode};
use hm_schema::event::{
    CURRENT_SCHEMA_VERSION, EventKind, MAXIMUM_VOCABULARY_ALIASES, MAXIMUM_VOCABULARY_NAME_BYTES,
    MAXIMUM_VOCABULARY_TERMS,
};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, Retention, Sensitivity, VocabularyCategory,
    VocabularyImported, VocabularyTerm,
};
use std::collections::{BTreeMap, BTreeSet};

pub const MAXIMUM_SOURCE_BYTES: usize = 1024 * 1024;
pub const SOURCE_MEDIA_TYPE: &str = "application/n-triples";

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const RDFS_SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDFS_SUB_PROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const OWL_DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
const OWL_NAMED_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";
const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
const OWL_SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
const SKOS_ALT_LABEL: &str = "http://www.w3.org/2004/02/skos/core#altLabel";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Recognised {
    Type,
    Alias,
    Equivalent,
    Parent,
}

const RECOGNISED_PREDICATES: [(&str, Recognised); 8] = [
    (OWL_EQUIVALENT_CLASS, Recognised::Equivalent),
    (OWL_EQUIVALENT_PROPERTY, Recognised::Equivalent),
    (OWL_SAME_AS, Recognised::Equivalent),
    (RDFS_LABEL, Recognised::Alias),
    (RDFS_SUB_CLASS_OF, Recognised::Parent),
    (RDFS_SUB_PROPERTY_OF, Recognised::Parent),
    (RDF_TYPE, Recognised::Type),
    (SKOS_ALT_LABEL, Recognised::Alias),
];

const DECLARED_CATEGORIES: [(&str, VocabularyCategory); 6] = [
    (OWL_CLASS, VocabularyCategory::EntityType),
    (OWL_DATATYPE_PROPERTY, VocabularyCategory::Relation),
    (OWL_NAMED_INDIVIDUAL, VocabularyCategory::EntityInstance),
    (OWL_OBJECT_PROPERTY, VocabularyCategory::Relation),
    (RDFS_CLASS, VocabularyCategory::EntityType),
    (RDF_PROPERTY, VocabularyCategory::Relation),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VocabularySource<'a> {
    pub vocabulary_id: &'a [u8],
    pub version: u16,
    pub source_uri: &'a str,
    pub document: &'a str,
    pub retention: Retention,
    pub sensitivity: Sensitivity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TripleObject<'a> {
    Iri(&'a str),
    Literal(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Triple<'a> {
    subject: &'a str,
    predicate: &'a str,
    object: TripleObject<'a>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SubjectTerms<'a> {
    category: Option<VocabularyCategory>,
    parent_term_id: Option<&'a str>,
    aliases: BTreeSet<String>,
    recognised: u32,
}

pub fn parse_terms(document: &str) -> Result<(Vec<VocabularyTerm>, u32), Error> {
    if document.len() > MAXIMUM_SOURCE_BYTES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let mut subjects: BTreeMap<&str, SubjectTerms<'_>> = BTreeMap::new();
    let mut ignored: u32 = 0;
    for line in document.lines() {
        let Some(triple) = parse_triple(line)? else {
            continue;
        };
        if !apply(&mut subjects, &triple) {
            ignored = ignored.saturating_add(1);
        }
    }
    let mut terms = Vec::new();
    for (term_id, subject) in subjects {
        let Some(category) = subject.category else {
            ignored = ignored.saturating_add(subject.recognised);
            continue;
        };
        terms.push(term(term_id, category, &subject)?);
    }
    if terms.len() > MAXIMUM_VOCABULARY_TERMS {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    Ok((terms, ignored))
}

pub fn import_envelope(source: &VocabularySource<'_>) -> Result<IngestResult, Error> {
    let (terms, ignored_triples) = parse_terms(source.document)?;
    if source.vocabulary_id.is_empty()
        || source.version == 0
        || source.source_uri.is_empty()
        || terms.is_empty()
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    let digest = *blake3::hash(source.document.as_bytes()).as_bytes();
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::VocabularyImported(Box::new(VocabularyImported {
            vocabulary_id: source.vocabulary_id.to_vec(),
            version: source.version,
            source_uri: source.source_uri.to_owned(),
            source_media_type: SOURCE_MEDIA_TYPE.to_owned(),
            source_digest: digest.to_vec(),
            terms,
            ignored_triples,
        })),
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: Authority::DerivedInference,
        retention: source.retention,
        sensitivity: source.sensitivity,
        event_time_ns: 0,
    };
    ingest(
        EventKind::VocabularyImported,
        IngestSource::User,
        envelope,
        false,
    )
}

fn term(
    term_id: &str,
    category: VocabularyCategory,
    subject: &SubjectTerms<'_>,
) -> Result<VocabularyTerm, Error> {
    let canonical_name = local_name(term_id);
    if canonical_name.is_empty() {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    if term_id.len() > MAXIMUM_VOCABULARY_NAME_BYTES
        || subject.aliases.len() > MAXIMUM_VOCABULARY_ALIASES
        || subject
            .aliases
            .iter()
            .any(|alias| alias.len() > MAXIMUM_VOCABULARY_NAME_BYTES)
        || subject
            .parent_term_id
            .is_some_and(|parent| parent.len() > MAXIMUM_VOCABULARY_NAME_BYTES)
    {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let aliases = if subject.aliases.is_empty() {
        None
    } else {
        Some(subject.aliases.iter().cloned().collect())
    };
    Ok(VocabularyTerm {
        term_id: term_id.to_owned(),
        canonical_name: canonical_name.to_owned(),
        category,
        parent_term_id: subject.parent_term_id.map(str::to_owned),
        aliases,
    })
}

fn apply<'a>(subjects: &mut BTreeMap<&'a str, SubjectTerms<'a>>, triple: &Triple<'a>) -> bool {
    let Some(recognised) = recognised_predicate(triple.predicate) else {
        return false;
    };
    let contribution = match (recognised, &triple.object) {
        (Recognised::Type, TripleObject::Iri(iri)) => {
            let Some(category) = declared_category(iri) else {
                return false;
            };
            Contribution::Category(category)
        }
        (Recognised::Alias, TripleObject::Literal(value)) => {
            if value.is_empty() {
                return false;
            }
            Contribution::Alias(value.clone())
        }
        (Recognised::Equivalent, TripleObject::Iri(iri)) => {
            let name = local_name(iri);
            if name.is_empty() {
                return false;
            }
            Contribution::Alias(name.to_owned())
        }
        (Recognised::Parent, TripleObject::Iri(iri)) => Contribution::Parent(iri),
        _ => return false,
    };
    let entry = subjects.entry(triple.subject).or_default();
    match contribution {
        Contribution::Category(category) => {
            if entry.category.is_none() {
                entry.category = Some(category);
            }
        }
        Contribution::Alias(alias) => {
            entry.aliases.insert(alias);
        }
        Contribution::Parent(parent) => {
            if entry.parent_term_id.is_none() {
                entry.parent_term_id = Some(parent);
            }
        }
    }
    entry.recognised = entry.recognised.saturating_add(1);
    true
}

enum Contribution<'a> {
    Category(VocabularyCategory),
    Alias(String),
    Parent(&'a str),
}

fn recognised_predicate(predicate: &str) -> Option<Recognised> {
    RECOGNISED_PREDICATES
        .iter()
        .find(|(iri, _)| *iri == predicate)
        .map(|(_, recognised)| *recognised)
}

fn declared_category(object: &str) -> Option<VocabularyCategory> {
    DECLARED_CATEGORIES
        .iter()
        .find(|(iri, _)| *iri == object)
        .map(|(_, category)| *category)
}

fn local_name(iri: &str) -> &str {
    match iri.rfind(['#', '/']) {
        Some(index) => &iri[index + 1..],
        None => iri,
    }
}

fn parse_triple(line: &str) -> Result<Option<Triple<'_>>, Error> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }
    let (subject, rest) = take_iri(trimmed)?;
    let (predicate, rest) = take_iri(skip_spaces(rest))?;
    let (object, rest) = take_object(skip_spaces(rest))?;
    let rest = skip_spaces(rest).strip_prefix('.').ok_or_else(malformed)?;
    if !skip_spaces(rest).is_empty() {
        return Err(malformed());
    }
    Ok(Some(Triple {
        subject,
        predicate,
        object,
    }))
}

fn skip_spaces(input: &str) -> &str {
    input.trim_start_matches([' ', '\t'])
}

fn take_iri(input: &str) -> Result<(&str, &str), Error> {
    let rest = input.strip_prefix('<').ok_or_else(malformed)?;
    let end = rest.find('>').ok_or_else(malformed)?;
    let iri = &rest[..end];
    if iri.is_empty()
        || iri.contains('<')
        || iri.contains('"')
        || iri.chars().any(char::is_whitespace)
    {
        return Err(malformed());
    }
    Ok((iri, &rest[end + 1..]))
}

fn take_object(input: &str) -> Result<(TripleObject<'_>, &str), Error> {
    if input.starts_with('<') {
        let (iri, rest) = take_iri(input)?;
        return Ok((TripleObject::Iri(iri), rest));
    }
    if input.starts_with('"') {
        let (value, rest) = take_literal(input)?;
        return Ok((TripleObject::Literal(value), rest));
    }
    Err(malformed())
}

fn take_literal(input: &str) -> Result<(String, &str), Error> {
    let mut rest = input.strip_prefix('"').ok_or_else(malformed)?;
    let mut value = String::new();
    loop {
        let mut characters = rest.chars();
        let Some(character) = characters.next() else {
            return Err(malformed());
        };
        rest = characters.as_str();
        match character {
            '"' => break,
            '\\' => {
                let (decoded, remainder) = take_escape(rest)?;
                value.push(decoded);
                rest = remainder;
            }
            _ => value.push(character),
        }
    }
    if let Some(remainder) = rest.strip_prefix("^^") {
        let (_, remainder) = take_iri(remainder)?;
        rest = remainder;
    } else if let Some(remainder) = rest.strip_prefix('@') {
        let end = remainder
            .find(|character: char| !character.is_ascii_alphanumeric() && character != '-')
            .unwrap_or(remainder.len());
        if end == 0 {
            return Err(malformed());
        }
        rest = &remainder[end..];
    }
    Ok((value, rest))
}

fn take_escape(input: &str) -> Result<(char, &str), Error> {
    let mut characters = input.chars();
    let Some(marker) = characters.next() else {
        return Err(malformed());
    };
    let rest = characters.as_str();
    let simple = match marker {
        't' => Some('\t'),
        'b' => Some('\u{8}'),
        'n' => Some('\n'),
        'r' => Some('\r'),
        'f' => Some('\u{c}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '\\' => Some('\\'),
        _ => None,
    };
    if let Some(decoded) = simple {
        return Ok((decoded, rest));
    }
    let width = match marker {
        'u' => 4,
        'U' => 8,
        _ => return Err(malformed()),
    };
    let digits = rest.get(..width).ok_or_else(malformed)?;
    if !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return Err(malformed());
    }
    let code = u32::from_str_radix(digits, 16).map_err(|_| malformed())?;
    let decoded = char::from_u32(code).ok_or_else(malformed)?;
    Ok((decoded, &rest[width..]))
}

fn malformed() -> Error {
    Error::new(ErrorCode::SchemaInvalid)
}
