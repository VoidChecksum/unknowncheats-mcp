use unknowncheats_mcp::{config::Config, forum_client::ForumClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter("info")
        .init();

    let cfg = Config::from_env()?;
    let client = ForumClient::new(cfg)?;

    println!("=== Testing UnknownCheats (no Cloudflare) ===");
    match client.unknowncheats().list_forums().await {
        Ok(forums) => {
            println!("✓ Found {} forums", forums.len());
            for forum in forums.iter().take(5) {
                println!("  - [{}] {}", forum.id, forum.title);
            }
        }
        Err(e) => {
            println!("✗ Error: {}", e);
        }
    }

    println!("\n=== Testing Elitepvpers (Cloudflare bypass) ===");
    println!("This will spawn Byparr Docker container on first request...");
    match client.elitepvpers().list_forums().await {
        Ok(forums) => {
            println!("✓ Found {} forums", forums.len());
            for forum in forums.iter().take(5) {
                println!("  - [{}] {}", forum.id, forum.title);
            }
        }
        Err(e) => {
            println!("✗ Error: {}", e);
        }
    }

    Ok(())
}
