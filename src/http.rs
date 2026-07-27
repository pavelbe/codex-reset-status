use std::time::Duration;

use serde_json::Value;
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig};

use crate::auth::Auth;
use crate::cli::is_loopback_endpoint;
use crate::error::{CliError, ErrorKind};

const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

pub fn fetch(endpoint: &str, timeout_secs: u64, auth: &Auth) -> Result<Value, CliError> {
    let tls = TlsConfig::builder()
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    let loopback = is_loopback_endpoint(endpoint);
    let mut builder = Agent::config_builder()
        .tls_config(tls)
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(timeout_secs)))
        .user_agent(concat!(
            "codex-reset-status/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/pavelbe/codex-reset-status)"
        ));
    if loopback {
        builder = builder.https_only(false).proxy(None);
    } else {
        builder = builder.https_only(true);
    }
    let config = builder.build();
    let agent = Agent::new_with_config(config);

    let authorization = format!("Bearer {}", auth.token.expose_for_header());
    let mut request = agent
        .get(endpoint)
        .header("Accept", "application/json")
        .header("Authorization", authorization)
        .header("OpenAI-Beta", "codex-1")
        .header("originator", "Codex Desktop");
    if let Some(account_id) = &auth.account_id {
        request = request.header("ChatGPT-Account-ID", account_id);
    }

    let mut response = request.call().map_err(|error| {
        let message = auth.token.redact(&error.to_string());
        CliError::new(ErrorKind::Transport, format!("request failed: {message}"))
    })?;
    let status = response.status().as_u16();
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES + 1)
        .read_to_vec()
        .map_err(|error| {
            CliError::new(
                ErrorKind::Response,
                format!("cannot read bounded response body: {error}"),
            )
        })?;
    if body.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(CliError::new(
            ErrorKind::Response,
            format!("response exceeds {MAX_RESPONSE_BYTES} bytes"),
        ));
    }

    match status {
        200..=299 => serde_json::from_slice(&body).map_err(|error| {
            CliError::new(
                ErrorKind::Response,
                format!("endpoint returned invalid JSON: {error}"),
            )
        }),
        401 | 403 => Err(CliError::new(
            ErrorKind::Unauthorized,
            format!("endpoint rejected Codex authentication with HTTP {status}; sign in again"),
        )),
        429 => {
            let suffix = retry_after
                .map(|value| format!("; Retry-After: {value}"))
                .unwrap_or_default();
            Err(CliError::new(
                ErrorKind::Response,
                format!("endpoint rate-limited the request with HTTP 429{suffix}"),
            ))
        }
        _ => Err(CliError::new(
            ErrorKind::Response,
            format!("endpoint returned unexpected HTTP {status}"),
        )),
    }
}
