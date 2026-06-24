use maplit::hashmap;
use unknowncheats_mcp::{config::Config, forum_client::ForumClient};

#[tokio::test]
async fn write_tools_are_disabled_by_default() {
    let cfg = Config::from_env_map(hashmap! {
        "UC_COOKIE".to_string() => "a=b".to_string(),
    })
    .unwrap();
    let client = ForumClient::new(cfg).unwrap();

    let err = client.reply_to_thread("1", "body").await.unwrap_err();
    assert!(err.to_string().contains("disabled"));
}
