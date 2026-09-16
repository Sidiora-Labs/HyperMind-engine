#![allow(clippy::missing_errors_doc)]

use regex::Regex;
use std::collections::BTreeSet;
use std::sync::OnceLock;

const GENERIC_PHRASE_MARKERS: [&str; 23] = [
    "this concept",
    "this memory concept",
    "expanding digital landscape",
    "fundamentally unknowable",
    "service gravity",
    "critical challenge",
    "inevitable future",
    "broader context",
    "deeper understanding",
    "multifaceted",
    "nuanced understanding",
    "holistic approach",
    "inherent complexity",
    "paradigm",
    "interconnected",
    "transformative",
    "this pattern unifies",
    "this pattern connects",
    "this pattern bridges",
    "this memory concept integrates",
    "adaptive knowledge",
    "structured coordination",
    "transparent boundaries",
];

const STOP_WORDS: &[&str] = &[
    "the",
    "a",
    "an",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "have",
    "has",
    "had",
    "do",
    "does",
    "did",
    "will",
    "would",
    "could",
    "should",
    "may",
    "might",
    "shall",
    "can",
    "need",
    "ought",
    "used",
    "to",
    "of",
    "in",
    "on",
    "at",
    "by",
    "for",
    "with",
    "about",
    "against",
    "between",
    "into",
    "through",
    "during",
    "before",
    "after",
    "above",
    "below",
    "from",
    "up",
    "down",
    "and",
    "but",
    "or",
    "nor",
    "so",
    "yet",
    "both",
    "either",
    "neither",
    "not",
    "only",
    "same",
    "than",
    "too",
    "very",
    "just",
    "because",
    "as",
    "until",
    "while",
    "although",
    "though",
    "that",
    "this",
    "these",
    "those",
    "it",
    "its",
    "i",
    "me",
    "my",
    "myself",
    "we",
    "our",
    "ours",
    "ourselves",
    "you",
    "your",
    "yours",
    "he",
    "him",
    "his",
    "she",
    "her",
    "hers",
    "they",
    "them",
    "their",
    "theirs",
    "what",
    "which",
    "who",
    "when",
    "where",
    "why",
    "how",
    "all",
    "each",
    "every",
    "no",
    "more",
    "most",
    "other",
    "some",
    "such",
    "then",
    "also",
    "if",
    "now",
    "like",
    "well",
    "even",
    "back",
    "any",
    "there",
    "think",
    "see",
    "know",
    "get",
    "one",
    "two",
    "three",
    "new",
    "good",
    "first",
    "last",
    "long",
    "great",
    "little",
    "own",
    "right",
    "big",
    "high",
    "different",
    "small",
    "large",
    "next",
    "early",
    "young",
    "old",
    "public",
    "private",
    "real",
    "best",
    "free",
    "much",
    "want",
    "make",
    "time",
    "year",
    "day",
    "way",
    "man",
    "many",
    "look",
    "come",
    "still",
    "here",
    "take",
    "give",
    "use",
    "find",
    "tell",
    "ask",
    "seem",
    "feel",
    "leave",
    "call",
    "keep",
    "let",
    "begin",
    "show",
    "hear",
    "play",
    "run",
    "move",
    "live",
    "believe",
    "hold",
    "bring",
    "happen",
    "write",
    "provide",
    "sit",
    "stand",
    "lose",
    "pay",
    "meet",
    "include",
    "continue",
    "set",
    "learn",
    "change",
    "lead",
    "understand",
    "watch",
    "follow",
    "stop",
    "create",
    "speak",
    "read",
    "spend",
];

const STOP_CAPS: &[&str] = &[
    "The",
    "This",
    "That",
    "These",
    "Those",
    "When",
    "What",
    "Which",
    "Where",
    "While",
    "After",
    "Before",
    "Both",
    "Each",
    "Every",
    "From",
    "With",
    "Without",
    "Since",
    "Then",
    "There",
    "They",
    "Their",
    "Them",
    "Because",
    "However",
    "Instead",
    "Also",
    "Only",
    "Some",
    "Many",
    "Most",
    "More",
    "Such",
    "Over",
    "Under",
    "Into",
    "Given",
    "And",
    "But",
    "For",
    "Not",
    "Its",
    "Our",
    "You",
    "Your",
    "His",
    "Her",
    "She",
    "Who",
    "How",
    "Why",
    "Yes",
    "Now",
    "Here",
    "Thus",
    "Hence",
    "Although",
    "Though",
    "Unless",
    "Until",
    "Once",
    "Still",
    "Even",
    "Just",
    "Like",
    "Rather",
    "Whether",
    "Beyond",
    "Within",
    "Across",
    "Among",
    "Between",
    "During",
    "Through",
    "Toward",
    "Towards",
    "Upon",
    "About",
    "Above",
    "Below",
    "Behind",
    "Around",
    "Along",
    "Against",
    "Despite",
    "Except",
    "Including",
    "Regarding",
    "Perhaps",
    "Maybe",
    "Often",
    "Sometimes",
    "Usually",
    "Always",
    "Never",
    "Already",
    "Another",
    "Any",
    "All",
    "One",
    "Two",
    "Three",
    "First",
    "Second",
    "Third",
    "New",
    "Old",
    "Nothing",
    "Something",
    "Everything",
    "Anything",
    "Someone",
    "Later",
    "Earlier",
    "Today",
    "Tonight",
    "Yesterday",
    "Tomorrow",
    "Meanwhile",
    "Otherwise",
    "Therefore",
    "Yet",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThoughtQualityOptions {
    pub minimum_grounding_micros: u32,
    pub require_sentence_end: bool,
    pub minimum_length: usize,
    pub maximum_length: usize,
}

impl Default for ThoughtQualityOptions {
    fn default() -> Self {
        Self {
            minimum_grounding_micros: 250_000,
            require_sentence_end: true,
            minimum_length: 20,
            maximum_length: 2_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThoughtQualityResult {
    pub accepted: bool,
    pub grounding_micros: Option<u32>,
    pub generic_hits: Vec<String>,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewriteGuardResult {
    pub accepted: bool,
    pub reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LabelledDecision {
    pub accepted: bool,
    pub expected_accepted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClassifierMetrics {
    pub examples: u64,
    pub predicted_rejections: u64,
    pub true_rejections: u64,
    pub rejection_precision_micros: Option<u32>,
}

#[must_use]
pub fn measure_classifier(decisions: &[LabelledDecision]) -> ClassifierMetrics {
    let predicted_rejections = decisions
        .iter()
        .filter(|decision| !decision.accepted)
        .count() as u64;
    let true_rejections = decisions
        .iter()
        .filter(|decision| !decision.accepted && !decision.expected_accepted)
        .count() as u64;
    let rejection_precision_micros = (predicted_rejections != 0).then(|| {
        u32::try_from(true_rejections.saturating_mul(1_000_000) / predicted_rejections)
            .unwrap_or(1_000_000)
    });
    ClassifierMetrics {
        examples: decisions.len() as u64,
        predicted_rejections,
        true_rejections,
        rejection_precision_micros,
    }
}

#[must_use]
pub fn grounding_score_micros(text: &str, evidence: &[&str]) -> u32 {
    let thought = extract_keywords(text, 50);
    if thought.is_empty() {
        return 1_000_000;
    }
    let evidence = extract_keywords(&evidence.join(" "), 500)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let hits = thought
        .iter()
        .filter(|keyword| evidence.contains(*keyword))
        .count();
    u32::try_from(hits.saturating_mul(1_000_000) / thought.len()).unwrap_or(1_000_000)
}

#[must_use]
pub fn assess_thought(
    text: &str,
    evidence: &[&str],
    options: ThoughtQualityOptions,
) -> ThoughtQualityResult {
    let trimmed = text.trim();
    let mut reasons = Vec::new();
    if trimmed.len() < options.minimum_length {
        reasons.push(format!("too short (<{} chars)", options.minimum_length));
    }
    if trimmed.len() > options.maximum_length {
        reasons.push(format!("too long (>{} chars)", options.maximum_length));
    }
    if options.require_sentence_end && !sentence_complete(trimmed) {
        reasons.push("does not end with sentence punctuation (possible truncation)".to_owned());
    }
    if markdown_leak(trimmed) {
        reasons.push("markdown formatting leaked into thought".to_owned());
    }
    if self_referential_opener(trimmed) {
        reasons.push("describes the memory rather than its subject (meta-text opener)".to_owned());
    }
    if placeholder_regex().is_match(trimmed) {
        reasons
            .push("internal placeholder scaffolding leaked into thought (Concept A/B)".to_owned());
    }
    let lower = trimmed.to_lowercase();
    let generic_hits = GENERIC_PHRASE_MARKERS
        .iter()
        .filter(|marker| lower.contains(**marker))
        .map(|marker| (*marker).to_owned())
        .collect::<Vec<_>>();
    if generic_hits.len() >= 2 {
        reasons.push(format!(
            "generic phrasing ({} marker hits)",
            generic_hits.len()
        ));
    }
    let grounding_micros = if evidence.is_empty() {
        if generic_hits.len() == 1 {
            reasons.push(
                "generic phrasing (marker hit, no evidence available to check grounding)"
                    .to_owned(),
            );
        }
        None
    } else {
        let grounding = grounding_score_micros(trimmed, evidence);
        if grounding < options.minimum_grounding_micros {
            reasons.push(format!(
                "ungrounded ({grounding} < {} keyword overlap with evidence)",
                options.minimum_grounding_micros
            ));
        } else if generic_hits.len() == 1
            && grounding < options.minimum_grounding_micros.saturating_add(150_000)
        {
            reasons.push(format!(
                "generic phrasing with marginal grounding ({grounding})"
            ));
        }
        Some(grounding)
    };
    ThoughtQualityResult {
        accepted: reasons.is_empty(),
        grounding_micros,
        generic_hits,
        reasons,
    }
}

#[must_use]
pub fn check_rewrite(name: &str, old: &str, next: &str, evidence: &[&str]) -> RewriteGuardResult {
    let next = next.trim();
    let evidence = evidence.join(" ");
    let mut reasons = Vec::new();
    if boilerplate_regex().is_match(next) {
        reasons.push("boilerplate opener".to_owned());
    }
    if opens_with_name(next, name) && !opens_with_name(old, name) {
        reasons.push("name restated as opener".to_owned());
    }
    let lost_numbers = captures(number_regex(), old)
        .into_iter()
        .filter(|value| !next.contains(value))
        .take(5)
        .collect::<Vec<_>>();
    if !lost_numbers.is_empty() {
        reasons.push(format!("dropped numbers: {}", lost_numbers.join(" ")));
    }
    let lost_quotes = quoted(old)
        .into_iter()
        .filter(|value| !next.contains(value))
        .count();
    if lost_quotes != 0 {
        reasons.push(format!("dropped quotations: {lost_quotes}"));
    }
    if !captures(first_person_regex(), old).is_empty()
        && captures(first_person_regex(), next).is_empty()
    {
        reasons.push("first person became third person".to_owned());
    }
    let old_entities = entities(old);
    let next_entities = entities(next);
    let lost_entities = old_entities
        .difference(&next_entities)
        .take(8)
        .cloned()
        .collect::<Vec<_>>();
    if !lost_entities.is_empty() {
        reasons.push(format!("dropped entities: {}", lost_entities.join(", ")));
    }
    let known = entities(&format!("{old} {evidence}"));
    let foreign = next_entities
        .difference(&known)
        .take(8)
        .cloned()
        .collect::<Vec<_>>();
    if !foreign.is_empty() {
        reasons.push(format!("new entities: {}", foreign.join(", ")));
    }
    let old_hedges = hedges(old);
    let next_hedges = hedges(next);
    let dropped_hedges = old_hedges
        .difference(&next_hedges)
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    if !dropped_hedges.is_empty() {
        reasons.push(format!("dropped hedges: {}", dropped_hedges.join(", ")));
    }
    let allowed = hedges(&format!("{old} {evidence}"));
    let added = next_hedges
        .difference(&allowed)
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    if !added.is_empty() {
        reasons.push(format!("added hedges: {}", added.join(", ")));
    }
    RewriteGuardResult {
        accepted: reasons.is_empty(),
        reasons,
    }
}

fn extract_keywords(text: &str, maximum: usize) -> Vec<String> {
    let stop = STOP_WORDS.iter().copied().collect::<BTreeSet<_>>();
    let normalized = text
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || character.is_ascii_whitespace()
                || character == '\''
                || character == '-'
            {
                character
            } else {
                ' '
            }
        })
        .collect::<String>();
    let mut seen = BTreeSet::new();
    normalized
        .split_whitespace()
        .map(|word| word.trim_matches(['\'', '-']))
        .filter(|word| {
            word.len() >= 3 && !stop.contains(word) && !word.bytes().all(|b| b.is_ascii_digit())
        })
        .filter(|word| seen.insert((*word).to_owned()))
        .take(maximum)
        .map(str::to_owned)
        .collect()
}

fn sentence_complete(text: &str) -> bool {
    let text = text.trim_end_matches(['"', '\'', ')', ']', '”']);
    text.ends_with(['.', '!', '?'])
}

fn markdown_leak(text: &str) -> bool {
    text.lines().any(|line| {
        let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
        (1..=6).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ')
    }) || paired_marker(text, "**")
        || paired_marker(text, "__")
}

fn paired_marker(text: &str, marker: &str) -> bool {
    let mut offset = 0;
    while let Some(start) = text[offset..].find(marker) {
        let start = offset + start;
        let boundary = start == 0
            || text[..start]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace);
        let content_start = start + marker.len();
        if boundary && let Some(end) = text[content_start..].find(marker) {
            let content = &text[content_start..content_start + end];
            if !content.is_empty() && content.trim() == content {
                return true;
            }
        }
        offset = content_start;
    }
    false
}

fn self_referential_opener(text: &str) -> bool {
    self_referential_regex().is_match(text)
}

fn opens_with_name(text: &str, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || !text.to_lowercase().starts_with(&name.to_lowercase()) {
        return false;
    }
    name_opener_regex().is_match(&text[name.len()..])
}

fn captures(regex: &Regex, text: &str) -> Vec<String> {
    regex
        .find_iter(text)
        .map(|value| value.as_str().to_owned())
        .collect()
}

fn quoted(text: &str) -> Vec<String> {
    quote_regex()
        .captures_iter(text)
        .filter_map(|capture| capture.get(1).or_else(|| capture.get(2)))
        .map(|value| value.as_str().to_owned())
        .collect()
}

fn entities(text: &str) -> BTreeSet<String> {
    capitalized_regex()
        .find_iter(text)
        .map(|value| value.as_str())
        .filter(|value| !STOP_CAPS.contains(value))
        .map(str::to_owned)
        .collect()
}

fn hedges(text: &str) -> BTreeSet<String> {
    hedge_regex()
        .find_iter(text)
        .map(|value| value.as_str().to_lowercase())
        .collect()
}

fn placeholder_regex() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"\bConcept\s+[A-Z]\b").expect("constant regex"))
}

fn self_referential_regex() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(r"(?i)^(?:(?:the|this)\s+(?:(?:memory|refined|unifying|underlying)\s+)?concept\b|(?:the|this)\s+memory\s+(?:phenomenon|entry|record)\b|(?:this|it)\s+refers\s+to\b|the\s+term\s+(?:refers|describes|is)\b|the\s+idea\s+(?:that|of)\b|the\s+phrase\b)").expect("constant regex")
    })
}

fn boilerplate_regex() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"(?i)^(?:(?:this|the)\s+(?:concept|memory|belief|refined concept|insight|pattern)\b|concept\s+[ab]\b)").expect("constant regex"))
}

fn name_opener_regex() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(r"^\s*(?::|—|–|-|\bis\b|\brefers\b|\bdescribes\b|\bmeans\b)")
            .expect("constant regex")
    })
}

fn number_regex() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"\b\d[\d.,%:/-]*\b").expect("constant regex"))
}

fn quote_regex() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r#"\"([^\"”]{3,})\"|“([^\"”]{3,})”"#).expect("constant regex"))
}

fn first_person_regex() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(r"\b(?:I|I'm|I've|I'd|I'll|my|me|mine|myself)\b").expect("constant regex")
    })
}

fn capitalized_regex() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"\b[A-Z][A-Za-z0-9#.-]{2,}\b").expect("constant regex"))
}

fn hedge_regex() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"(?i)\b(?:may|might|could)\b|\bbut only\b|\bin\s+(?:some|certain|specific|particular)\s+(?:cases|contexts|configurations|situations|environments|circumstances)\b|\bdepending on\b|\bnot necessarily\b|\bpotentially\b|\backnowledg(?:e|es|ed|ing)\b|\b(?:it is|it's)\s+(?:important|worth)\s+(?:to note|noting)\b|\bgenerally\b|\btypically\b|\bin general\b|\bwhere\s+(?:supported|applicable|available)\b|\bsubject to\b|\bto some extent\b|\bas a construct\b").expect("constant regex"))
}
