use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_codex-reset-status")
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn renders_allowlisted_json() {
    let output = Command::new(binary())
        .args(["--json", "--fixture", &fixture("ok.json")])
        .output()
        .expect("run binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"schema\": \"codex-reset-status/v1\""));
    assert!(stdout.contains("\"source\": \"fixture\""));
    assert!(!stdout.contains("must-not-appear"));
    assert!(!stdout.contains("account_email"));
}

#[test]
fn accepts_an_explicit_empty_credit_list() {
    let output = Command::new(binary())
        .args(["--fixture", &fixture("empty.json")])
        .output()
        .expect("run binary");
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("No reset credits found.")
    );
}

#[test]
fn fails_closed_on_schema_drift() {
    let output = Command::new(binary())
        .args(["--fixture", &fixture("drift-no-credits.json")])
        .output()
        .expect("run binary");
    assert_eq!(output.status.code(), Some(6));
}

#[test]
fn rejects_plain_http_without_loopback_opt_in() {
    let output = Command::new(binary())
        .args(["--endpoint", "http://example.com"])
        .output()
        .expect("run binary");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn rejects_an_arbitrary_https_endpoint() {
    let output = Command::new(binary())
        .args(["--endpoint", "https://example.com/collect"])
        .output()
        .expect("run binary");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn rejects_loopback_userinfo_exfiltration() {
    let output = Command::new(binary())
        .args(["--endpoint", "http://127.0.0.1:1@attacker.example/collect"])
        .env("CODEX_RESET_STATUS_ALLOW_INSECURE_LOOPBACK", "1")
        .output()
        .expect("run binary");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn redacts_a_token_echoed_by_an_unauthorized_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let token = "test-token-that-must-not-leak";
    let server_token = token.to_owned();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 8192];
        let size = stream.read(&mut request).unwrap();
        assert!(String::from_utf8_lossy(&request[..size]).contains(&server_token));
        let body = format!("echo: {server_token}");
        write!(
            stream,
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let auth_path = std::env::temp_dir().join(format!(
        "codex-reset-status-auth-{}-{unique}.json",
        std::process::id()
    ));
    fs::write(
        &auth_path,
        format!(r#"{{"tokens":{{"access_token":"{token}","account_id":"test"}}}}"#),
    )
    .unwrap();

    let output = Command::new(binary())
        .args([
            "--auth-file",
            auth_path.to_str().unwrap(),
            "--endpoint",
            &endpoint,
            "--timeout",
            "5",
        ])
        .env("CODEX_RESET_STATUS_ALLOW_INSECURE_LOOPBACK", "1")
        .env_remove("HTTPS_PROXY")
        .env_remove("https_proxy")
        .env_remove("ALL_PROXY")
        .env_remove("all_proxy")
        .output()
        .expect("run binary");

    server.join().unwrap();
    fs::remove_file(auth_path).unwrap();
    assert_eq!(output.status.code(), Some(5));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains(token));
    assert!(stderr.contains("HTTP 401"));
}
