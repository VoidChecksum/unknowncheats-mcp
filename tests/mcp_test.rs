use maplit::hashmap;
use serde_json::json;
use unknowncheats_mcp::{config::Config, forum_client::ForumClient, mcp::McpServer};

#[tokio::test]
async fn lists_tools_for_both_forums() {
    let server = server();

    let response = server
        .handle_json(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        }))
        .await;

    let names: Vec<String> = response["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();

    assert!(names.contains(&"read_thread".to_string()));
    assert!(names.contains(&"reply_to_thread".to_string()));
    assert!(names.contains(&"elitepvpers_read_thread".to_string()));
    assert!(names.contains(&"elitepvpers_reply_to_thread".to_string()));
}

#[tokio::test]
async fn disabled_unknowncheats_write_tool_returns_mcp_error() {
    let server = server();

    let response = server
        .handle_json(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "reply_to_thread",
                "arguments": {"thread_id": "1", "body": "hello"}
            }
        }))
        .await;

    assert_eq!(response["error"]["code"], -32000);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("disabled"));
}

#[tokio::test]
async fn disabled_elitepvpers_write_tool_returns_mcp_error() {
    let server = server();

    let response = server
        .handle_json(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "elitepvpers_reply_to_thread",
                "arguments": {"thread_id": "1", "body": "hello"}
            }
        }))
        .await;

    assert_eq!(response["error"]["code"], -32000);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("disabled"));
}

fn server() -> McpServer {
    let cfg = Config::from_env_map(hashmap! {
        "UC_COOKIE".to_string() => "a=b".to_string(),
        "EP_COOKIE".to_string() => "c=d".to_string(),
    })
    .unwrap();
    McpServer::new(ForumClient::new(cfg).unwrap())
}
