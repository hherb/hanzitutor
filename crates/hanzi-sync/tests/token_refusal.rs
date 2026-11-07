//! A revoked or expired Dropbox sign-in, through the real HTTP client.
//!
//! `tests/dropbox.rs` drives everything above [`Http`] with an in-memory Dropbox,
//! and so cannot see what [`UreqHttp`] makes of a status code — which is where this
//! failure was lost. Dropbox's token endpoint answers a revoked refresh token with
//! `400 {"error": "invalid_grant", ...}`; `ureq` turns any 4xx into an error before
//! the OAuth code sees the body, and the 400 branch used to report it as a request
//! this program got wrong, with the raw JSON attached, on every sync for ever.
//!
//! So this answers one request from a socket on the loopback interface — no
//! network, no account — and checks what the client hands back.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use hanzi_sync::{Http, SyncError, UreqHttp};

/// Answer exactly one HTTP request with `status` and a JSON `body`, and return the
/// URL to send it to.
fn answer_once(status: &str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/oauth2/token", listener.local_addr().unwrap());
    let status = status.to_string();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        // Read the request head, then the body it announces, so the client is not
        // answered before it has finished sending.
        let mut length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
            if line == "\r\n" || line.is_empty() {
                break;
            }
        }
        let mut request_body = vec![0; length];
        reader.read_exact(&mut request_body).unwrap();
        let mut stream = reader.into_inner();
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    url
}

#[test]
fn a_revoked_refresh_token_is_reported_as_a_sign_in_to_renew() {
    let url = answer_once(
        "400 Bad Request",
        r#"{"error": "invalid_grant", "error_description": "refresh token is invalid or revoked"}"#,
    );
    let error = UreqHttp::new()
        .form(
            &url,
            "grant_type=refresh_token&refresh_token=gone&client_id=key",
        )
        .unwrap_err();
    assert_eq!(
        error,
        SyncError::Unauthorized,
        "the app tells the learner to connect again only for this variant"
    );
}

#[test]
fn a_400_that_is_this_programs_mistake_still_says_so() {
    let url = answer_once(
        "400 Bad Request",
        r#"{"error": "unsupported_grant_type", "error_description": "no such grant"}"#,
    );
    let error = UreqHttp::new()
        .form(&url, "grant_type=nonsense")
        .unwrap_err();
    assert!(
        matches!(&error, SyncError::Io(why) if why.contains("(400)")),
        "a malformed request is not a sign-in problem: {error:?}"
    );
}
