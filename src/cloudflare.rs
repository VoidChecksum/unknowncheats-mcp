use anyhow::Result;
use antibot_rs::{Antibot, Provider, SessionHandle, Solution, SolveRequest};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::info;

#[derive(Default)]
pub struct CloudflareBypass {
    client: Arc<RwLock<Option<Arc<Antibot>>>>,
    session: Arc<Mutex<Option<SessionHandle>>>,
}

impl CloudflareBypass {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn solve_get(&self, url: &str, existing_cookies: &str) -> Result<Solution> {
        let request = SolveRequest::get(url).with_cookies(parse_cookies(existing_cookies));
        self.execute(request).await
    }

    pub async fn solve_form(
        &self,
        url: &str,
        fields: HashMap<String, String>,
        existing_cookies: &str,
    ) -> Result<Solution> {
        let request = SolveRequest::post(url)
            .form(fields)
            .with_cookies(parse_cookies(existing_cookies));
        self.execute(request).await
    }

    pub async fn warm(&self, url: &str, existing_cookies: &str) -> Result<Solution> {
        let request = SolveRequest::get(url)
            .with_cookies(parse_cookies(existing_cookies))
            .return_only_cookies();
        self.execute(request).await
    }

    async fn execute(&self, request: SolveRequest) -> Result<Solution> {
        let client = self.get_or_create_client().await?;
        let mut session_slot = self.session.lock().await;
        let cookie_count = request.cookies.as_ref().map(|cookies| cookies.len()).unwrap_or(0);

        for attempt in 0..2 {
            if session_slot.is_none() {
                *session_slot = Some(client.create_session().await?);
                info!("Cloudflare bypass: FlareSolverr session created");
            }

            let session = session_slot.as_ref().expect("session exists");
            info!(
                "Cloudflare bypass: solving {} with {} pre-seeded cookies (attempt {})",
                request.url,
                cookie_count,
                attempt + 1
            );

            match session.execute(request.clone()).await {
                Ok(solution) => return Ok(solution),
                Err(err) if attempt == 0 => {
                    info!("Cloudflare bypass: session retry after error: {}", err);
                    session_slot.take();
                }
                Err(err) => return Err(err.into()),
            }
        }

        unreachable!("Cloudflare retry loop must return");
    }

    async fn get_or_create_client(&self) -> Result<Arc<Antibot>> {
        let client = self.client.read().await;
        if let Some(antibot) = &*client {
            return Ok(antibot.clone());
        }
        drop(client);

        let mut client = self.client.write().await;
        if let Some(antibot) = &*client {
            return Ok(antibot.clone());
        }

        let antibot = Arc::new(
            Antibot::builder()
                .provider(Provider::FlareSolverr)
                .auto_start(true)
                .build()
                .await?,
        );

        *client = Some(antibot.clone());
        info!("Cloudflare bypass: FlareSolverr container started");
        Ok(antibot)
    }
}

fn parse_cookies(header: &str) -> Vec<antibot_rs::Cookie> {
    header
        .split(';')
        .filter_map(|pair| {
            let pair = pair.trim();
            if pair.is_empty() {
                return None;
            }
            let (name, value) = pair.split_once('=')?;
            Some(antibot_rs::Cookie::new(
                name.trim().to_string(),
                value.trim().to_string(),
            ))
        })
        .collect()
}
