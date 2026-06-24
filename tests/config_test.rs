use maplit::hashmap;
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
