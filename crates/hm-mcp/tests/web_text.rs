#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_mcp::tools::webtext::{MAXIMUM_EXTRACTED_BYTES, extract_text, is_textual};

const PAGE: &str = concat!(
    "<html><head><title>Ledger &amp; Recall</title>",
    "<style>.body { content: \"poison\"; }</style></head>",
    "<body><!-- poison comment --><p>First <b>visible</b> paragraph.</p>",
    "<div><span>Second</span> line</div>",
    "<script>var marker = \"poison\";</script></body></html>"
);

#[test]
fn html_extraction_drops_scripts_styles_comments_and_tags() {
    let extracted = extract_text("text/html", PAGE.as_bytes()).expect("extraction");
    assert_eq!(extracted.media_type, "text/html");
    assert_eq!(extracted.title.as_deref(), Some("Ledger & Recall"));
    assert!(!extracted.text.contains("poison"));
    assert!(!extracted.text.contains('<'));
    assert!(!extracted.text.contains('>'));
    assert_eq!(extracted.text, "First visible paragraph.\nSecond line");
    let repeated = extract_text("text/html", PAGE.as_bytes()).expect("extraction");
    assert_eq!(repeated, extracted);

    let unclosed = extract_text("text/html", b"<p>kept</p><script>poison").expect("extraction");
    assert_eq!(unclosed.text, "kept");
}

#[test]
fn entities_and_whitespace_are_normalised() {
    let source = "<p>&amp; &lt; &gt; &quot; &#39; &nbsp; &#65;&#x42; &zzz;</p><p>a  \t\n  b</p>";
    let extracted = extract_text("application/xhtml+xml", source.as_bytes()).expect("extraction");
    assert_eq!(extracted.media_type, "application/xhtml+xml");
    assert_eq!(extracted.title, None);
    assert_eq!(
        extracted.text, "& < > \" ' \u{a0} AB &zzz;\na b",
        "entities decode, unknown entities stay verbatim and whitespace collapses"
    );
}

#[test]
fn plain_markdown_csv_and_json_pass_through() {
    let source = "  line one\n\nline  two\t\n ";
    for media_type in [
        "text/plain",
        "text/markdown",
        "text/csv",
        "application/json",
    ] {
        let extracted = extract_text(media_type, source.as_bytes()).expect("extraction");
        assert_eq!(extracted.media_type, media_type);
        assert_eq!(extracted.title, None);
        assert_eq!(extracted.text, "line one\n\nline  two");
    }
}

#[test]
fn unsupported_invalid_and_oversize_payloads_are_refused() {
    let unsupported = extract_text("image/png", b"\x89PNG\r\n\x1a\n").expect_err("refusal");
    assert_eq!(unsupported.code, ErrorCode::OperationUnavailable);

    let invalid = extract_text("text/plain", &[b'a', 0xFF, 0xFE, b'b']).expect_err("refusal");
    assert_eq!(invalid.code, ErrorCode::SchemaInvalid);

    let oversize = vec![b'a'; MAXIMUM_EXTRACTED_BYTES + 1];
    let refused = extract_text("text/plain", &oversize).expect_err("refusal");
    assert_eq!(refused.code, ErrorCode::CapacityExceeded);

    let accepted = vec![b'a'; MAXIMUM_EXTRACTED_BYTES];
    let extracted = extract_text("text/plain", &accepted).expect("extraction");
    assert_eq!(extracted.text.len(), MAXIMUM_EXTRACTED_BYTES);
}

#[test]
fn is_textual_matches_the_declared_set() {
    for media_type in [
        "text/html",
        "application/xhtml+xml",
        "text/plain",
        "text/markdown",
        "text/csv",
        "application/json",
    ] {
        assert!(is_textual(media_type), "{media_type} is textual");
    }
    for media_type in [
        "image/png",
        "audio/mpeg",
        "application/pdf",
        "text/html; charset=utf-8",
    ] {
        assert!(!is_textual(media_type), "{media_type} is not textual");
    }
}
