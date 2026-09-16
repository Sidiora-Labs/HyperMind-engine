#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_mcp::tools::websource::{
    CrawlPolicy, FetchResponse, MAXIMUM_SOURCE_BYTES, RecordedFetchTransport, WebSourceRuntime,
    policy_from_env, validate_target,
};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

const WEB_SOURCE_KEYS: [&str; 6] = [
    "HM_WEB_SOURCE_HOSTS",
    "HM_WEB_SOURCE_MAX_BYTES",
    "HM_WEB_SOURCE_MAX_REDIRECTS",
    "HM_WEB_SOURCE_MIN_INTERVAL_MS",
    "HM_WEB_SOURCE_TIMEOUT_MS",
    "HM_WEB_SOURCE_CROSS_HOST_REDIRECT",
];

fn policy(hosts: &[&str]) -> CrawlPolicy {
    CrawlPolicy {
        allowed_hosts: hosts.iter().map(|host| (*host).to_owned()).collect(),
        maximum_bytes: 4096,
        maximum_redirects: 3,
        minimum_interval_ms: 0,
        request_timeout_ms: 1000,
        allow_cross_host_redirect: false,
    }
}

fn redirect(location: &str) -> FetchResponse {
    FetchResponse {
        status: 302,
        content_type: None,
        content_length: None,
        location: Some(location.to_owned()),
        body: Vec::new(),
    }
}

fn body(content_type: &str, bytes: &[u8]) -> FetchResponse {
    FetchResponse {
        status: 200,
        content_type: Some(content_type.to_owned()),
        content_length: None,
        location: None,
        body: bytes.to_vec(),
    }
}

fn runtime(
    policy: CrawlPolicy,
    responses: Vec<(String, FetchResponse)>,
) -> (WebSourceRuntime, Arc<RecordedFetchTransport>) {
    let transport = Arc::new(RecordedFetchTransport::new(responses));
    (WebSourceRuntime::new(policy, transport.clone()), transport)
}

fn recorded(url: &str, response: FetchResponse) -> (String, FetchResponse) {
    (url.to_owned(), response)
}

#[test]
fn crawl_policy_refuses_unlisted_hosts_schemes_and_credentials() {
    let policy = policy(&["example.com"]);
    let allowed = ["https://example.com/a", "http://example.com/a"];
    let refused = [
        "ftp://example.com/a",
        "file:///etc/passwd",
        "https://user:pw@example.com/a",
        "https://other.example.org/a",
        "https://example.com.evil.test/a",
    ];
    for url in allowed {
        assert!(
            validate_target(&policy, url).is_ok(),
            "{url} must be allowed"
        );
    }
    for url in refused {
        let error = validate_target(&policy, url).expect_err(url);
        assert_eq!(error.code, ErrorCode::CapabilityDenied, "{url}");
    }
}

#[test]
fn private_literal_targets_are_refused() {
    let urls = [
        "http://127.0.0.1/x",
        "http://10.1.2.3/x",
        "http://172.16.0.1/x",
        "http://192.168.1.1/x",
        "http://169.254.169.254/latest/meta-data",
        "http://100.64.0.1/x",
        "http://[::1]/x",
        "http://[fd00::1]/x",
    ];
    let policy = policy(&[
        "127.0.0.1",
        "10.1.2.3",
        "172.16.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "[::1]",
        "[fd00::1]",
    ]);
    for url in urls {
        let error = validate_target(&policy, url).expect_err(url);
        assert_eq!(error.code, ErrorCode::CapabilityDenied, "{url}");
    }
}

#[test]
fn redirect_chain_is_revalidated_at_every_hop_and_bounded() {
    let (following, transport) = runtime(
        policy(&["example.com"]),
        vec![
            recorded("https://example.com/a", redirect("https://example.com/b")),
            recorded("https://example.com/b", redirect("/c")),
            recorded("https://example.com/c", body("text/plain", b"final")),
        ],
    );
    let source = following.fetch("https://example.com/a").unwrap();
    assert_eq!(source.final_url, "https://example.com/c");
    assert_eq!(
        source.redirects,
        vec![
            "https://example.com/a".to_owned(),
            "https://example.com/b".to_owned()
        ]
    );
    assert_eq!(source.media_type, "text/plain");
    assert_eq!(transport.remaining(), 0);

    let mut crossing = policy(&["example.com", "127.0.0.1"]);
    crossing.allow_cross_host_redirect = true;
    let (into_private, transport) = runtime(
        crossing,
        vec![
            recorded("https://example.com/a", redirect("https://example.com/b")),
            recorded("https://example.com/b", redirect("http://127.0.0.1/x")),
        ],
    );
    let error = into_private
        .fetch("https://example.com/a")
        .expect_err("private hop");
    assert_eq!(error.code, ErrorCode::CapabilityDenied);
    assert_eq!(transport.remaining(), 0);

    let mut bounded = policy(&["example.com"]);
    bounded.maximum_redirects = 1;
    let (limited, transport) = runtime(
        bounded,
        vec![
            recorded("https://example.com/a", redirect("https://example.com/b")),
            recorded("https://example.com/b", redirect("https://example.com/c")),
        ],
    );
    let error = limited.fetch("https://example.com/a").expect_err("budget");
    assert_eq!(error.code, ErrorCode::CapacityExceeded);
    assert_eq!(transport.remaining(), 0);
}

#[test]
fn size_limits_refuse_declared_and_actual_overflow() {
    let mut bounded = policy(&["example.com"]);
    bounded.maximum_bytes = 8;

    let mut declared = body("text/plain", b"12345678");
    declared.content_length = Some(9);
    let (runtime_declared, transport) = runtime(
        bounded.clone(),
        vec![recorded("https://example.com/a", declared)],
    );
    let error = runtime_declared
        .fetch("https://example.com/a")
        .expect_err("declared overflow");
    assert_eq!(error.code, ErrorCode::CapacityExceeded);
    assert_eq!(transport.remaining(), 0);

    let (runtime_actual, transport) = runtime(
        bounded.clone(),
        vec![recorded(
            "https://example.com/a",
            body("text/plain", b"123456789"),
        )],
    );
    let error = runtime_actual
        .fetch("https://example.com/a")
        .expect_err("actual overflow");
    assert_eq!(error.code, ErrorCode::CapacityExceeded);
    assert_eq!(transport.remaining(), 0);

    let mut exact = body("text/plain", b"12345678");
    exact.content_length = Some(8);
    let (runtime_exact, transport) =
        runtime(bounded, vec![recorded("https://example.com/a", exact)]);
    let source = runtime_exact.fetch("https://example.com/a").unwrap();
    assert_eq!(source.bytes.len(), 8);
    assert_eq!(transport.remaining(), 0);
}

#[test]
fn fetch_records_mime_final_url_and_digest() {
    let bytes = b"<html><body>hypermind</body></html>";
    let (single, transport) = runtime(
        policy(&["example.com"]),
        vec![recorded(
            "https://example.com/page",
            body("text/html; charset=utf-8", bytes),
        )],
    );
    let source = single.fetch("https://example.com/page").unwrap();
    assert_eq!(source.requested_url, "https://example.com/page");
    assert_eq!(source.final_url, "https://example.com/page");
    assert_eq!(source.media_type, "text/html");
    assert_eq!(source.status, 200);
    assert!(source.redirects.is_empty());
    assert_eq!(source.bytes, bytes.to_vec());
    assert_eq!(source.digest, *blake3::hash(bytes).as_bytes());
    assert_eq!(transport.remaining(), 0);
}

#[test]
fn per_host_minimum_interval_is_enforced() {
    let mut paced = policy(&["example.com"]);
    paced.minimum_interval_ms = 50;
    let (throttled, transport) = runtime(
        paced,
        vec![
            recorded("https://example.com/a", body("text/plain", b"a")),
            recorded("https://example.com/b", body("text/plain", b"b")),
        ],
    );
    let started = Instant::now();
    throttled.fetch("https://example.com/a").unwrap();
    throttled.fetch("https://example.com/b").unwrap();
    assert!(started.elapsed().as_millis() >= 50);
    assert_eq!(transport.remaining(), 0);
}

fn run_env_child(case: &str, values: &[(&str, &str)]) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg("--ignored")
        .arg("--nocapture")
        .arg(case);
    for key in WEB_SOURCE_KEYS {
        command.env_remove(key);
    }
    for (key, value) in values {
        command.env(key, value);
    }
    let output = command.output().unwrap();
    let report = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "{case} failed: {report}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(report.contains("1 passed"), "{case} did not run: {report}");
}

#[test]
fn policy_from_env_parses_clamps_and_disables() {
    run_env_child("policy_from_env_child_absent", &[]);
    run_env_child(
        "policy_from_env_child_clamped",
        &[
            ("HM_WEB_SOURCE_HOSTS", "Example.com, docs.example.com"),
            ("HM_WEB_SOURCE_MAX_BYTES", "999999999"),
            ("HM_WEB_SOURCE_MAX_REDIRECTS", "40"),
            ("HM_WEB_SOURCE_CROSS_HOST_REDIRECT", "true"),
        ],
    );
    run_env_child(
        "policy_from_env_child_invalid",
        &[
            ("HM_WEB_SOURCE_HOSTS", "example.com"),
            ("HM_WEB_SOURCE_TIMEOUT_MS", "soon"),
        ],
    );
}

#[test]
#[ignore = "child process of policy_from_env_parses_clamps_and_disables"]
fn policy_from_env_child_absent() {
    assert!(policy_from_env().unwrap().is_none());
}

#[test]
#[ignore = "child process of policy_from_env_parses_clamps_and_disables"]
fn policy_from_env_child_clamped() {
    let policy = policy_from_env().unwrap().unwrap();
    assert_eq!(
        policy.allowed_hosts,
        vec!["example.com".to_owned(), "docs.example.com".to_owned()]
    );
    assert_eq!(policy.maximum_bytes, MAXIMUM_SOURCE_BYTES);
    assert_eq!(policy.maximum_redirects, 8);
    assert_eq!(policy.minimum_interval_ms, 1000);
    assert_eq!(policy.request_timeout_ms, 15_000);
    assert!(policy.allow_cross_host_redirect);
}

#[test]
#[ignore = "child process of policy_from_env_parses_clamps_and_disables"]
fn policy_from_env_child_invalid() {
    let error = policy_from_env().expect_err("unparsable timeout");
    assert_eq!(error.code, ErrorCode::InvalidArgument);
}
