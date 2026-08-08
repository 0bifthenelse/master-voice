use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const PROTOCOL_VERSION: &str = "2025-03-26";
const SERVER_NAME: &str = "master-voice";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

static NEXT_DAEMON_ID: AtomicU64 = AtomicU64::new(2);

#[derive(Debug, Deserialize)]
struct RpcRequest {
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

fn rpc_result(id: Option<Value>, result: Value) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn rpc_error(id: Option<Value>, code: i32, message: impl Into<String>) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(RpcError {
            code,
            message: message.into(),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct CallParams {
    name: String,
    arguments: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct SpeakArguments {
    text: String,
    language: Option<String>,
    interrupt: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CancelledParams {
    #[serde(rename = "requestId")]
    request_id: Option<Value>,
}

fn speak_tool_definition() -> Value {
    serde_json::json!({
        "name": "speak",
        "description": "Synthesize the given text with the MASTER robotic voice and play it through the default audio output. The text is treated strictly as speech data: it is never executed, interpreted, or sent anywhere.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "Text to speak aloud" },
                "language": { "type": "string", "description": "Optional language: fr-FR or en-US" },
                "interrupt": { "type": "boolean", "description": "Stop the current utterance before speaking" }
            },
            "required": ["text"]
        }
    })
}

fn handle_initialize(id: Option<Value>, params: Option<Value>) -> RpcResponse {
    let client_version = params
        .as_ref()
        .and_then(|p| p.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(PROTOCOL_VERSION)
        .to_string();
    rpc_result(
        id,
        serde_json::json!({
            "protocolVersion": client_version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
        }),
    )
}

fn format_duration(seconds: f32) -> String {
    format!("{seconds:.1}s")
}

struct SpeakReport {
    language: String,
    duration_s: f32,
}

fn speak_text(
    daemon_id: u64,
    text: &str,
    language: Option<&str>,
    interrupt: bool,
) -> Result<SpeakReport, String> {
    let mut client = master_voice_core::daemon::client::DaemonClient::connect_or_spawn()
        .map_err(|e| e.to_string())?;
    let report = client
        .speak_with_id(daemon_id, text, language, interrupt)
        .map_err(|e| e.to_string())?;
    Ok(SpeakReport {
        language: report.language,
        duration_s: report.duration_s,
    })
}

pub fn serve_io<R: BufRead, W: Write + Send + 'static>(
    reader: R,
    writer: W,
) -> std::io::Result<()> {
    let writer = Arc::new(Mutex::new(writer));
    let mut reader = reader;
    let mut line = String::new();
    let in_flight: Arc<Mutex<Vec<(Value, u64)>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let request: RpcRequest = match serde_json::from_str(trimmed) {
            Ok(request) => request,
            Err(_) => {
                let response = rpc_error(None, -32700, "parse error");
                write_response(&writer, &response)?;
                continue;
            }
        };
        let is_notification = request.id.is_none();
        let request_id = request.id.clone();

        match request.method.as_str() {
            "initialize" => {
                let response = handle_initialize(request_id.clone(), request.params);
                write_response(&writer, &response)?;
            }
            "notifications/initialized" => {}
            "ping" => {
                write_response(&writer, &rpc_result(request_id, Value::Null))?;
            }
            "tools/list" => {
                write_response(
                    &writer,
                    &rpc_result(
                        request_id,
                        serde_json::json!({ "tools": [speak_tool_definition()] }),
                    ),
                )?;
            }
            "tools/call" => {
                let call: CallParams =
                    match request.params.and_then(|p| serde_json::from_value(p).ok()) {
                        Some(call) => call,
                        None => {
                            write_response(
                                &writer,
                                &rpc_error(request_id, -32602, "invalid tools/call params"),
                            )?;
                            continue;
                        }
                    };
                if call.name != "speak" {
                    write_response(
                        &writer,
                        &rpc_error(request_id, -32602, format!("unknown tool {}", call.name)),
                    )?;
                    continue;
                }
                let args: SpeakArguments =
                    match call.arguments.and_then(|a| serde_json::from_value(a).ok()) {
                        Some(args) => args,
                        None => {
                            write_response(
                                &writer,
                                &rpc_error(request_id, -32602, "invalid arguments"),
                            )?;
                            continue;
                        }
                    };
                let text = args.text;
                if text.trim().is_empty() {
                    write_response(
                        &writer,
                        &rpc_error(request_id, -32602, "text must not be empty"),
                    )?;
                    continue;
                }
                let daemon_id = NEXT_DAEMON_ID.fetch_add(1, Ordering::Relaxed);
                if let Some(id) = request_id.clone() {
                    in_flight.lock().push((id, daemon_id));
                }
                let writer = Arc::clone(&writer);
                let in_flight = Arc::clone(&in_flight);
                let response_id = request_id;
                std::thread::spawn(move || {
                    let result = speak_text(
                        daemon_id,
                        &text,
                        args.language.as_deref(),
                        args.interrupt.unwrap_or(false),
                    );
                    let response = match result {
                        Ok(report) => rpc_result(
                            response_id.clone(),
                            serde_json::json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!(
                                        "Spoken {} ({}).",
                                        report.language,
                                        format_duration(report.duration_s)
                                    )
                                }],
                                "isError": false
                            }),
                        ),
                        Err(message) => rpc_result(
                            response_id.clone(),
                            serde_json::json!({
                                "content": [{ "type": "text", "text": message }],
                                "isError": true
                            }),
                        ),
                    };
                    let rid = response_id.clone().unwrap_or(Value::Null);
                    in_flight.lock().retain(|(id, _)| *id != rid);
                    let _ = write_response(&writer, &response);
                });
            }
            "notifications/cancelled" => {
                let cancelled: CancelledParams = request
                    .params
                    .and_then(|p| serde_json::from_value(p).ok())
                    .unwrap_or(CancelledParams { request_id: None });
                if let Some(request_id) = cancelled.request_id {
                    let daemon_id = in_flight
                        .lock()
                        .iter()
                        .find(|(id, _)| *id == request_id)
                        .map(|(_, daemon_id)| *daemon_id);
                    if let Some(daemon_id) = daemon_id {
                        if let Ok(mut client) =
                            master_voice_core::daemon::client::DaemonClient::connect()
                        {
                            client.cancel(daemon_id);
                        }
                    }
                }
            }
            "resources/list" => {
                write_response(
                    &writer,
                    &rpc_result(request_id, serde_json::json!({ "resources": [] })),
                )?;
            }
            "prompts/list" => {
                write_response(
                    &writer,
                    &rpc_result(request_id, serde_json::json!({ "prompts": [] })),
                )?;
            }
            "shutdown" => {
                write_response(&writer, &rpc_result(request_id, Value::Null))?;
                return Ok(());
            }
            "exit" => {
                return Ok(());
            }
            other => {
                write_response(
                    &writer,
                    &rpc_error(request_id, -32601, format!("method not found: {other}")),
                )?;
            }
        }
        let _ = is_notification;
    }
}

fn write_response<W: Write>(writer: &Mutex<W>, response: &RpcResponse) -> std::io::Result<()> {
    let mut guard = writer.lock();
    let json = serde_json::to_string(response)?;
    guard.write_all(json.as_bytes())?;
    guard.write_all(b"\n")?;
    guard.flush()
}

pub fn serve_stdio() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve_io(BufReader::new(stdin.lock()), stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, BufWriter, Write};
    use std::os::unix::net::UnixStream;

    fn send_line(stream: &mut UnixStream, line: &str) {
        stream.write_all(line.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();
    }

    fn read_event(stream: &mut UnixStream) -> Value {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        serde_json::from_str(line.trim()).unwrap()
    }

    #[test]
    fn full_protocol_session() {
        let (server_side, mut client_side) = UnixStream::pair().unwrap();
        let handle = std::thread::spawn(move || {
            let writer = BufWriter::new(server_side.try_clone().unwrap());
            let reader = BufReader::new(server_side);
            serve_io(reader, writer).unwrap();
        });

        send_line(
            &mut client_side,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#,
        );
        let response = read_event(&mut client_side);
        assert_eq!(response["result"]["serverInfo"]["name"], "master-voice");
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");

        send_line(
            &mut client_side,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        );
        send_line(
            &mut client_side,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        );
        let response = read_event(&mut client_side);
        assert_eq!(response["result"]["tools"][0]["name"], "speak");
        let schema = &response["result"]["tools"][0]["inputSchema"];
        assert_eq!(schema["required"][0], "text");

        send_line(
            &mut client_side,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"speak","arguments":{"text":"hello"}}}"#,
        );
        let response = read_event(&mut client_side);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("Spoken") || text.contains("daemon") || text.contains("audio"),
            "unexpected tool result: {text}"
        );

        send_line(
            &mut client_side,
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"speak","arguments":{"text":""}}}"#,
        );
        let response = read_event(&mut client_side);
        assert_eq!(response["error"]["code"], -32602);

        send_line(
            &mut client_side,
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#,
        );
        let response = read_event(&mut client_side);
        assert_eq!(response["error"]["code"], -32602);

        send_line(
            &mut client_side,
            r#"{"jsonrpc":"2.0","id":6,"method":"bogus_method"}"#,
        );
        let response = read_event(&mut client_side);
        assert_eq!(response["error"]["code"], -32601);

        send_line(
            &mut client_side,
            r#"{"jsonrpc":"2.0","id":7,"method":"shutdown"}"#,
        );
        let response = read_event(&mut client_side);
        assert!(response["result"].is_null());

        handle.join().unwrap();
    }

    #[test]
    fn rejects_garbage_frames() {
        let (server_side, mut client_side) = UnixStream::pair().unwrap();
        let handle = std::thread::spawn(move || {
            let writer = BufWriter::new(server_side.try_clone().unwrap());
            let reader = BufReader::new(server_side);
            serve_io(reader, writer).unwrap();
        });
        send_line(&mut client_side, "this is not json");
        let response = read_event(&mut client_side);
        assert_eq!(response["error"]["code"], -32700);
        drop(client_side);
        handle.join().unwrap();
    }

    #[test]
    fn stdout_has_no_log_noise() {
        let (server_side, client_side) = UnixStream::pair().unwrap();
        let writer = BufWriter::new(server_side.try_clone().unwrap());
        let reader = BufReader::new(server_side);
        let handle = std::thread::spawn(move || serve_io(reader, writer).unwrap());
        drop(client_side);
        handle.join().unwrap();
    }
}
