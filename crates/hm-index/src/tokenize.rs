use tantivy::tokenizer::{
    LowerCaser, RemoveLongFilter, SimpleTokenizer, TextAnalyzer, TokenStream,
};

pub const MAXIMUM_TERM_BYTES: usize = 64;

#[must_use]
pub fn tokenize(text: &str) -> Vec<String> {
    let mut analyzer = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(MAXIMUM_TERM_BYTES))
        .filter(LowerCaser)
        .build();
    let mut terms = Vec::new();
    analyzer.token_stream(text).process(&mut |token| {
        if token.text.len() >= 2 {
            terms.push(token.text.clone());
        }
    });
    terms
}
