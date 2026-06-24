use anyhow::Result;
use regex::Regex;
use scraper::{Html, Selector};
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
    parse_links(html, base_url, "forumdisplay.php?f=", "f").map(|items| {
        items
            .into_iter()
            .map(|(id, title, url)| Forum { id, title, url })
            .collect()
    })
}

/// Parse search results (vBulletin search.php)
pub fn parse_search(html: &str, base_url: &str) -> Result<Vec<Thread>> {
    parse_links(html, base_url, "showthread.php?t=", "t").map(|items| {
        items
            .into_iter()
            .map(|(id, title, url)| Thread { id, title, url })
            .collect()
    })
}

/// Extract vBulletin security token from a page
pub fn extract_security_token(html: &str) -> Option<String> {
    // Look for <input type="hidden" name="securitytoken" value="...">
    let re = Regex::new(r#"name="securitytoken"[^>]*value="([^"]+)""#).ok()?;
    re.captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

pub fn parse_threads(html: &str, base_url: &str) -> Result<Vec<Thread>> {
    parse_links(html, base_url, "showthread.php?t=", "t").map(|items| {
        items
            .into_iter()
            .map(|(id, title, url)| Thread { id, title, url })
            .collect()
    })
}

pub fn parse_posts(html: &str) -> Result<Vec<Post>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("li.postcontainer").unwrap();
    let text_selector = Selector::parse(".postbody .content").unwrap();

    let mut posts = Vec::new();
    for element in document.select(&selector) {
        // Get id from the li element itself
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

fn parse_links(
    html: &str,
    base_url: &str,
    needle: &str,
    key: &str,
) -> Result<Vec<(String, String, String)>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();
    let base = Url::parse(base_url)?;

    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if !href.contains(needle) {
                continue;
            }
            let url = base.join(href)?;
            let query = url.query();
            if let Some(id) = query_id(query.unwrap_or(""), key) {
                if seen.insert(id.clone()) {
                    let title = clean_text(&element.text().collect::<String>());
                    out.push((id, title, url.to_string()));
                }
            }
        }
    }

    Ok(out)
}

fn query_id(query: &str, key: &str) -> Option<String> {
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

fn clean_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
