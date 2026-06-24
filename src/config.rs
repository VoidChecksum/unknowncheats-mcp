use anyhow::{Context, Result, bail};
use std::{collections::HashMap, fmt, path::Path};
use url::Url;

#[derive(Clone)]
pub struct Config {
    pub unknowncheats: ForumConfig,
    pub elitepvpers: ForumConfig,
    pub enable_writes: bool,
}

#[derive(Clone)]
pub struct ForumConfig {
    pub base_url: Url,
    pub cookie_header: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        if Path::new(".env").exists() {
            dotenvy::from_filename_override(".env")
                .context("failed to load .env from current directory")?;
        }
        Self::from_env_map(std::env::vars().collect())
    }

    pub fn from_env_map(env: HashMap<String, String>) -> Result<Self> {
        let unknowncheats =
            ForumConfig::from_env_map(&env, "UC", "https://www.unknowncheats.me/forum/", true)?;
        let elitepvpers =
            ForumConfig::from_env_map(&env, "EP", "https://www.elitepvpers.com/forum/", false)?;
        let enable_writes = env
            .get("UC_ENABLE_WRITES")
            .or_else(|| env.get("ENABLE_WRITES"))
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Ok(Self {
            unknowncheats,
            elitepvpers,
            enable_writes,
        })
    }
}

impl ForumConfig {
    fn from_env_map(
        env: &HashMap<String, String>,
        prefix: &str,
        default_base_url: &str,
        required: bool,
    ) -> Result<Self> {
        let cookie_key = format!("{prefix}_COOKIE");
        let cookie_header = env.get(&cookie_key).cloned().unwrap_or_default();
        if required && cookie_header.trim().is_empty() {
            bail!("{cookie_key} is required");
        }

        let base_url_key = format!("{prefix}_BASE_URL");
        let base_url = env
            .get(&base_url_key)
            .map(String::as_str)
            .unwrap_or(default_base_url);

        Ok(Self {
            base_url: Url::parse(base_url)?,
            cookie_header,
            username: env.get(&format!("{prefix}_USERNAME")).cloned(),
            password: env.get(&format!("{prefix}_PASSWORD")).cloned(),
        })
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("unknowncheats", &self.unknowncheats)
            .field("elitepvpers", &self.elitepvpers)
            .field("enable_writes", &self.enable_writes)
            .finish()
    }
}

impl fmt::Debug for ForumConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForumConfig")
            .field("base_url", &self.base_url)
            .field("cookie_header", &"<redacted>")
            .field("username", &self.username.as_ref().map(|_| "<redacted>"))
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}
