use anyhow::Result;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Forum {
    pub id: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Thread {
    pub id: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Post {
    pub id: String,
    pub body: String,
}

pub fn parse_forums(html: &str, base_url: &str) -> Result<Vec<Forum>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();
    let base = Url::parse(base_url)?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for element in document.select(&selector) {
        let Some(href) = element.value().attr("href") else {
            continue;
        };
        let title = clean_text(&element.text().collect::<String>());
        if title.is_empty() {
            continue;
        }

        if href.contains("forumdisplay.php?f=") {
            let url = base.join(href)?;
            if let Some(id) = query_id(url.query().unwrap_or(""), "f") {
                if seen.insert(id.clone()) {
                    out.push(Forum { id, title, url: url.to_string() });
                }
            }
            continue;
        }

        let url = base.join(href)?;
        if let Some(id) = seo_forum_id(&url, &base, &element) {
            if seen.insert(id.clone()) {
                out.push(Forum { id, title, url: url.to_string() });
            }
        }
    }

    Ok(out)
}

pub fn parse_search(html: &str, base_url: &str) -> Result<Vec<Thread>> {
    parse_threads(html, base_url)
}

pub fn extract_security_token(html: &str) -> Option<String> {
    let re = Regex::new(r#"name="securitytoken"[^>]*value="([^"]+)""#).ok()?;
    re.captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

pub fn parse_threads(html: &str, base_url: &str) -> Result<Vec<Thread>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();
    let base = Url::parse(base_url)?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for element in document.select(&selector) {
        let Some(href) = element.value().attr("href") else {
            continue;
        };
        let title = clean_text(&element.text().collect::<String>());
        if title.is_empty() {
            continue;
        }

        if href.contains("showthread.php?t=") {
            let url = base.join(href)?;
            if let Some(id) = query_id(url.query().unwrap_or(""), "t") {
                if seen.insert(id.clone()) {
                    out.push(Thread { id, title, url: url.to_string() });
                }
            }
            continue;
        }

        let url = base.join(href)?;
        if let Some(id) = seo_thread_id(&url, &base) {
            if seen.insert(id.clone()) {
                out.push(Thread { id, title, url: url.to_string() });
            }
        }
    }

    Ok(out)
}

pub fn parse_posts(html: &str) -> Result<Vec<Post>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("li.postcontainer").unwrap();
    let text_selector = Selector::parse(".postbody .content").unwrap();

    let mut posts = Vec::new();
    for element in document.select(&selector) {
        let id = element.value().id().map(str::to_string).unwrap_or_default();
        let body = element
            .select(&text_selector)
            .next()
            .map(|e| clean_text(&e.text().collect::<String>()))
            .unwrap_or_default();
        posts.push(Post { id, body });
    }

    Ok(posts)
}

fn seo_forum_id(url: &Url, base: &Url, element: &ElementRef<'_>) -> Option<String> {
    let rel = normalize_relative_path(url, base)?;
    if rel.is_empty() || !rel.ends_with('/') || rel.contains('.') || rel.contains('?') {
        return None;
    }
    let strong_selector = Selector::parse("strong").unwrap();

    element.select(&strong_selector).next()?;

    Some(rel)
}

fn seo_thread_id(url: &Url, base: &Url) -> Option<String> {
    let rel = normalize_relative_path(url, base)?;
    let thread_re = Regex::new(r"^[^/]+/\d+-.+\.html$").ok()?;
    if rel.ends_with("-new-post.html") || rel.contains("-post") || !thread_re.is_match(&rel) {
        return None;
    }

    Some(rel)
}

fn normalize_relative_path(url: &Url, base: &Url) -> Option<String> {
    let base_prefix = base.path().trim_start_matches('/').trim_end_matches('/');
    let mut path = url.path().trim_start_matches('/');
    if !base_prefix.is_empty() {
        let prefix = format!("{base_prefix}/");
        path = path.strip_prefix(&prefix).unwrap_or(path);
    }

    if path.is_empty() {
        return None;
    }

    let mut rel = path.to_string();
    if let Some(query) = url.query() {
        rel.push('?');
        rel.push_str(query);
    }
    Some(rel)
}

fn query_id(query: &str, key: &str) -> Option<String> {
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

fn clean_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
