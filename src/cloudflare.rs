use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use antibot_rs::{Antibot, Cookie, Provider, SolveRequest};

/// Cloudflare bypass using antibot-rs (Byparr/FlareSolverr)
/// Automatically manages Docker container lifecycle
#[derive(Default)]
pub struct CloudflareBypass {
    client: Arc<RwLock<Option<Arc<Antibot>>>>,
}

impl CloudflareBypass {
    pub fn new() -> Self {
        Self::default()
    }

    /// Solve Cloudflare challenge and return HTML + cookie header
    /// Passes existing session cookies to preserve vBulletin auth
    /// Byparr automatically:
    /// - Spawns headless Chrome in Docker
    /// - Solves JavaScript challenges (interstitial, Turnstile, etc.)
    /// - Returns merged cf_clearance + session cookies
    pub async fn solve(&self, url: &str, existing_cookies: &str) -> Result<(String, String)> {
        let antibot = self.get_or_create_client().await?;

        // Parse existing cookies
        let cookies = parse_cookies(existing_cookies);

        info!(
            "Cloudflare bypass: solving challenge via Byparr for {} ({} session cookies)",
            url,
            cookies.len()
        );

        // Build request with session cookies
        let mut req = SolveRequest::get(url);
        for cookie in cookies {
            req = req.with_cookie(cookie);
        }

        // Use execute() to pass cookies
        match antibot.execute(req).await {
            Ok(solution) => {
                let html = solution.html().to_string();
                let cookies = solution.cookie_header();
                info!(
                    "Cloudflare bypass: obtained {} cookies",
                    solution.cookies.len()
                );
                Ok((html, cookies))
            }
            Err(e) => {
                info!("Cloudflare bypass failed: {}", e);
                Err(e.into())
            }
        }
    }

    async fn get_or_create_client(&self) -> Result<Arc<Antibot>> {
        let client = self.client.read().await;
        if let Some(ref antibot) = *client {
            return Ok(antibot.clone());
        }
        drop(client);

        // Create new client with write lock
        let mut client = self.client.write().await;
        let antibot = Arc::new(
            Antibot::builder()
                .provider(Provider::Byparr)
                .auto_start(true)
                .enable_session_cache()
                .build()
                .await?,
        );

        *client = Some(antibot.clone());
        info!("Cloudflare bypass: Byparr container started");
        Ok(antibot)
    }
}

/// Parse cookie header string into Cookie structs
fn parse_cookies(header: &str) -> Vec<Cookie> {
    header
        .split(';')
        .filter_map(|pair| {
            let pair = pair.trim();
            if pair.is_empty() {
                return None;
            }
            let (name, value) = pair.split_once('=')?;
            Some(Cookie::new(name.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}