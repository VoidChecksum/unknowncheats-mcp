use crate::{
    cloudflare::CloudflareBypass,
    config::{Config, ForumConfig},
    parser,
};
use anyhow::{bail, Result};
use reqwest::{header, multipart, Client, StatusCode};
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Clone)]
pub struct ForumClient {
    unknowncheats: VBulletinClient,
    elitepvpers: VBulletinClient,
    enable_writes: bool,
}

#[derive(Clone)]
pub struct VBulletinClient {
    cfg: ForumConfig,
    http: Client,
    /// Cloudflare bypass for sites that need it
    cf_bypass: Arc<RwLock<Option<CloudflareBypass>>>,
    /// Whether this client needs CF bypass
    needs_cf_bypass: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadFile {
    pub field: String,
    pub path: String,
}

impl ForumClient {
    pub fn new(cfg: Config) -> Result<Self> {
        Ok(Self {
            // Both UC and EPVP now use Cloudflare
            unknowncheats: VBulletinClient::new(cfg.unknowncheats, true)?,
            elitepvpers: VBulletinClient::new(cfg.elitepvpers, true)?,
            enable_writes: cfg.enable_writes,
        })
    }

    pub fn unknowncheats(&self) -> ForumHandle<'_> {
        ForumHandle::new(&self.unknowncheats, self.enable_writes)
    }

    pub fn elitepvpers(&self) -> ForumHandle<'_> {
        ForumHandle::new(&self.elitepvpers, self.enable_writes)
    }

    pub async fn list_forums(&self) -> Result<Vec<parser::Forum>> {
        self.unknowncheats().list_forums().await
    }
    pub async fn search_forum(&self, query: &str) -> Result<Vec<parser::Thread>> {
        self.unknowncheats().search_forum(query).await
    }
    pub async fn list_threads(&self, forum_id: &str) -> Result<Vec<parser::Thread>> {
        self.unknowncheats().list_threads(forum_id).await
    }
    pub async fn read_thread(&self, thread_id: &str) -> Result<Vec<parser::Post>> {
        self.unknowncheats().read_thread(thread_id).await
    }
    pub async fn get_profile(&self, user_id: &str) -> Result<String> {
        self.unknowncheats().get_profile(user_id).await
    }
    pub async fn get_logged_in_user(&self) -> Result<String> {
        self.unknowncheats().get_logged_in_user().await
    }
    pub async fn list_private_messages(&self) -> Result<String> {
        self.unknowncheats().list_private_messages().await
    }
    pub async fn reply_to_thread(&self, thread_id: &str, body: &str) -> Result<String> {
        self.unknowncheats()
            .reply_to_thread(thread_id, body)
            .await
    }
    pub async fn create_thread(&self, forum_id: &str, title: &str, body: &str) -> Result<String> {
        self.unknowncheats()
            .create_thread(forum_id, title, body)
            .await
    }
    pub async fn send_private_message(&self, to: &str, title: &str, body: &str) -> Result<String> {
        self.unknowncheats()
            .send_private_message(to, title, body)
            .await
    }
}

pub struct ForumHandle<'a> {
    client: &'a VBulletinClient,
    enable_writes: bool,
}

impl<'a> ForumHandle<'a> {
    fn new(client: &'a VBulletinClient, enable_writes: bool) -> Self {
        Self {
            client,
            enable_writes,
        }
    }

    pub async fn list_forums(&self) -> Result<Vec<parser::Forum>> {
        let html = self.client.get("index.php").await?;
        parser::parse_forums(&html, self.client.cfg.base_url.as_str())
    }

    pub async fn search_forum(&self, query: &str) -> Result<Vec<parser::Thread>> {
        let html = self
            .client
            .get(&format!(
                "search.php?do=process&query={}",
                urlencoding(query)
            ))
            .await?;
        parser::parse_search(&html, self.client.cfg.base_url.as_str())
    }

    pub async fn list_threads(&self, forum_id: &str) -> Result<Vec<parser::Thread>> {
        let html = self
            .client
            .get(&format!("forumdisplay.php?f={}", forum_id))
            .await?;
        parser::parse_threads(&html, self.client.cfg.base_url.as_str())
    }

    pub async fn read_thread(&self, thread_id: &str) -> Result<Vec<parser::Post>> {
        let html = self
            .client
            .get(&format!("showthread.php?t={}", thread_id))
            .await?;
        parser::parse_posts(&html)
    }

    pub async fn get_profile(&self, user_id: &str) -> Result<String> {
        self.client
            .get(&format!("member.php?u={}", user_id))
            .await
    }

    pub async fn get_logged_in_user(&self) -> Result<String> {
        self.client.get("usercp.php").await
    }

    /// User control panel (same as get_logged_in_user)
    pub async fn user_cp(&self) -> Result<String> {
        self.client.get("usercp.php").await
    }

    /// List subscribed threads
    pub async fn subscribed_threads(&self) -> Result<String> {
        self.client
            .get("subscription.php?do=viewsubscription")
            .await
    }

    /// List attachments
    pub async fn attachments(&self) -> Result<String> {
        self.client.get("profile.php?do=editattachments").await
    }

    pub async fn list_private_messages(&self) -> Result<String> {
        self.client.get("private.php").await
    }

    pub async fn reply_to_thread(&self, thread_id: &str, body: &str) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        let thread_url = format!("showthread.php?t={}", thread_id);
        let page = self.client.get(&thread_url).await?;
        let token = parser::extract_security_token(&page);
        let form = &[
            ("do", "postreply"),
            ("t", thread_id),
            ("p", ""),
            ("securitytoken", token.as_deref().unwrap_or("guest")),
            ("message", body),
            ("sbutton", "Submit Reply"),
        ];
        self.client
            .post_form("newreply.php?do=postreply&t=", form)
            .await
    }

    pub async fn create_thread(
        &self,
        forum_id: &str,
        title: &str,
        body: &str,
    ) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        let newthread_url = format!("newthread.php?do=newthread&f={}", forum_id);
        let page = self.client.get(&newthread_url).await?;
        let token = parser::extract_security_token(&page);
        let form = &[
            ("do", "postthread"),
            ("f", forum_id),
            ("securitytoken", token.as_deref().unwrap_or("guest")),
            ("subject", title),
            ("message", body),
            ("sbutton", "Submit New Thread"),
        ];
        self.client
            .post_form("newthread.php?do=postthread&f=", form)
            .await
    }

    pub async fn send_private_message(
        &self,
        to: &str,
        title: &str,
        body: &str,
    ) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        let page = self.client.get("private.php?do=newpm").await?;
        let token = parser::extract_security_token(&page);
        let form = &[
            ("do", "insertpm"),
            ("securitytoken", token.as_deref().unwrap_or("guest")),
            ("recipients", to),
            ("title", title),
            ("message", body),
            ("sbutton", "Submit Message"),
        ];
        self.client.post_form("private.php?do=insertpm", form).await
    }

    pub async fn edit_post(&self, post_id: &str, body: &str) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        let edit_url = format!("editpost.php?do=editpost&p={}", post_id);
        let page = self.client.get(&edit_url).await?;
        let token = parser::extract_security_token(&page);
        let form = &[
            ("do", "updatepost"),
            ("p", post_id),
            ("securitytoken", token.as_deref().unwrap_or("guest")),
            ("message", body),
            ("sbutton", "Save Changes"),
        ];
        self.client
            .post_form("editpost.php?do=updatepost&pn=", form)
            .await
    }

    pub async fn delete_post(&self, post_id: &str, _reason: &str) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        let delete_url = format!("editpost.php?do=deletepost&p={}", post_id);
        let page = self.client.get(&delete_url).await?;
        let token = parser::extract_security_token(&page);
        let form = &[
            ("do", "deletepost"),
            ("p", post_id),
            ("securitytoken", token.as_deref().unwrap_or("guest")),
            ("deletepost", "delete"),
            ("reason", ""),
        ];
        self.client
            .post_form("editpost.php?do=deletepost", form)
            .await
    }

    pub async fn report_post(&self, post_id: &str, reason: &str) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        let report_url = format!("report.php?p={}", post_id);
        let page = self.client.get(&report_url).await?;
        let token = parser::extract_security_token(&page);
        let form = &[
            ("do", "sendemail"),
            ("p", post_id),
            ("securitytoken", token.as_deref().unwrap_or("guest")),
            ("reason", reason),
            ("sbutton", "Send Report"),
        ];
        self.client.post_form("report.php?do=sendemail", form).await
    }

    pub async fn get_page(&self, path: &str) -> Result<String> {
        self.client.get(path).await
    }

    pub async fn submit_form(
        &self,
        path: &str,
        fields: &HashMap<String, String>,
    ) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        self.client.post_form_owned(path, fields).await
    }

    pub async fn submit_multipart_form(
        &self,
        path: &str,
        fields: &HashMap<String, String>,
        files: &[UploadFile],
    ) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        self.client.post_multipart_form(path, fields, files).await
    }
}

impl VBulletinClient {
    fn new(cfg: ForumConfig, needs_cf_bypass: bool) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        if !cfg.cookie_header.trim().is_empty() {
            headers.insert(
                header::COOKIE,
                header::HeaderValue::from_str(&cfg.cookie_header)?,
            );
        }
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(
            header::ACCEPT_LANGUAGE,
            header::HeaderValue::from_static("en-US,en;q=0.9"),
        );
        let http = Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .build()?;
        Ok(Self {
            cfg,
            http,
            cf_bypass: Arc::new(RwLock::new(if needs_cf_bypass {
                Some(CloudflareBypass::new())
            } else {
                None
            })),
            needs_cf_bypass,
        })
    }

    async fn get(&self, path: &str) -> Result<String> {
        let url = self.cfg.base_url.join(path)?;
        let resp = self.http.get(url.clone()).send().await?;

        // Check for Cloudflare challenge (403 with specific HTML)
        if resp.status() == StatusCode::FORBIDDEN {
            let body = resp.text().await?;
            if is_cloudflare_challenge(&body) {
                if self.needs_cf_bypass {
                    info!("Cloudflare challenge detected, bypassing via Byparr");
                    return self.bypass_cloudflare(&url).await;
                } else {
                    warn!("Cloudflare challenge detected but bypass not enabled for this client");
                }
            }
            bail!("403 Forbidden: {}", body);
        }

        // Check for Cloudflare in 200 response (JavaScript challenge)
        let status = resp.status();
        let body = resp.text().await?;
        if status.is_success() && is_cloudflare_challenge(&body) && self.needs_cf_bypass {
            info!("Cloudflare JS challenge detected in 200 response, bypassing");
            return self.bypass_cloudflare(&url).await;
        }

        Ok(body)
    }

    async fn post_form(&self, path: &str, form: &[(&str, &str)]) -> Result<String> {
        let url = self.cfg.base_url.join(path)?;
        Ok(self
            .http
            .post(url)
            .form(form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    async fn post_form_owned(&self, path: &str, form: &HashMap<String, String>) -> Result<String> {
        let url = self.cfg.base_url.join(path)?;
        Ok(self
            .http
            .post(url)
            .form(form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    async fn post_multipart_form(
        &self,
        path: &str,
        fields: &HashMap<String, String>,
        files: &[UploadFile],
    ) -> Result<String> {
        let url = self.cfg.base_url.join(path)?;
        let mut form = multipart::Form::new();

        for (key, value) in fields {
            form = form.text(key.clone(), value.clone());
        }

        for file in files {
            let bytes = tokio::fs::read(&file.path).await?;
            let filename = Path::new(&file.path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("upload.bin")
                .to_string();
            let part = multipart::Part::bytes(bytes).file_name(filename);
            form = form.part(file.field.clone(), part);
        }

        Ok(self
            .http
            .post(url)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    async fn bypass_cloudflare(&self, url: &url::Url) -> Result<String> {
        let mut bypass = self.cf_bypass.write().await;
        let bypass = bypass
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Cloudflare bypass not configured for this client"))?;

        let (html, _cookies) = bypass.solve(url.as_str(), &self.cfg.cookie_header).await?;
        Ok(html)
    }
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

/// Check if response body is a Cloudflare challenge page
fn is_cloudflare_challenge(body: &str) -> bool {
    // Cloudflare challenge patterns
    body.contains("cf-browser-verification")
        || body.contains("cf_clearance")
        || body.contains("Checking your browser")
        || body.contains("Just a moment")
        || body.contains("ddr-allowed-age-check")
        || body.contains("challenge-platform")
        || body.contains("ray id:")
        || body.contains("__cf_bm")
        || (body.contains("403 Forbidden") && body.contains("cloudflare"))
        // Additional Cloudflare bot detection
        || body.contains("cf-wrapper")
        || body.contains("cf-error-code")
        || body.contains("cf-cdn-container")
}