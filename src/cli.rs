use std::env;
use std::path::PathBuf;

use crate::error::{CliError, ErrorKind};

pub const DEFAULT_ENDPOINT: &str = "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits";
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub enum Action {
    Run(Config),
    Help,
    Version,
}

pub struct Config {
    pub json: bool,
    pub utc: bool,
    pub auth_file: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
    pub endpoint: String,
    pub timeout_secs: u64,
}

pub fn parse() -> Result<Action, CliError> {
    parse_from(env::args().skip(1))
}

fn parse_from<I>(args: I) -> Result<Action, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut json = false;
    let mut utc = env::var("CODEX_RESET_STATUS_UTC").as_deref() == Ok("1");
    let mut auth_file = env::var_os("CODEX_RESET_STATUS_AUTH_FILE").map(PathBuf::from);
    let mut fixture = env::var_os("CODEX_RESET_STATUS_FIXTURE").map(PathBuf::from);
    let mut endpoint =
        env::var("CODEX_RESET_STATUS_ENDPOINT").unwrap_or_else(|_| DEFAULT_ENDPOINT.to_owned());
    let mut timeout_secs = env::var("CODEX_RESET_STATUS_TIMEOUT_SECS")
        .ok()
        .map(|value| parse_timeout(&value))
        .transpose()?
        .unwrap_or(DEFAULT_TIMEOUT_SECS);

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--utc" => utc = true,
            "--auth-file" => {
                auth_file = Some(PathBuf::from(require_value(&mut args, "--auth-file")?));
            }
            "--fixture" => {
                fixture = Some(PathBuf::from(require_value(&mut args, "--fixture")?));
            }
            "--endpoint" => endpoint = require_value(&mut args, "--endpoint")?,
            "--timeout" => {
                timeout_secs = parse_timeout(&require_value(&mut args, "--timeout")?)?;
            }
            "-h" | "--help" => return Ok(Action::Help),
            "-V" | "--version" => return Ok(Action::Version),
            "--" => {
                if let Some(positional) = args.next() {
                    return Err(usage(format!(
                        "unexpected positional argument after --: {positional}"
                    )));
                }
            }
            _ => return Err(usage(format!("unknown option: {arg}"))),
        }
    }

    if fixture.is_none() {
        validate_endpoint(&endpoint)?;
    }

    Ok(Action::Run(Config {
        json,
        utc,
        auth_file,
        fixture,
        endpoint,
        timeout_secs,
    }))
}

fn require_value<I>(args: &mut I, flag: &str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| usage(format!("{flag} requires a value")))
}

fn parse_timeout(value: &str) -> Result<u64, CliError> {
    let seconds = value
        .parse::<u64>()
        .map_err(|_| usage("--timeout must be an integer from 1 to 120"))?;
    if !(1..=120).contains(&seconds) {
        return Err(usage("--timeout must be an integer from 1 to 120"));
    }
    Ok(seconds)
}

pub fn insecure_loopback_allowed() -> bool {
    env::var("CODEX_RESET_STATUS_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1")
}

fn validate_endpoint(endpoint: &str) -> Result<(), CliError> {
    if endpoint == DEFAULT_ENDPOINT {
        return Ok(());
    }

    if is_loopback_endpoint(endpoint) && insecure_loopback_allowed() {
        return Ok(());
    }

    Err(usage(
        "--endpoint may select only the built-in ChatGPT endpoint or an opted-in loopback test",
    ))
}

pub fn is_loopback_endpoint(endpoint: &str) -> bool {
    let Ok(uri) = endpoint.parse::<ureq::http::Uri>() else {
        return false;
    };
    if uri.scheme_str() != Some("http") {
        return false;
    }
    let Some(authority) = uri.authority() else {
        return false;
    };
    if authority.as_str().contains('@') {
        return false;
    }
    matches!(uri.host(), Some("127.0.0.1" | "::1" | "[::1]"))
}

fn usage(message: impl Into<String>) -> CliError {
    CliError::new(ErrorKind::Usage, message)
}

pub fn help() -> &'static str {
    "Usage: codex-reset-status [OPTIONS]\n\
\n\
Show available ChatGPT/Codex reset credits and their expiry times.\n\
\n\
Options:\n\
  --json              Print codex-reset-status/v1 JSON\n\
  --utc               Show times in UTC instead of the local system zone\n\
  --auth-file <PATH>  Override the Codex auth.json path\n\
  --fixture <PATH>    Read endpoint JSON from a file; performs no network request\n\
  --endpoint <URL>    Select the built-in endpoint or an opted-in loopback test\n\
  --timeout <SECONDS> Request timeout from 1 to 120 (default: 30)\n\
  -V, --version       Print version\n\
  -h, --help          Print help\n\
\n\
Environment:\n\
  CODEX_HOME, CODEX_RESET_STATUS_AUTH_FILE, CODEX_RESET_STATUS_FIXTURE,\n\
  CODEX_RESET_STATUS_UTC, TZ,\n\
  CODEX_RESET_STATUS_ENDPOINT, CODEX_RESET_STATUS_TIMEOUT_SECS,\n\
  HTTPS_PROXY, ALL_PROXY, NO_PROXY\n\
\n\
Uses an undocumented ChatGPT backend endpoint that can change without notice.\n\
Unofficial project; not affiliated with or endorsed by OpenAI."
}

#[cfg(test)]
mod tests {
    use super::{Action, is_loopback_endpoint, parse_from};

    #[test]
    fn rejects_unknown_options() {
        let error = parse_from(["--wat".to_owned()]).err().expect("must fail");
        assert_eq!(error.kind.exit_code(), 2);
    }

    #[test]
    fn accepts_json_fixture() {
        let action = parse_from([
            "--json".to_owned(),
            "--fixture".to_owned(),
            "fixture.json".to_owned(),
        ])
        .expect("valid arguments");
        assert!(matches!(action, Action::Run(_)));
    }

    #[test]
    fn rejects_timeouts_outside_the_documented_range() {
        for value in ["0", "121", "abc", "-1", ""] {
            let error = parse_from(["--timeout".to_owned(), value.to_owned()])
                .err()
                .unwrap_or_else(|| panic!("--timeout {value} must be rejected"));
            assert_eq!(error.kind.exit_code(), 2);
        }
        assert!(parse_from(["--timeout".to_owned(), "120".to_owned()]).is_ok());
    }

    #[test]
    fn loopback_parser_rejects_userinfo_and_suffix_hosts() {
        assert!(is_loopback_endpoint("http://127.0.0.1:3000/test"));
        assert!(is_loopback_endpoint("http://[::1]:3000/test"));
        assert!(!is_loopback_endpoint(
            "http://127.0.0.1:3000@attacker.example/test"
        ));
        assert!(!is_loopback_endpoint("http://127.0.0.1.example/test"));
    }
}
