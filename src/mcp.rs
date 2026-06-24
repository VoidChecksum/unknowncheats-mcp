use crate::forum_client::{ForumClient, ForumHandle, UploadFile};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct McpServer {
    client: ForumClient,
}

impl McpServer {
    pub fn new(client: ForumClient) -> Self {
        Self { client }
    }

    pub async fn run_stdio(self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut lines = BufReader::new(stdin).lines();
        let mut stdout = tokio::io::stdout();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let request: Value = serde_json::from_str(&line)?;
            let response = self.handle_json(request).await;
            stdout
                .write_all(serde_json::to_string(&response)?.as_bytes())
                .await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
        Ok(())
    }

    pub async fn handle_json(&self, request: Value) -> Value {
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        match self.handle_request(&request).await {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err(err) => {
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": err.to_string()}})
            }
        }
    }

    async fn handle_request(&self, request: &Value) -> Result<Value> {
        match request
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "initialize" => Ok(
                json!({"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "forum-mcp", "version": "0.1.0"}}),
            ),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => {
                self.call_tool(request.get("params").unwrap_or(&Value::Null))
                    .await
            }
            method => Err(anyhow!("unsupported method: {method}")),
        }
    }

    async fn call_tool(&self, params: &Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing tool name"))?;
        let args = params.get("arguments").unwrap_or(&Value::Null);

        let result = if let Some(tool_name) = name.strip_prefix("elitepvpers_") {
            call_forum_tool(self.client.elitepvpers(), tool_name, args).await?
        } else {
            call_forum_tool(self.client.unknowncheats(), name, args).await?
        };

        Ok(json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&result)?}]}))
    }
}

async fn call_forum_tool(forum: ForumHandle<'_>, name: &str, args: &Value) -> Result<Value> {
    Ok(match name {
        "list_forums" => serde_json::to_value(forum.list_forums().await?)?,
        "search_forum" => {
            serde_json::to_value(forum.search_forum(required(args, "query")?).await?)?
        }
        "list_threads" => {
            serde_json::to_value(forum.list_threads(required(args, "forum_id")?).await?)?
        }
        "read_thread" => {
            serde_json::to_value(forum.read_thread(required(args, "thread_id")?).await?)?
        }
        "get_profile" => json!(forum.get_profile(required(args, "user_id")?).await?),
        "get_logged_in_user" | "user_cp" => json!(forum.user_cp().await?),
        "list_private_messages" => json!(forum.list_private_messages().await?),
        "get_page" => json!(forum.get_page(required(args, "path")?).await?),
        "subscribed_threads" => json!(forum.subscribed_threads().await?),
        "attachments" => json!(forum.attachments().await?),
        "submit_form" => json!(
            forum
                .submit_form(required(args, "path")?, &fields(args)?)
                .await?
        ),
        "submit_multipart_form" => json!(
            forum
                .submit_multipart_form(required(args, "path")?, &fields(args)?, &files(args)?)
                .await?
        ),
        "reply_to_thread" => json!(
            forum
                .reply_to_thread(required(args, "thread_id")?, required(args, "body")?)
                .await?
        ),
        "create_thread" => json!(
            forum
                .create_thread(
                    required(args, "forum_id")?,
                    required(args, "title")?,
                    required(args, "body")?
                )
                .await?
        ),
        "send_private_message" => json!(
            forum
                .send_private_message(
                    required(args, "to")?,
                    required(args, "title")?,
                    required(args, "body")?
                )
                .await?
        ),
        "edit_post" => json!(
            forum
                .edit_post(required(args, "post_id")?, required(args, "body")?)
                .await?
        ),
        "delete_post" => json!(
            forum
                .delete_post(required(args, "post_id")?, optional(args, "reason"))
                .await?
        ),
        "report_post" => json!(
            forum
                .report_post(required(args, "post_id")?, required(args, "reason")?)
                .await?
        ),
        _ => return Err(anyhow!("unknown tool: {name}")),
    })
}

fn required<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing argument: {key}"))
}

fn optional<'a>(args: &'a Value, key: &str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or("")
}

fn fields(args: &Value) -> Result<HashMap<String, String>> {
    let object = args
        .get("fields")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("missing argument: fields"))?;

    object
        .iter()
        .map(|(key, value)| {
            value
                .as_str()
                .map(|text| (key.clone(), text.to_string()))
                .ok_or_else(|| anyhow!("field {key} must be a string"))
        })
        .collect()
}

fn files(args: &Value) -> Result<Vec<UploadFile>> {
    let Some(array) = args.get("files").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    array
        .iter()
        .map(|item| {
            Ok(UploadFile {
                field: item
                    .get("field")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("file field is required"))?
                    .to_string(),
                path: item
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("file path is required"))?
                    .to_string(),
            })
        })
        .collect()
}

fn tools() -> Value {
    let mut out = forum_tools("");
    out.extend(forum_tools("elitepvpers_"));
    Value::Array(out)
}

fn forum_tools(prefix: &str) -> Vec<Value> {
    vec![
        tool(
            prefix,
            "list_forums",
            "List forum categories",
            empty_schema(),
        ),
        tool(
            prefix,
            "search_forum",
            "Search threads",
            schema(&[("query", "string")], &["query"]),
        ),
        tool(
            prefix,
            "list_threads",
            "List threads in a forum",
            schema(&[("forum_id", "string")], &["forum_id"]),
        ),
        tool(
            prefix,
            "read_thread",
            "Read posts in a thread",
            schema(&[("thread_id", "string")], &["thread_id"]),
        ),
        tool(
            prefix,
            "get_profile",
            "Fetch a user profile page",
            schema(&[("user_id", "string")], &["user_id"]),
        ),
        tool(
            prefix,
            "get_logged_in_user",
            "Fetch account control panel page",
            empty_schema(),
        ),
        tool(
            prefix,
            "list_private_messages",
            "Fetch private messages page",
            empty_schema(),
        ),
        tool(
            prefix,
            "get_page",
            "Fetch any forum-relative page path",
            schema(&[("path", "string")], &["path"]),
        ),
        tool(
            prefix,
            "user_cp",
            "Fetch user control panel",
            empty_schema(),
        ),
        tool(
            prefix,
            "subscribed_threads",
            "Fetch subscribed threads page",
            empty_schema(),
        ),
        tool(
            prefix,
            "attachments",
            "Fetch attachments manager page",
            empty_schema(),
        ),
        tool(
            prefix,
            "submit_form",
            "Submit any forum-relative form path with string fields",
            form_schema(),
        ),
        tool(
            prefix,
            "submit_multipart_form",
            "Submit any multipart form path with string fields and local files",
            multipart_form_schema(),
        ),
        tool(
            prefix,
            "reply_to_thread",
            "Reply to a thread",
            schema(
                &[("thread_id", "string"), ("body", "string")],
                &["thread_id", "body"],
            ),
        ),
        tool(
            prefix,
            "create_thread",
            "Create a thread",
            schema(
                &[
                    ("forum_id", "string"),
                    ("title", "string"),
                    ("body", "string"),
                ],
                &["forum_id", "title", "body"],
            ),
        ),
        tool(
            prefix,
            "send_private_message",
            "Send a private message",
            schema(
                &[("to", "string"), ("title", "string"), ("body", "string")],
                &["to", "title", "body"],
            ),
        ),
        tool(
            prefix,
            "edit_post",
            "Edit an existing post",
            schema(
                &[("post_id", "string"), ("body", "string")],
                &["post_id", "body"],
            ),
        ),
        tool(
            prefix,
            "delete_post",
            "Delete a post if account has permission",
            schema(&[("post_id", "string"), ("reason", "string")], &["post_id"]),
        ),
        tool(
            prefix,
            "report_post",
            "Report a post",
            schema(
                &[("post_id", "string"), ("reason", "string")],
                &["post_id", "reason"],
            ),
        ),
    ]
}

fn tool(prefix: &str, name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name": format!("{prefix}{name}"), "description": description, "inputSchema": input_schema})
}

fn empty_schema() -> Value {
    json!({"type": "object", "properties": {}})
}

fn form_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string"},
            "fields": {"type": "object", "additionalProperties": {"type": "string"}}
        },
        "required": ["path", "fields"]
    })
}

fn multipart_form_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string"},
            "fields": {"type": "object", "additionalProperties": {"type": "string"}},
            "files": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "field": {"type": "string"},
                        "path": {"type": "string"}
                    },
                    "required": ["field", "path"]
                }
            }
        },
        "required": ["path", "fields"]
    })
}

fn schema(props: &[(&str, &str)], required: &[&str]) -> Value {
    let properties = props
        .iter()
        .map(|(name, kind)| ((*name).to_string(), json!({"type": kind})))
        .collect::<serde_json::Map<_, _>>();
    json!({"type": "object", "properties": properties, "required": required})
}
