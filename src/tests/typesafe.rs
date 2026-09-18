use super::*;
use std::{io::Write, net::TcpListener, thread};

fn response_body() -> String {
    json!({
        "model": "jev-test",
        "answers": {
            "route": { "type": "choice", "choice": "implementation", "confidence": 0.9,
                "probabilities": { "implementation": 0.9, "planning": 0.1 }
            },
            "context_missing": { "type": "noul", "noul": 0.05 }
        },
        "usage": { "input_tokens": 12, "output_tokens": 8 }
    })
    .to_string()
}

fn server(status: &str, body: String, delay: Duration) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1/systemone", listener.local_addr().unwrap());
    let status = status.to_string();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        let headers = String::from_utf8(request.clone()).unwrap();
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")?
                    .parse()
                    .ok()
            })
            .unwrap_or(0);
        let mut payload = vec![0; length];
        stream.read_exact(&mut payload).unwrap();
        request.extend(payload);
        thread::sleep(delay);
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        String::from_utf8(request).unwrap()
    });
    (endpoint, worker)
}

#[test]
fn http_boundary_sends_json_and_decodes_usage() {
    let (url, worker) = server("200 OK", response_body(), Duration::ZERO);
    let body = request_body(
        json!({ "task": "fix typo", "context": "", "enabled_capabilities": ["implementation"] }),
        json!({ "route": { "type": "choice", "instructions": "Choose a capability", "criteria": { "implementation": { "what": "Write the change" } } } }),
    );
    let answer = send(&url, "test-key", body.clone(), Duration::from_secs(2)).unwrap();
    assert_eq!(answer.answers.route.choice, "implementation");
    assert_eq!(answer.usage.input_tokens, 12);
    let request = worker.join().unwrap();
    assert!(request.starts_with("POST /v1/systemone HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-key")
    );
    let (_, payload) = request.split_once("\r\n\r\n").unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(payload).unwrap(),
        json!({
            "model": "jev-1.13.0",
            "state": {"task": "fix typo", "context": "", "enabled_capabilities": ["implementation"]},
            "questions": {"route": {
                "type": "choice", "instructions": "Choose a capability",
                "criteria": {"implementation": {"what": "Write the change"}}
            }}
        })
    );
}

#[test]
fn service_errors_and_malformed_answers_return_errors_without_response_body_leaks() {
    for (status, body) in [
        ("401 Unauthorized", "do-not-echo-this-body".into()),
        ("302 Found", "redirect".into()),
        ("200 OK", "{}".into()),
        (
            "200 OK",
            r#"{"model":"do-not-echo-this-body","answers":"do-not-echo-this-body"}"#.into(),
        ),
        ("200 OK", "x".repeat(RESPONSE_LIMIT as usize + 1)),
    ] {
        let (url, worker) = server(status, body, Duration::ZERO);
        let error = send(&url, "test-key", json!({}), Duration::from_secs(2))
            .unwrap_err()
            .to_string();
        assert!(!error.contains("do-not-echo-this-body"));
        assert!(!error.contains("test-key"));
        worker.join().unwrap();
    }
    let mut body: Value = serde_json::from_str(&response_body()).unwrap();
    body["answers"]["route"]
        .as_object_mut()
        .unwrap()
        .remove("confidence");
    let (url, worker) = server("200 OK", body.to_string(), Duration::ZERO);
    assert!(send(&url, "test-key", json!({}), Duration::from_secs(2)).is_err());
    worker.join().unwrap();
}

#[test]
fn stalled_request_is_bounded_by_deadline() {
    let (url, worker) = server("200 OK", response_body(), Duration::from_millis(200));
    assert!(send(&url, "test-key", json!({}), Duration::from_millis(30)).is_err());
    worker.join().unwrap();
}
