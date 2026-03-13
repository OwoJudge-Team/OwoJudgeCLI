use anyhow::Result;
use async_trait::async_trait;
use mcp_sdk_rs::error::ErrorCode;
use mcp_sdk_rs::server::{Server, ServerHandler};
use mcp_sdk_rs::transport::stdio::StdioTransport;
use mcp_sdk_rs::types::{ClientCapabilities, Implementation, ServerCapabilities};
use owojudge_cli::client::OwoClient;
use owojudge_cli::config::AppConfig;
use owojudge_cli::models::{
    Announcement, Contest, Problem, Standing, Submission, SubmissionList, User,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::Mutex;

/// Extract the PDF URL from a problem description, handling both HTML iframes
/// and OwoJudge's `{{iframe:URL}}` template syntax.
fn extract_pdf_url(description: &str) -> Option<String> {
    let src = extract_html_iframe_src(description)
        .or_else(|| extract_template_iframe_src(description))?;
    if let Some(pos) = src.to_lowercase().find("url=") {
        let encoded = src[pos + 4..].split('&').next().unwrap_or("");
        Some(percent_decode(encoded))
    } else if src.to_lowercase().ends_with(".pdf") || src.contains("/download") {
        Some(src)
    } else {
        None
    }
}

fn extract_html_iframe_src(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<iframe")?;
    let tag_end = lower[start..].find('>')? + start;
    let tag = &html[start..tag_end];
    let pattern = "src=\"";
    let pos = tag.to_lowercase().find(pattern)? + pattern.len();
    let rest = &tag[pos..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_template_iframe_src(description: &str) -> Option<String> {
    let prefix = "{{iframe:";
    let start = description.find(prefix)? + prefix.len();
    let end = description[start..].find("}}") ? + start;
    Some(description[start..end].to_string())
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(n) = u8::from_str_radix(hex, 16) {
                result.push(n);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

struct MyServerHandler {
    client: Arc<Mutex<OwoClient>>,
}

#[async_trait]
impl ServerHandler for MyServerHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, mcp_sdk_rs::Error> {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(json!({ "listChanged": true }));
        Ok(caps)
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, mcp_sdk_rs::Error> {
        let mut client = self.client.lock().await;

        match method {
            "tools/list" => {
                let tools = vec![
                    json!({
                        "name": "login",
                        "description": "Log in to the judge. Note: Sessions are persisted in config.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "username": { "type": "string" },
                                "password": { "type": "string" }
                            },
                            "required": ["username", "password"]
                        }
                    }),
                    json!({
                        "name": "logout",
                        "description": "Log out from the judge and clear local session.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }),
                    json!({
                        "name": "get_auth_status",
                        "description": "Check current authentication status and user info.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }),
                    json!({
                        "name": "list_problems",
                        "description": "List all available programming problems",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }),
                    json!({
                        "name": "get_problem",
                        "description": "Get detailed information about a problem. Automatically converts PDF descriptions to text.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "serial_number": { "type": "integer" }
                            },
                            "required": ["serial_number"]
                        }
                    }),
                    json!({
                        "name": "submit_solution",
                        "description": "Submit a code solution for a problem.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "problem_serial_number": { "type": "integer" },
                                "language": { "type": "string", "description": "e.g. 'gcc c17', 'g++ c++17', 'python3'" },
                                "filename": { "type": "string" },
                                "content": { "type": "string", "description": "The source code content" }
                            },
                            "required": ["problem_serial_number", "language", "filename", "content"]
                        }
                    }),
                    json!({
                        "name": "list_submissions",
                        "description": "List recent code submissions",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }),
                    json!({
                        "name": "get_submission",
                        "description": "Get detailed information about a submission including results and code",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "serial_number": { "type": "integer" }
                            },
                            "required": ["serial_number"]
                        }
                    }),
                    json!({
                        "name": "list_announcements",
                        "description": "List all judge announcements",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }),
                    json!({
                        "name": "list_contests",
                        "description": "List available programming contests",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }),
                    json!({
                        "name": "get_contest_standings",
                        "description": "Get the scoreboard for a specific contest",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "contest_id": { "type": "string" }
                            },
                            "required": ["contest_id"]
                        }
                    }),
                    json!({
                        "name": "get_contest",
                        "description": "Get detailed information about a specific contest including its problem list",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "contest_id": { "type": "string" }
                            },
                            "required": ["contest_id"]
                        }
                    }),
                    json!({
                        "name": "get_announcement",
                        "description": "Get the full content of a specific announcement",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "announcement_id": { "type": "string" }
                            },
                            "required": ["announcement_id"]
                        }
                    }),
                ];
                Ok(json!({ "tools": tools }))
            }
            "tools/call" => {
                let params = params.ok_or_else(|| {
                    mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing params")
                })?;
                let name = params["name"].as_str().ok_or_else(|| {
                    mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing name")
                })?;
                let args = &params["arguments"];

                match name {
                    "login" => {
                        let username = args["username"].as_str().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing username")
                        })?;
                        let password = args["password"].as_str().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing password")
                        })?;
                        client.login(username, password).await.map_err(|e| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": "Logged in successfully."
                            }]
                        }))
                    }
                    "logout" => {
                        client.logout().await.map_err(|e| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": "Logged out successfully."
                            }]
                        }))
                    }
                    "get_auth_status" => {
                        let user: User = client.get("/api/auth/status").await.map_err(|e| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&user).unwrap()
                            }]
                        }))
                    }
                    "list_problems" => {
                        let problems: Vec<Problem> = client.get("/api/problems").await.map_err(|e| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&problems).unwrap()
                            }]
                        }))
                    }
                    "get_problem" => {
                        let sn = args["serial_number"].as_u64().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(
                                ErrorCode::InvalidParams,
                                "Missing serial_number",
                            )
                        })?;
                        let mut problem: Problem = client
                            .get(&format!("/api/problems/{}", sn))
                            .await
                            .map_err(|e| {
                                mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                            })?;

                        // PDF Conversion Logic
                        if let Some(desc) = &problem.description {
                            if let Some(pdf_url) = extract_pdf_url(desc) {
                                let resolved_url = client.resolve_url(&pdf_url).map_err(|e| {
                                    mcp_sdk_rs::Error::protocol(
                                        ErrorCode::InternalError,
                                        e.to_string(),
                                    )
                                })?;

                                let pdf_data = client.download_url(&resolved_url).await.map_err(|e| {
                                    mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                                })?;

                                let tmp_path = format!("/tmp/problem_{}.pdf", sn);
                                std::fs::write(&tmp_path, pdf_data).map_err(|e| {
                                    mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                                })?;

                                eprintln!("Converting PDF {} to text using gemini...", tmp_path);

                                let output = Command::new("gemini")
                                    .arg("-p")
                                    .arg(format!("Extract all text from the following PDF file and return it as markdown: {}", tmp_path))
                                    .output()
                                    .await
                                    .map_err(|e| mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string()))?;

                                let _ = std::fs::remove_file(&tmp_path);

                                if output.status.success() {
                                    let converted_text = String::from_utf8_lossy(&output.stdout).to_string();
                                    problem.description = Some(converted_text);
                                } else {
                                    let err_text = String::from_utf8_lossy(&output.stderr);
                                    eprintln!("Gemini PDF conversion failed: {}", err_text);
                                }
                            }
                        }

                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&problem).unwrap()
                            }]
                        }))
                    }
                    "submit_solution" => {
                        let problem_id = args["problem_serial_number"].as_u64().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(
                                ErrorCode::InvalidParams,
                                "Missing problem_serial_number",
                            )
                        })?;
                        let language = args["language"].as_str().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing language")
                        })?;
                        let filename = args["filename"].as_str().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing filename")
                        })?;
                        let content = args["content"].as_str().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing content")
                        })?;

                        let body = json!({
                            "problemSerialNumber": problem_id,
                            "language": language,
                            "userSolution": [{
                                "filename": filename,
                                "content": content
                            }]
                        });

                        let res: Submission = client.post("/api/submissions", &body).await.map_err(|e| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Submission created successfully.\nSerial Number: {}\nStatus: {}", res.serial_number, res.status)
                            }]
                        }))
                    }
                    "list_submissions" => {
                        let submissions: SubmissionList = client.get("/api/submissions").await.map_err(|e| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&submissions).unwrap()
                            }]
                        }))
                    }
                    "get_submission" => {
                        let sn = args["serial_number"].as_u64().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(
                                ErrorCode::InvalidParams,
                                "Missing serial_number",
                            )
                        })?;
                        let submission: Submission = client
                            .get(&format!("/api/submission/{}", sn))
                            .await
                            .map_err(|e| {
                                mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                            })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&submission).unwrap()
                            }]
                        }))
                    }
                    "list_announcements" => {
                        let announcements: Vec<Announcement> =
                            client.get("/api/announcement").await.map_err(|e| {
                                mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                            })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&announcements).unwrap()
                            }]
                        }))
                    }
                    "list_contests" => {
                        let contests: Vec<Contest> = client.get("/api/contests").await.map_err(|e| {
                            mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&contests).unwrap()
                            }]
                        }))
                    }
                    "get_contest_standings" => {
                        let id = args["contest_id"].as_str().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(
                                ErrorCode::InvalidParams,
                                "Missing contest_id",
                            )
                        })?;
                        let standings: Vec<Standing> = client
                            .get(&format!("/api/contests/{}/standings", id))
                            .await
                            .map_err(|e| {
                                mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                            })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&standings).unwrap()
                            }]
                        }))
                    }
                    "get_contest" => {
                        let id = args["contest_id"].as_str().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(
                                ErrorCode::InvalidParams,
                                "Missing contest_id",
                            )
                        })?;
                        let contest: Contest = client
                            .get(&format!("/api/contests/{}", id))
                            .await
                            .map_err(|e| {
                                mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                            })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&contest).unwrap()
                            }]
                        }))
                    }
                    "get_announcement" => {
                        let id = args["announcement_id"].as_str().ok_or_else(|| {
                            mcp_sdk_rs::Error::protocol(
                                ErrorCode::InvalidParams,
                                "Missing announcement_id",
                            )
                        })?;
                        let announcement: Announcement = client
                            .get(&format!("/api/announcement/{}", id))
                            .await
                            .map_err(|e| {
                                mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string())
                            })?;
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&announcement).unwrap()
                            }]
                        }))
                    }
                    _ => Err(mcp_sdk_rs::Error::protocol(
                        ErrorCode::MethodNotFound,
                        format!("Unknown tool: {}", name),
                    )),
                }
            }
            _ => Err(mcp_sdk_rs::Error::protocol(
                ErrorCode::MethodNotFound,
                format!("Method not found: {}", method),
            )),
        }
    }

    async fn shutdown(&self) -> Result<(), mcp_sdk_rs::Error> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    let client = OwoClient::new(config);
    let handler = Arc::new(MyServerHandler {
        client: Arc::new(Mutex::new(client)),
    });

    let (read_tx, read_rx) = tokio::sync::mpsc::channel::<String>(100);
    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<String>(100);

    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
        let mut line = String::new();
        while let Ok(n) = reader.read_line(&mut line).await {
            if n == 0 {
                break;
            }
            if read_tx.send(line.clone()).await.is_err() {
                break;
            }
            line.clear();
        }
    });

    tokio::spawn(async move {
        let mut writer = tokio::io::stdout();
        while let Some(msg) = write_rx.recv().await {
            if writer.write_all(msg.as_bytes()).await.is_err() {
                break;
            }
            if writer.write_all(b"\n").await.is_err() {
                break;
            }
            if writer.flush().await.is_err() {
                break;
            }
        }
    });

    let transport = Arc::new(StdioTransport::new(read_rx, write_tx));
    let server = Server::new(transport, handler);

    server.start().await.map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
