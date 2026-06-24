use crate::{
    cloudflare::CloudflareBypass,
    config::{Config, ForumConfig},
    parser,
};
use anyhow::{bail, Result};
use reqwest::{
    cookie::Jar,
    header,
    multipart::{Form, Part},
    Client, StatusCode,
};
use std::{collections::HashMap, path::Path, process::Command, sync::Arc};
use tracing::info;
use url::Url;

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
    cookie_jar: Arc<Jar>,
    cf_bypass: Arc<CloudflareBypass>,
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
        self.unknowncheats().reply_to_thread(thread_id, body).await
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
        let html = self.client.get(&forum_path(forum_id)).await?;
        parser::parse_threads(&html, self.client.cfg.base_url.as_str())
    }

    pub async fn read_thread(&self, thread_id: &str) -> Result<Vec<parser::Post>> {
        let html = self.client.get(&thread_path(thread_id)).await?;
        parser::parse_posts(&html)
    }

    pub async fn get_profile(&self, user_id: &str) -> Result<String> {
        self.client.get(&profile_path(user_id)).await
    }

    pub async fn get_logged_in_user(&self) -> Result<String> {
        self.client.get("usercp.php").await
    }

    pub async fn user_cp(&self) -> Result<String> {
        self.client.get("usercp.php").await
    }

    pub async fn subscribed_threads(&self) -> Result<String> {
        self.client
            .get("subscription.php?do=viewsubscription")
            .await
    }

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
        let page = self.client.get(&thread_path(thread_id)).await?;
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

    pub async fn create_thread(&self, forum_id: &str, title: &str, body: &str) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        let page = self
            .client
            .get(&format!("newthread.php?do=newthread&f={}", forum_id))
            .await?;
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

    pub async fn send_private_message(&self, to: &str, title: &str, body: &str) -> Result<String> {
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
        let page = self
            .client
            .get(&format!("editpost.php?do=editpost&p={}", post_id))
            .await?;
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

    pub async fn delete_post(&self, post_id: &str, reason: &str) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        let page = self
            .client
            .get(&format!("editpost.php?do=deletepost&p={}", post_id))
            .await?;
        let token = parser::extract_security_token(&page);
        let form = &[
            ("do", "deletepost"),
            ("p", post_id),
            ("securitytoken", token.as_deref().unwrap_or("guest")),
            ("deletepost", "delete"),
            ("reason", reason),
        ];
        self.client
            .post_form("editpost.php?do=deletepost", form)
            .await
    }

    pub async fn report_post(&self, post_id: &str, reason: &str) -> Result<String> {
        if !self.enable_writes {
            bail!("Writes are disabled. Set UC_ENABLE_WRITES=true to enable.");
        }
        let page = self
            .client
            .get(&format!("report.php?p={}", post_id))
            .await?;
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
        let cookie_jar = Arc::new(Jar::default());
        seed_cookie_jar(&cookie_jar, &cfg.base_url, &cfg.cookie_header);

        let mut headers = header::HeaderMap::new();
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
            .cookie_provider(cookie_jar.clone())
            .build()?;

        Ok(Self {
            cfg,
            http,
            cookie_jar,
            cf_bypass: Arc::new(CloudflareBypass::new()),
            needs_cf_bypass,
        })
    }

    async fn get(&self, path: &str) -> Result<String> {
        let url = self.cfg.base_url.join(path)?;
        let resp = self.http.get(url.clone()).send().await?;
        let status = resp.status();
        let body = resp.text().await?;

        if self.is_challenge(status, &body) {
            info!("Cloudflare challenge detected for GET {}", url);
            match self
                .cf_bypass
                .solve_get(url.as_str(), &self.cfg.cookie_header)
                .await
            {
                Ok(solution) => {
                    self.store_solution_cookies(&url, &solution);
                    let html = solution.html().to_string();
                    if !html.is_empty() {
                        return Ok(html);
                    }
                    return self.retry_get(&url).await;
                }
                Err(err) => {
                    if let Some(html) = self.external_fetch(&url)? {
                        return Ok(html);
                    }
                    bail!(
                        "Cloudflare challenge detected and bypass failed: {err}. Refresh forum cookies including cf_clearance, or set UC_FETCH_CMD/FORUM_FETCH_CMD to a local stealth fetch command."
                    );
                }
            }
        }

        if !status.is_success() {
            bail!("HTTP {}: {}", status, body);
        }

        Ok(body)
    }

    async fn post_form(&self, path: &str, form: &[(&str, &str)]) -> Result<String> {
        let url = self.cfg.base_url.join(path)?;
        let resp = self.http.post(url.clone()).form(form).send().await?;
        let status = resp.status();
        let body = resp.text().await?;

        if self.is_challenge(status, &body) {
            info!("Cloudflare challenge detected for POST {}", url);
            let fields = form
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect();
            let solution = self
                .cf_bypass
                .solve_form(url.as_str(), fields, &self.cfg.cookie_header)
                .await?;
            self.store_solution_cookies(&url, &solution);
            let html = solution.html().to_string();
            if !html.is_empty() {
                return Ok(html);
            }
            return self.retry_post_form(&url, form).await;
        }

        if !status.is_success() {
            bail!("HTTP {}: {}", status, body);
        }

        Ok(body)
    }

    async fn post_form_owned(&self, path: &str, form: &HashMap<String, String>) -> Result<String> {
        let url = self.cfg.base_url.join(path)?;
        let resp = self.http.post(url.clone()).form(form).send().await?;
        let status = resp.status();
        let body = resp.text().await?;

        if self.is_challenge(status, &body) {
            info!("Cloudflare challenge detected for POST {}", url);
            let solution = self
                .cf_bypass
                .solve_form(url.as_str(), form.clone(), &self.cfg.cookie_header)
                .await?;
            self.store_solution_cookies(&url, &solution);
            let html = solution.html().to_string();
            if !html.is_empty() {
                return Ok(html);
            }
            return self.retry_post_form_owned(&url, form).await;
        }

        if !status.is_success() {
            bail!("HTTP {}: {}", status, body);
        }

        Ok(body)
    }

    async fn post_multipart_form(
        &self,
        path: &str,
        fields: &HashMap<String, String>,
        files: &[UploadFile],
    ) -> Result<String> {
        let url = self.cfg.base_url.join(path)?;
        let resp = self
            .http
            .post(url.clone())
            .multipart(build_multipart_form(fields, files).await?)
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;

        if self.is_challenge(status, &body) {
            info!("Cloudflare challenge detected for multipart POST {}", url);
            let warm = self
                .cf_bypass
                .warm(url.as_str(), &self.cfg.cookie_header)
                .await?;
            self.store_solution_cookies(&url, &warm);
            return self.retry_post_multipart(&url, fields, files).await;
        }

        if !status.is_success() {
            bail!("HTTP {}: {}", status, body);
        }

        Ok(body)
    }

    fn is_challenge(&self, status: StatusCode, body: &str) -> bool {
        self.needs_cf_bypass
            && ((status == StatusCode::FORBIDDEN && is_cloudflare_challenge(body))
                || (status.is_success() && is_cloudflare_challenge(body)))
    }

    async fn retry_get(&self, url: &Url) -> Result<String> {
        Ok(self
            .http
            .get(url.clone())
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    async fn retry_post_form(&self, url: &Url, form: &[(&str, &str)]) -> Result<String> {
        Ok(self
            .http
            .post(url.clone())
            .form(form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    async fn retry_post_form_owned(
        &self,
        url: &Url,
        form: &HashMap<String, String>,
    ) -> Result<String> {
        Ok(self
            .http
            .post(url.clone())
            .form(form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    async fn retry_post_multipart(
        &self,
        url: &Url,
        fields: &HashMap<String, String>,
        files: &[UploadFile],
    ) -> Result<String> {
        Ok(self
            .http
            .post(url.clone())
            .multipart(build_multipart_form(fields, files).await?)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    fn external_fetch(&self, url: &Url) -> Result<Option<String>> {
        let Some(cmd) = &self.cfg.fetch_cmd else {
            return Ok(None);
        };
        let output = Command::new(cmd)
            .arg(url.as_str())
            .env("FORUM_COOKIE", &self.cfg.cookie_header)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("external fetch command failed: {}", stderr.trim());
        }
        Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
    }

    fn store_solution_cookies(&self, url: &Url, solution: &antibot_rs::Solution) {
        for cookie in &solution.cookies {
            self.cookie_jar
                .add_cookie_str(&format!("{}={}", cookie.name, cookie.value), url);
        }
    }
}

fn seed_cookie_jar(cookie_jar: &Jar, base_url: &Url, cookie_header: &str) {
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if pair.is_empty() || !pair.contains('=') {
            continue;
        }
        cookie_jar.add_cookie_str(pair, base_url);
    }
}

async fn build_multipart_form(
    fields: &HashMap<String, String>,
    files: &[UploadFile],
) -> Result<Form> {
    let mut form = Form::new();

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
        let part = Part::bytes(bytes).file_name(filename);
        form = form.part(file.field.clone(), part);
    }

    Ok(form)
}

fn forum_path(forum_id: &str) -> String {
    resource_path_or(forum_id, |id| format!("forumdisplay.php?f={id}"))
}

fn thread_path(thread_id: &str) -> String {
    resource_path_or(thread_id, |id| format!("showthread.php?t={id}"))
}

fn profile_path(user_id: &str) -> String {
    resource_path_or(user_id, |id| format!("member.php?u={id}"))
}

fn resource_path_or(input: &str, fallback: impl FnOnce(&str) -> String) -> String {
    if looks_like_resource_path(input) {
        normalize_resource_path(input)
    } else {
        fallback(input)
    }
}

fn looks_like_resource_path(input: &str) -> bool {
    input.contains('/')
        || input.ends_with(".html")
        || input.starts_with("http://")
        || input.starts_with("https://")
}

fn normalize_resource_path(input: &str) -> String {
    if let Ok(url) = Url::parse(input) {
        let mut path = url.path().trim_start_matches('/').to_string();
        if let Some(stripped) = path.strip_prefix("forum/") {
            path = stripped.to_string();
        }
        if let Some(query) = url.query() {
            path.push('?');
            path.push_str(query);
        }
        return path;
    }

    input
        .trim_start_matches('/')
        .trim_start_matches("forum/")
        .to_string()
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

fn is_cloudflare_challenge(body: &str) -> bool {
    body.contains("cf-browser-verification")
        || body.contains("cf_clearance")
        || body.contains("Checking your browser")
        || body.contains("Just a moment")
        || body.contains("ddr-allowed-age-check")
        || body.contains("challenge-platform")
        || body.contains("ray id:")
        || body.contains("__cf_bm")
        || body.contains("cf-wrapper")
        || body.contains("cf-error-code")
        || body.contains("cf-cdn-container")
}
