use maplit::hashmap;
use std::{collections::HashMap, path::PathBuf};
use unknowncheats_mcp::config::Config;

#[test]
fn loads_required_cookie_and_defaults() {
    let cfg = Config::from_env_map(hashmap! {
        "UC_COOKIE".to_string() => "bbsessionhash=secret; darktheme_enabled=1".to_string(),
    })
    .unwrap();

    assert_eq!(
        cfg.unknowncheats.base_url.as_str(),
        "https://www.unknowncheats.me/forum/"
    );
    assert_eq!(
        cfg.unknowncheats.cookie_header,
        "bbsessionhash=secret; darktheme_enabled=1"
    );
    assert_eq!(
        cfg.elitepvpers.base_url.as_str(),
        "https://www.elitepvpers.com/forum/"
    );
    assert!(!cfg.enable_writes);
}

#[test]
fn loads_elitepvpers_cookie() {
    let cfg = Config::from_env_map(hashmap! {
        "UC_COOKIE".to_string() => "a=b".to_string(),
        "EP_COOKIE".to_string() => "bbsessionhash=ep-secret; vbseo_loggedin=yes".to_string(),
        "EP_USERNAME".to_string() => "example-user".to_string(),
    })
    .unwrap();

    assert_eq!(
        cfg.elitepvpers.cookie_header,
        "bbsessionhash=ep-secret; vbseo_loggedin=yes"
    );
    assert_eq!(cfg.elitepvpers.username.as_deref(), Some("example-user"));
}

#[test]
fn parses_write_gate_bool() {
    let cfg = Config::from_env_map(hashmap! {
        "UC_COOKIE".to_string() => "a=b".to_string(),
        "UC_ENABLE_WRITES".to_string() => "true".to_string(),
    })
    .unwrap();

    assert!(cfg.enable_writes);
}

#[test]
fn redacts_cookies_from_debug_output() {
    let cfg = Config::from_env_map(hashmap! {
        "UC_COOKIE".to_string() => "bbsessionhash=super-secret".to_string(),
        "EP_COOKIE".to_string() => "bbsessionhash=ep-super-secret".to_string(),
    })
    .unwrap();

    let rendered = format!("{cfg:?}");
    assert!(!rendered.contains("super-secret"));
    assert!(!rendered.contains("ep-super-secret"));
    assert!(rendered.contains("<redacted>"));
}

#[test]
fn from_env_file_accepts_raw_cookie_semicolons() {
    let path = std::env::temp_dir().join("unknowncheats-mcp-raw-cookie.env");
    std::fs::write(
        &path,
        "UC_COOKIE=bbsessionhash=secret; darktheme_enabled=1; bbuserid=1\nUC_ENABLE_WRITES=false\n",
    )
    .unwrap();

    let cfg = Config::from_env_file(&path).unwrap();

    assert_eq!(
        cfg.unknowncheats.cookie_header,
        "bbsessionhash=secret; darktheme_enabled=1; bbuserid=1"
    );

    std::fs::remove_file(path).unwrap();
}

#[test]
fn env_example_is_parseable_and_contains_required_keys() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env.example");
    let vars: HashMap<String, String> = dotenvy::from_path_iter(path)
        .unwrap()
        .map(|item| item.unwrap())
        .collect();

    assert_eq!(
        vars.get("UC_BASE_URL").map(String::as_str),
        Some("https://www.unknowncheats.me/forum/")
    );
    assert_eq!(
        vars.get("EP_BASE_URL").map(String::as_str),
        Some("https://www.elitepvpers.com/forum/")
    );
    assert!(vars.contains_key("UC_COOKIE"));
    assert!(vars.contains_key("EP_COOKIE"));
    assert_eq!(
        vars.get("UC_ENABLE_WRITES").map(String::as_str),
        Some("false")
    );
}
