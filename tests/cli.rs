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

fn write_temp_auth(token: &str, account_id: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "codex-reset-status-auth-{}-{unique}.json",
        std::process::id()
    ));
    fs::write(
        &path,
        format!(r#"{{"tokens":{{"access_token":"{token}","account_id":"{account_id}"}}}}"#),
    )
    .unwrap();
    path
}

/// Serves one HTTP response and returns the raw request bytes it received.
fn serve_once(
    listener: TcpListener,
    status_line: &'static str,
    body: String,
) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 8192];
        let size = stream.read(&mut request).unwrap();
        let _ = write!(
            stream,
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.flush();
        String::from_utf8_lossy(&request[..size]).into_owned()
    })
}

/// Points every proxy variable at a closed port so a request that honors the
/// environment proxy cannot succeed.
fn with_dead_proxy(command: &mut Command) -> &mut Command {
    for variable in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        command.env(variable, "http://127.0.0.1:1");
    }
    command.env_remove("NO_PROXY").env_remove("no_proxy")
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
    let server = serve_once(
        listener,
        "HTTP/1.1 401 Unauthorized",
        format!("echo: {token}"),
    );
    let auth_path = write_temp_auth(token, "test");

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

    let request = server.join().unwrap();
    fs::remove_file(auth_path).unwrap();
    assert!(
        request.contains(token),
        "server must observe the bearer token"
    );
    assert_eq!(output.status.code(), Some(5));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains(token));
    assert!(stderr.contains("HTTP 401"));
}

#[test]
fn bypasses_the_environment_proxy_for_a_loopback_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = serve_once(
        listener,
        "HTTP/1.1 200 OK",
        r#"{"available_count":0,"credits":[]}"#.to_owned(),
    );
    let auth_path = write_temp_auth("proxy-bypass-token", "test");

    let mut command = Command::new(binary());
    command
        .args([
            "--auth-file",
            auth_path.to_str().unwrap(),
            "--endpoint",
            &endpoint,
            "--timeout",
            "5",
        ])
        .env("CODEX_RESET_STATUS_ALLOW_INSECURE_LOOPBACK", "1");
    let output = with_dead_proxy(&mut command).output().expect("run binary");

    let request = server.join().unwrap();
    fs::remove_file(auth_path).unwrap();
    assert!(
        request.starts_with("GET /"),
        "loopback request must reach the listener directly, got: {request}"
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("No reset credits found.")
    );
}

#[test]
fn rejects_a_response_larger_than_the_body_cap() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let padding = "x".repeat(1024 * 1024 + 64);
    let server = serve_once(
        listener,
        "HTTP/1.1 200 OK",
        format!(r#"{{"credits":[],"padding":"{padding}"}}"#),
    );
    let auth_path = write_temp_auth("cap-token", "test");

    let mut command = Command::new(binary());
    command
        .args([
            "--auth-file",
            auth_path.to_str().unwrap(),
            "--endpoint",
            &endpoint,
            "--timeout",
            "15",
        ])
        .env("CODEX_RESET_STATUS_ALLOW_INSECURE_LOOPBACK", "1");
    let output = with_dead_proxy(&mut command).output().expect("run binary");

    let _ = server.join();
    fs::remove_file(auth_path).unwrap();
    assert_eq!(output.status.code(), Some(6));
    // The bounded reader refuses the oversized body; the explicit length check in
    // http.rs stays as a second line of defence, so accept either message.
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("larger than request limit: 1048577") || stderr.contains("1048576 bytes"),
        "expected a body cap error, got: {stderr}"
    );
}

#[test]
fn rejects_a_control_character_account_id_before_any_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let auth_path = write_temp_auth("header-token", r"test\r\nX-Injected: 1");

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
        .output()
        .expect("run binary");

    fs::remove_file(auth_path).unwrap();
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("account_id"), "stderr: {stderr}");
    listener
        .set_nonblocking(true)
        .expect("listener must stay usable");
    assert!(
        listener.accept().is_err(),
        "no request may be sent when the account id is rejected"
    );
}

#[test]
fn renders_utc_on_request_and_names_the_zone() {
    let output = Command::new(binary())
        .args(["--utc", "--fixture", &fixture("ok.json")])
        .env("TZ", "Asia/Tokyo")
        .output()
        .expect("run binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("(UTC)"), "stdout: {stdout}");
    assert!(!stdout.contains("JST"), "stdout: {stdout}");
}

#[test]
fn renders_the_host_zone_by_default() {
    let output = Command::new(binary())
        .args(["--fixture", &fixture("ok.json")])
        .env("TZ", "Asia/Tokyo")
        .output()
        .expect("run binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("(Asia/Tokyo)"), "stdout: {stdout}");
    assert!(stdout.contains("JST"), "stdout: {stdout}");
}

#[test]
fn warns_instead_of_silently_falling_back_when_the_zone_is_unusable() {
    let output = Command::new(binary())
        .args(["--fixture", &fixture("ok.json")])
        .env("TZ", "Invalid/Zone")
        .output()
        .expect("run binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("cannot determine the system time zone"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("(UTC)"), "stdout: {stdout}");
}

#[test]
fn reports_the_deadline_for_the_first_expiring_reset() {
    let output = Command::new(binary())
        .args(["--json", "--fixture", &fixture("ok.json")])
        .output()
        .expect("run binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"nextExpirySeconds\""), "stdout: {stdout}");
    assert!(stdout.contains("\"timeZone\""), "stdout: {stdout}");
    assert!(stdout.contains("\"checkedAtLocal\""), "stdout: {stdout}");
}
