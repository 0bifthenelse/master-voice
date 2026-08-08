use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpClient {
    fn spawn(runtime_dir: &std::path::Path) -> Self {
        let bin = env!("CARGO_BIN_EXE_master-voice");
        let mut child = Command::new(bin)
            .arg("mcp")
            .env("XDG_RUNTIME_DIR", runtime_dir)
            .env("XDG_CONFIG_HOME", runtime_dir.join("config"))
            .env("XDG_CACHE_HOME", runtime_dir.join("cache"))
            .env("RUST_LOG", "off")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn master-voice mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn send(&mut self, value: Value) {
        let line = serde_json::to_string(&value).unwrap();
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert!(!line.trim().is_empty(), "EOF from MCP server");
        serde_json::from_str(line.trim()).unwrap()
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }));
        self.recv()
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn make_env() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "mv-mcp-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn mcp_protocol_roundtrip() {
    let runtime_dir = make_env();
    let mut client = McpClient::spawn(&runtime_dir);

    let response = client.request(
        1,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": { "name": "it-test", "version": "1" }
        }),
    );
    assert_eq!(response["result"]["serverInfo"]["name"], "master-voice");
    assert_eq!(response["result"]["protocolVersion"], "2025-03-26");

    client.send(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}));

    let response = client.request(2, "tools/list", serde_json::json!({}));
    let tools = response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "speak");
    assert_eq!(tools[0]["inputSchema"]["required"][0], "text");

    let response = client.request(
        3,
        "tools/call",
        serde_json::json!({
            "name": "speak",
            "arguments": { "text": "hello", "language": "en-US" }
        }),
    );
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("Spoken") || text.contains("audio") || text.contains("daemon"),
        "unexpected: {text}"
    );

    let response = client.request(
        4,
        "tools/call",
        serde_json::json!({ "name": "speak", "arguments": { "text": "" } }),
    );
    assert_eq!(response["error"]["code"], -32602);

    let response = client.request(
        5,
        "tools/call",
        serde_json::json!({ "name": "speak", "arguments": { "text": "hi", "language": "klingon" } }),
    );
    assert!(response["result"]["isError"].as_bool() == Some(true));

    let response = client.request(6, "ping", serde_json::json!({}));
    assert!(response["result"].is_null());

    let response = client.request(7, "shutdown", serde_json::json!({}));
    assert!(response["result"].is_null());

    let status = client.child.wait().unwrap();
    assert!(status.success(), "server exited non-zero after shutdown");
}

#[test]
fn mcp_stdout_is_protocol_only() {
    let runtime_dir = make_env();
    let mut client = McpClient::spawn(&runtime_dir);
    let response = client.request(
        1,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": { "name": "it-test", "version": "1" }
        }),
    );
    assert_eq!(response["result"]["serverInfo"]["name"], "master-voice");
    client.send(serde_json::json!({"jsonrpc":"2.0","method":"shutdown","id":2}));
    let _ = client.recv();
    let status = client.child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn mcp_large_text_handled() {
    let runtime_dir = make_env();
    let mut client = McpClient::spawn(&runtime_dir);
    let _ = client.request(
        1,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": { "name": "it-test", "version": "1" }
        }),
    );
    let large = "word ".repeat(500);
    let response = client.request(
        2,
        "tools/call",
        serde_json::json!({
            "name": "speak",
            "arguments": { "text": large }
        }),
    );
    assert!(response["result"]["isError"].is_boolean());
    client.send(serde_json::json!({"jsonrpc":"2.0","method":"shutdown","id":3}));
    let _ = client.recv();
    let _ = client.child.wait();
}
