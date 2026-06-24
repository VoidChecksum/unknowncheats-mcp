use maplit::hashmap;
use serde_json::json;
use unknowncheats_mcp::{config::Config, forum_client::ForumClient, mcp::McpServer};

#[tokio::test]
async fn exposes_generic_capability_tools_for_both_forums() {
    let response = server()
        .handle_json(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
        .await;

    let names: Vec<String> = response["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();

    for name in [
        "get_page",
        "submit_form",
        "submit_multipart_form",
        "user_cp",
        "subscribed_threads",
        "attachments",
        "edit_post",
        "delete_post",
        "report_post",
        "elitepvpers_get_page",
        "elitepvpers_submit_form",
        "elitepvpers_submit_multipart_form",
        "elitepvpers_user_cp",
        "elitepvpers_subscribed_threads",
        "elitepvpers_attachments",
        "elitepvpers_edit_post",
        "elitepvpers_delete_post",
        "elitepvpers_report_post",
    ] {
        assert!(names.contains(&name.to_string()), "missing {name}");
    }
}

#[tokio::test]
async fn generic_submit_form_is_write_gated() {
    let response = server()
        .handle_json(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "submit_form",
                "arguments": {"path": "profile.php?do=updateprofile", "fields": {"homepage": "https://example.com"}}
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
async fn generic_multipart_form_is_write_gated() {
    let response = server()
        .handle_json(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "submit_multipart_form",
                "arguments": {
                    "path": "newattachment.php?do=manageattach",
                    "fields": {"securitytoken": "token"},
                    "files": [{"field": "upload", "path": "/tmp/example.bin"}]
                }
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
async fn elitepvpers_report_post_is_write_gated() {
    let response = server()
        .handle_json(json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "elitepvpers_report_post",
                "arguments": {"post_id": "123", "reason": "spam"}
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
async fn elitepvpers_multipart_form_is_write_gated() {
    let response = server()
        .handle_json(json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "elitepvpers_submit_multipart_form",
                "arguments": {
                    "path": "newattachment.php?do=manageattach",
                    "fields": {"securitytoken": "token"},
                    "files": [{"field": "upload", "path": "/tmp/example.bin"}]
                }
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
