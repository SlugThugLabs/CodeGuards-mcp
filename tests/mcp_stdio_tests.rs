//! End-to-end MCP stdio handshake test.
//!
//! Spawns the real `codeguards-mcp serve` binary in a hermetic environment
//! (temp `SLUGTHUG_HOME` so nothing touches the live ~/.slugthug state) and
//! verifies the full JSON-RPC 2.0 lifecycle over stdio:
//!
//!   initialize -> notifications/initialized -> tools/list -> tools/call

use helpers::{handshake, rpc_id, send, spawn_server, wait_for_response};
use std::io::BufReader;

/// Test-only fixtures.
///
/// The module is marked `#[cfg(test)]` (which an integration test target always
/// is) so the `allow-unwrap-in-tests` / `allow-expect-in-tests` /
/// `allow-panic-in-tests` settings from `clippy.toml` apply here: a panic in
/// these helpers is the failure report, exactly as inside a `#[test]` body.
#[cfg(test)]
mod helpers {
    use std::io::{BufRead, Write};
    use std::process::{Child, Command, Stdio};

    pub(super) fn rpc_id(next: &mut i64) -> i64 {
        *next += 1;
        *next
    }

    /// Spawns the real binary in stdio mode with an isolated `SLUGTHUG_HOME`.
    pub(super) fn spawn_server(home: &std::path::Path) -> Child {
        Command::new(env!("CARGO_BIN_EXE_codeguards-mcp"))
            .arg("serve")
            .env("SLUGTHUG_HOME", home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn codeguards-mcp serve")
    }

    /// Sends one newline-delimited JSON-RPC message to the child's stdin.
    pub(super) fn send(child: &mut Child, msg: &str) {
        child
            .stdin
            .as_mut()
            .expect("child stdin must be piped")
            .write_all(msg.as_bytes())
            .expect("stdin write failed");
        child
            .stdin
            .as_mut()
            .expect("child stdin must be piped")
            .write_all(b"\n")
            .expect("stdin write failed");
        child
            .stdin
            .as_mut()
            .expect("child stdin must be piped")
            .flush()
            .expect("stdin flush failed");
    }

    /// Reads the next non-empty line and parses it as JSON, or `None` for a blank line.
    fn read_json_line(reader: &mut dyn BufRead, expected_id: i64) -> Option<serde_json::Value> {
        let mut line = String::new();
        let n = reader.read_line(&mut line).expect("stdout read failed");
        assert!(
            n != 0,
            "server closed stdout before responding to id {expected_id}"
        );
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(serde_json::from_str(trimmed).expect("non-JSON line on MCP stdio"))
    }

    /// Performs the mandatory `initialize` + `notifications/initialized`
    /// handshake and returns the `initialize` response.
    pub(super) fn handshake(
        child: &mut Child,
        reader: &mut dyn BufRead,
        next_id: &mut i64,
    ) -> serde_json::Value {
        let init_id = rpc_id(next_id);
        send(
            child,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"codeguards-test","version":"0.0.1"}}}"#,
        );
        let resp = wait_for_response(reader, init_id);
        send(
            child,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        );
        resp
    }

    /// Waits for the next JSON-RPC line whose `id` matches.
    /// Skips any notifications (e.g. logging) that have no id.
    pub(super) fn wait_for_response(
        reader: &mut dyn BufRead,
        expected_id: i64,
    ) -> serde_json::Value {
        for _ in 0..200 {
            let Some(v) = read_json_line(reader, expected_id) else {
                continue;
            };
            if v.get("id").and_then(serde_json::Value::as_i64) == Some(expected_id) {
                return v;
            }
            // Otherwise a notification or out-of-order message — skip.
        }
        panic!("no response for id {expected_id} within 200 lines");
    }
}

#[test]
fn mcp_stdio_full_handshake() {
    let home = tempfile::tempdir().expect("tempdir for SLUGTHUG_HOME");
    let mut child = spawn_server(home.path());
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut next_id = 0i64;

    // 1) + 2) initialize handshake
    let resp = handshake(&mut child, &mut reader, &mut next_id);
    let result = resp
        .get("result")
        .unwrap_or_else(|| panic!("initialize failed: {resp}"));
    assert_eq!(
        result["serverInfo"]["name"], "codeguards-mcp",
        "server identity must be codeguards-mcp"
    );
    assert!(
        result["capabilities"]["tools"].is_object(),
        "server must advertise tools capability"
    );

    // 3) tools/list
    let list_id = rpc_id(&mut next_id);
    send(
        &mut child,
        &format!("{{\"jsonrpc\":\"2.0\",\"id\":{list_id},\"method\":\"tools/list\"}}"),
    );
    let list_resp = wait_for_response(&mut reader, list_id);
    let tools = &list_resp["result"]["tools"];
    let tool_names: Vec<&str> = tools
        .as_array()
        .expect("tools/list must return an array")
        .iter()
        .map(|t| t["name"].as_str().expect("tool name"))
        .collect();
    for expected in [
        "validate_architecture",
        "list_guard_tests",
        "create_guard_test",
        "add_exception",
    ] {
        assert!(
            tool_names.contains(&expected),
            "missing {expected}, got {tool_names:?}"
        );
    }

    // 4) tools/call list_guard_tests — built-ins must be seeded into the
    //    hermetic SLUGTHUG_HOME and listed back.
    let call_id = rpc_id(&mut next_id);
    send(
        &mut child,
        &format!(
            r#"{{"jsonrpc":"2.0","id":{call_id},"method":"tools/call","params":{{"name":"list_guard_tests","arguments":{{}}}}}}"#
        ),
    );
    let call_resp = wait_for_response(&mut reader, call_id);
    let content = &call_resp["result"]["content"];
    assert!(
        content.is_array() && !content.as_array().unwrap().is_empty(),
        "list_guard_tests must return non-empty content: {call_resp}"
    );
    let text = content[0]["text"].as_str().expect("content[0].text");
    let parsed: serde_json::Value =
        serde_json::from_str(text).expect("list_guard_tests result must be JSON");
    assert!(
        parsed.as_object().is_some_and(|m| !m.is_empty()),
        "catalog must be non-empty, got {text}"
    );
    // Spot-check a known built-in exists under some id.
    let ids: Vec<String> = parsed.as_object().unwrap().keys().cloned().collect();
    assert!(
        ids.iter().any(|i| i.contains("no-unwrap")),
        "expected built-in no-unwrap guard in catalog, got {ids:?}"
    );

    // 5) tools/call validate_architecture against a temp project with no
    //    contract — must return a structured error result, not crash.
    let project = tempfile::tempdir().expect("tempdir for project");
    let val_id = rpc_id(&mut next_id);
    send(
        &mut child,
        &format!(
            r#"{{"jsonrpc":"2.0","id":{val_id},"method":"tools/call","params":{{"name":"validate_architecture","arguments":{{"project_path":"{}"}}}}}}"#,
            project.path().display()
        ),
    );
    let val_resp = wait_for_response(&mut reader, val_id);
    // Either isError=true or a result with is_valid=false — either is a
    // well-formed structured response. An unhandled crash would close the pipe.
    let is_error = val_resp["result"]["isError"].as_bool().unwrap_or(false);
    let has_result = val_resp["result"].is_object();
    assert!(
        is_error || has_result,
        "validate_architecture must answer, got {val_resp}"
    );
    if has_result {
        let text = val_resp["result"]["content"][0]["text"]
            .as_str()
            .expect("text");
        let parsed: serde_json::Value =
            serde_json::from_str(text).expect("validation result must be JSON");
        assert_eq!(
            parsed["is_valid"].as_bool(),
            Some(false),
            "project without ARCHITECTURE.md must validate as invalid"
        );
    }

    // 6) Shutdown: kill the child; it must not have already exited.
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn mcp_stdio_rejects_unknown_tool() {
    let home = tempfile::tempdir().expect("tempdir for SLUGTHUG_HOME");
    let mut child = spawn_server(home.path());
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut next_id = 0i64;

    let _ = handshake(&mut child, &mut reader, &mut next_id);

    let bad_id = rpc_id(&mut next_id);
    send(
        &mut child,
        &format!(
            r#"{{"jsonrpc":"2.0","id":{bad_id},"method":"tools/call","params":{{"name":"definitely_not_a_tool","arguments":{{}}}}}}"#
        ),
    );
    let resp = wait_for_response(&mut reader, bad_id);
    // Must be either a JSON-RPC error or a structured isError result —
    // never a silent success, never a dead pipe.
    let jsonrpc_error = resp.get("error").is_some();
    let structured_error = resp["result"]["isError"].as_bool().unwrap_or(false);
    assert!(
        jsonrpc_error || structured_error,
        "unknown tool must produce an error, got {resp}"
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn mcp_stdio_survives_garbage_lines_after_initialization() {
    let home = tempfile::tempdir().expect("tempdir for SLUGTHUG_HOME");
    let mut child = spawn_server(home.path());
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut next_id = 0i64;

    // 1) + 2) Initialize first (required by MCP spec)
    let resp = handshake(&mut child, &mut reader, &mut next_id);
    assert_eq!(
        resp["result"]["serverInfo"]["name"], "codeguards-mcp",
        "initialization must succeed"
    );

    // 3) Now send garbage — server must stay alive
    send(&mut child, "this is not json");

    // 4) Send a malformed request — expect error response, not disconnect
    let bad_id = rpc_id(&mut next_id);
    send(
        &mut child,
        &format!(r#"{{"jsonrpc":"2.0","id":{bad_id},"method":"nonexistent/method"}}"#),
    );
    let bad_resp = wait_for_response(&mut reader, bad_id);
    assert!(
        bad_resp.get("error").is_some(),
        "malformed request must get error response"
    );

    // 5) Send a valid tools/list — must still work
    let list_id = rpc_id(&mut next_id);
    send(
        &mut child,
        &format!("{{\"jsonrpc\":\"2.0\",\"id\":{list_id},\"method\":\"tools/list\"}}"),
    );
    let list_resp = wait_for_response(&mut reader, list_id);
    assert!(
        list_resp["result"]["tools"].is_array(),
        "tools/list must work after garbage"
    );

    let _ = child.kill();
    let _ = child.wait();
}
