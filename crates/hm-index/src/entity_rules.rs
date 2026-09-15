use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EntityKind {
    Url,
    Domain,
    Path,
    HexId,
    StructuredId,
    ProperNoun,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entity {
    pub canonical: String,
    pub aliases: Vec<String>,
    pub kind: EntityKind,
}

#[must_use]
pub fn extract_entities(text: &str) -> Vec<Entity> {
    let tokens: Vec<_> = text.split_whitespace().map(clean_token).collect();
    let mut entities = BTreeMap::new();
    for token in &tokens {
        if token.is_empty() {
            continue;
        }
        if let Some(entity) = url_entity(token) {
            insert(&mut entities, entity);
        }
        if is_domain(token) {
            let canonical = normalize_domain(token);
            insert(
                &mut entities,
                Entity {
                    aliases: domain_aliases(&canonical),
                    canonical,
                    kind: EntityKind::Domain,
                },
            );
        }
        if is_path(token) {
            insert(
                &mut entities,
                Entity {
                    canonical: (*token).to_owned(),
                    aliases: vec![normalize(token)],
                    kind: EntityKind::Path,
                },
            );
        }
        if is_hex_id(token) {
            let canonical = token.to_ascii_lowercase();
            insert(
                &mut entities,
                Entity {
                    aliases: vec![canonical.clone()],
                    canonical,
                    kind: EntityKind::HexId,
                },
            );
        } else if is_structured_id(token) {
            let canonical = normalize(token);
            insert(
                &mut entities,
                Entity {
                    aliases: vec![canonical.clone()],
                    canonical,
                    kind: EntityKind::StructuredId,
                },
            );
        }
    }
    for phrase in proper_noun_phrases(&tokens) {
        let canonical = phrase.join(" ");
        let mut aliases = vec![normalize(&canonical)];
        if phrase.len() > 2 {
            aliases.extend(
                phrase
                    .windows(2)
                    .map(|pair| normalize(&format!("{} {}", pair[0], pair[1]))),
            );
        }
        aliases.sort();
        aliases.dedup();
        insert(
            &mut entities,
            Entity {
                canonical,
                aliases,
                kind: EntityKind::ProperNoun,
            },
        );
    }
    entities.into_values().collect()
}

#[must_use]
pub fn extract_query_entities(query: &str, turn_text: &str) -> Vec<Entity> {
    let mut entities = BTreeMap::new();
    for entity in extract_entities(query)
        .into_iter()
        .chain(extract_entities(turn_text))
    {
        insert(&mut entities, entity);
    }
    entities.into_values().collect()
}

fn insert(entities: &mut BTreeMap<(EntityKind, String), Entity>, entity: Entity) {
    entities.insert((entity.kind, normalize(&entity.canonical)), entity);
}

fn clean_token(token: &str) -> &str {
    token.trim_matches(|character: char| {
        matches!(
            character,
            ',' | ';' | '!' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\''
        )
    })
}

fn url_entity(token: &str) -> Option<Entity> {
    let remainder = token
        .strip_prefix("https://")
        .or_else(|| token.strip_prefix("http://"))?;
    let domain = remainder.split(['/', '?', '#']).next()?;
    if !is_domain(domain) {
        return None;
    }
    let canonical = token.to_owned();
    let mut aliases = vec![normalize(token)];
    aliases.extend(domain_aliases(&normalize_domain(domain)));
    aliases.sort();
    aliases.dedup();
    Some(Entity {
        canonical,
        aliases,
        kind: EntityKind::Url,
    })
}

fn is_domain(token: &str) -> bool {
    let token = token.trim_end_matches(['.', ':']);
    !token.contains('/')
        && token.contains('.')
        && token.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
        && token.rsplit_once('.').is_some_and(|(_, suffix)| {
            suffix.len() >= 2 && suffix.bytes().all(|b| b.is_ascii_alphabetic())
        })
}

fn normalize_domain(domain: &str) -> String {
    domain.trim_end_matches(['.', ':']).to_ascii_lowercase()
}

fn domain_aliases(domain: &str) -> Vec<String> {
    let without_www = domain.strip_prefix("www.").unwrap_or(domain);
    if without_www == domain {
        vec![domain.to_owned()]
    } else {
        vec![domain.to_owned(), without_www.to_owned()]
    }
}

fn is_path(token: &str) -> bool {
    token.starts_with('/')
        || token.starts_with("./")
        || token.starts_with("../")
        || token.starts_with("~/")
        || (token.contains('/')
            && !token.contains("://")
            && token
                .bytes()
                .all(|byte| !byte.is_ascii_whitespace() && byte != b'\\'))
}

fn is_hex_id(token: &str) -> bool {
    let digits = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
        .unwrap_or(token);
    digits.len() >= 8 && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_structured_id(token: &str) -> bool {
    token.len() >= 3
        && token.bytes().any(|byte| byte.is_ascii_alphabetic())
        && token.bytes().any(|byte| byte.is_ascii_digit())
        && token
            .bytes()
            .any(|byte| matches!(byte, b'-' | b'_' | b':' | b'/'))
        && token.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'/' | b'.')
        })
}

fn proper_noun_phrases<'a>(tokens: &'a [&'a str]) -> Vec<Vec<&'a str>> {
    let mut phrases = Vec::new();
    let mut current = Vec::new();
    for token in tokens {
        if is_proper_noun_token(token) {
            current.push(*token);
        } else if !current.is_empty() {
            phrases.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        phrases.push(current);
    }
    phrases
}

fn is_proper_noun_token(token: &str) -> bool {
    token.len() > 1
        && !token.contains(['/', '.', ':', '_'])
        && token.chars().next().is_some_and(char::is_uppercase)
        && token
            .chars()
            .all(|character| character.is_alphanumeric() || character == '-')
}

fn normalize(value: &str) -> String {
    value.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_every_entity_family_and_query_context() {
        let entities = extract_query_entities(
            "Find GH-123 at https://www.Example.com/docs",
            "Alice Smith changed src/main.rs near 0xdeadbeef",
        );
        assert!(entities.iter().any(|entity| entity.kind == EntityKind::Url));
        assert!(
            entities
                .iter()
                .any(|entity| entity.kind == EntityKind::Path)
        );
        assert!(
            entities
                .iter()
                .any(|entity| entity.kind == EntityKind::HexId)
        );
        assert!(
            entities
                .iter()
                .any(|entity| entity.kind == EntityKind::StructuredId)
        );
        let person = entities
            .iter()
            .find(|entity| entity.canonical == "Alice Smith")
            .expect("proper noun phrase");
        assert_eq!(person.aliases, ["alice smith"]);
        let url = entities
            .iter()
            .find(|entity| entity.kind == EntityKind::Url)
            .expect("URL");
        assert!(url.aliases.iter().any(|alias| alias == "example.com"));
    }
}
