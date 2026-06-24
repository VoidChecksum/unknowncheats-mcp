use anyhow::Result;
use unknowncheats_mcp::{config::Config, forum_client::ForumClient, mcp::McpServer};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let cfg = Config::from_env()?;
    let client = ForumClient::new(cfg)?;
    McpServer::new(client).run_stdio().await
}
