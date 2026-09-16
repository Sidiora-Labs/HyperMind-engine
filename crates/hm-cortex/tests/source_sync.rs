#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_cortex::connectors::sync::{
    MAXIMUM_SOURCE_IDS, MAXIMUM_SOURCE_PAGES, RecordedSourceTransport, SourceRequest,
    SourceResponse, SourceRevisionListing, list_revisions,
};
use std::collections::BTreeMap;
use std::fmt::Write as _;

const BASE_URL: &str = "https://source-index.internal/v1";
const ACCESS_TOKEN: &str = "connector-access-token";
const FIRST_PAGE_URL: &str = "https://source-index.internal/v1/sources";
const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn request(url: &str) -> SourceRequest {
    SourceRequest {
        url: url.to_owned(),
        headers: BTreeMap::from([
            ("accept".to_owned(), "application/json".to_owned()),
            (
                "authorization".to_owned(),
                "Bearer connector-access-token".to_owned(),
            ),
        ]),
    }
}

fn response(body: &str) -> SourceResponse {
    SourceResponse {
        status: 200,
        body: body.as_bytes().to_vec(),
    }
}

fn single_page(body: &str) -> RecordedSourceTransport {
    RecordedSourceTransport::new(vec![(request(FIRST_PAGE_URL), response(body))])
}

#[test]
fn source_index_listing_round_trips() {
    let transport = single_page(
        "{\"sources\":[\
         {\"source_id\":\"workspace/notes\",\"revision\":\"r-17\",\"content_digest\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"},\
         {\"source_id\":\"repository/engine\",\"revision\":\"9f2c\",\"content_digest\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"}\
         ],\"next_cursor\":null}",
    );
    let listings = list_revisions(&transport, BASE_URL, ACCESS_TOKEN).expect("listing succeeds");
    assert_eq!(
        listings,
        vec![
            SourceRevisionListing {
                source_id: "workspace/notes".to_owned(),
                revision: b"r-17".to_vec(),
                content_digest: [0xaa; 32],
            },
            SourceRevisionListing {
                source_id: "repository/engine".to_owned(),
                revision: b"9f2c".to_vec(),
                content_digest: [0xbb; 32],
            },
        ]
    );
    assert_eq!(transport.remaining(), 0);
}

#[test]
fn source_index_paging_follows_the_cursor() {
    let transport = RecordedSourceTransport::new(vec![
        (
            request(FIRST_PAGE_URL),
            response(
                "{\"sources\":[\
                 {\"source_id\":\"workspace/notes\",\"revision\":\"r-17\",\"content_digest\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\
                 ],\"next_cursor\":\"cursor-two\"}",
            ),
        ),
        (
            request("https://source-index.internal/v1/sources?cursor=cursor-two"),
            response(
                "{\"sources\":[\
                 {\"source_id\":\"repository/engine\",\"revision\":\"9f2c\",\"content_digest\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"}\
                 ]}",
            ),
        ),
    ]);
    let listings = list_revisions(&transport, BASE_URL, ACCESS_TOKEN).expect("paged listing");
    assert_eq!(
        listings
            .iter()
            .map(|listing| listing.source_id.as_str())
            .collect::<Vec<_>>(),
        vec!["workspace/notes", "repository/engine"]
    );
    assert_eq!(listings[1].content_digest, [0xbb; 32]);
    assert_eq!(transport.remaining(), 0);
}

#[test]
fn source_index_is_bounded() {
    let mut exchanges = Vec::with_capacity(MAXIMUM_SOURCE_PAGES);
    for page in 1..=MAXIMUM_SOURCE_PAGES {
        let url = if page == 1 {
            FIRST_PAGE_URL.to_owned()
        } else {
            format!("https://source-index.internal/v1/sources?cursor=cursor-{page}")
        };
        let body = format!("{{\"sources\":[],\"next_cursor\":\"cursor-{}\"}}", page + 1);
        exchanges.push((request(&url), response(&body)));
    }
    let paged = RecordedSourceTransport::new(exchanges);
    assert_eq!(
        list_revisions(&paged, BASE_URL, ACCESS_TOKEN)
            .expect_err("page walk is bounded")
            .code,
        ErrorCode::CapacityExceeded
    );
    assert_eq!(paged.remaining(), 0);

    let accepted = single_page(&listing_body(MAXIMUM_SOURCE_IDS));
    assert_eq!(
        list_revisions(&accepted, BASE_URL, ACCESS_TOKEN)
            .expect("the largest admissible listing")
            .len(),
        MAXIMUM_SOURCE_IDS
    );

    let refused = single_page(&listing_body(MAXIMUM_SOURCE_IDS + 1));
    assert_eq!(
        list_revisions(&refused, BASE_URL, ACCESS_TOKEN)
            .expect_err("listing size is bounded")
            .code,
        ErrorCode::CapacityExceeded
    );
}

#[test]
fn source_index_fails_closed_on_shape() {
    let malformed = [
        "{}".to_owned(),
        "[]".to_owned(),
        "this is not json".to_owned(),
        "{\"sources\":{}}".to_owned(),
        "{\"sources\":[\"workspace/notes\"]}".to_owned(),
        format!("{{\"sources\":[{{\"source_id\":\"\",\"revision\":\"r-1\",\"content_digest\":\"{DIGEST_A}\"}}]}}"),
        format!("{{\"sources\":[{{\"source_id\":\"workspace/notes\",\"revision\":\"\",\"content_digest\":\"{DIGEST_A}\"}}]}}"),
        "{\"sources\":[{\"source_id\":\"workspace/notes\",\"revision\":\"r-1\"}]}".to_owned(),
        "{\"sources\":[{\"source_id\":\"workspace/notes\",\"revision\":\"r-1\",\"content_digest\":\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"}]}".to_owned(),
        "{\"sources\":[{\"source_id\":\"workspace/notes\",\"revision\":\"r-1\",\"content_digest\":\"aaaa\"}]}".to_owned(),
        format!("{{\"sources\":[{{\"source_id\":\"workspace/notes\",\"revision\":\"r-1\",\"content_digest\":\"{DIGEST_B}\"}}],\"next_cursor\":7}}"),
        "{\"sources\":[],\"next_cursor\":\"\"}".to_owned(),
        "{\"sources\":[],\"next_cursor\":\"cursor two\"}".to_owned(),
    ];
    for body in &malformed {
        let transport = single_page(body);
        assert_eq!(
            list_revisions(&transport, BASE_URL, ACCESS_TOKEN)
                .expect_err("malformed source index body")
                .code,
            ErrorCode::SchemaInvalid,
            "body {body} was admitted"
        );
    }

    let repeated = RecordedSourceTransport::new(vec![
        (
            request(FIRST_PAGE_URL),
            response("{\"sources\":[],\"next_cursor\":\"cursor-two\"}"),
        ),
        (
            request("https://source-index.internal/v1/sources?cursor=cursor-two"),
            response("{\"sources\":[],\"next_cursor\":\"cursor-two\"}"),
        ),
    ]);
    assert_eq!(
        list_revisions(&repeated, BASE_URL, ACCESS_TOKEN)
            .expect_err("a repeated cursor is a loop")
            .code,
        ErrorCode::SchemaInvalid
    );
}

#[test]
fn source_index_rejects_error_status() {
    let transport = RecordedSourceTransport::new(vec![(
        request(FIRST_PAGE_URL),
        SourceResponse {
            status: 500,
            body: b"{\"sources\":[]}".to_vec(),
        },
    )]);
    assert_eq!(
        list_revisions(&transport, BASE_URL, ACCESS_TOKEN)
            .expect_err("a non-2xx page is never a partial success")
            .code,
        ErrorCode::BackendUnavailable
    );
    assert_eq!(transport.remaining(), 0);
}

#[test]
fn recorded_transport_pins_the_request_shape() {
    let mut without_accept = request(FIRST_PAGE_URL);
    without_accept.headers.remove("accept");
    let headers = RecordedSourceTransport::new(vec![(
        without_accept,
        response("{\"sources\":[],\"next_cursor\":null}"),
    )]);
    assert_eq!(
        list_revisions(&headers, BASE_URL, ACCESS_TOKEN)
            .expect_err("a differing header map is not the recorded request")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(headers.remaining(), 0);

    let elsewhere = RecordedSourceTransport::new(vec![(
        request("https://source-index.internal/v1/revisions"),
        response("{\"sources\":[],\"next_cursor\":null}"),
    )]);
    assert_eq!(
        list_revisions(&elsewhere, BASE_URL, ACCESS_TOKEN)
            .expect_err("a differing url is not the recorded request")
            .code,
        ErrorCode::InvalidArgument
    );
}

#[test]
fn source_index_rejects_empty_configuration() {
    let transport = single_page("{\"sources\":[],\"next_cursor\":null}");
    assert_eq!(
        list_revisions(&transport, "", ACCESS_TOKEN)
            .expect_err("an empty base url")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        list_revisions(&transport, BASE_URL, "")
            .expect_err("an empty access token")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(transport.remaining(), 1);
}

fn listing_body(count: usize) -> String {
    let mut body = String::from("{\"sources\":[");
    for index in 0..count {
        if index > 0 {
            body.push(',');
        }
        let _ = write!(
            body,
            "{{\"source_id\":\"workspace/note-{index}\",\"revision\":\"r-{index}\",\"content_digest\":\"{DIGEST_A}\"}}"
        );
    }
    body.push_str("],\"next_cursor\":null}");
    body
}
