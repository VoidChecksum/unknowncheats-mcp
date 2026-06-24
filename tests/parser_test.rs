use unknowncheats_mcp::parser::{parse_forums, parse_posts, parse_threads};

#[test]
fn parses_query_forum_links() {
    let html = r#"
      <a href="forumdisplay.php?f=1">Game Hacking</a>
      <a href="forumdisplay.php?f=2">Anti-Cheat Bypass</a>
    "#;

    let forums = parse_forums(html, "https://www.unknowncheats.me/forum/").unwrap();

    assert_eq!(forums.len(), 2);
    assert_eq!(forums[0].id, "1");
    assert_eq!(forums[0].title, "Game Hacking");
    assert_eq!(
        forums[0].url,
        "https://www.unknowncheats.me/forum/forumdisplay.php?f=1"
    );
}

#[test]
fn parses_seo_forum_links() {
    let html = r#"
      <a href="https://www.unknowncheats.me/forum/valorant/"><strong>Valorant</strong></a>
      <a href="other-games/"><strong>Other Games</strong></a>
    "#;

    let forums = parse_forums(html, "https://www.unknowncheats.me/forum/").unwrap();

    assert_eq!(forums.len(), 2);
    assert_eq!(forums[0].id, "valorant/");
    assert_eq!(forums[0].title, "Valorant");
    assert_eq!(forums[0].url, "https://www.unknowncheats.me/forum/valorant/");
    assert_eq!(forums[1].id, "other-games/");
}

#[test]
fn parses_query_thread_links() {
    let html = r#"
      <a href="showthread.php?t=42">Useful Thread</a>
      <a href="showthread.php?t=43&page=2">Paged Thread</a>
    "#;

    let threads = parse_threads(html, "https://www.unknowncheats.me/forum/").unwrap();

    assert_eq!(threads.len(), 2);
    assert_eq!(threads[0].id, "42");
    assert_eq!(threads[0].title, "Useful Thread");
}

#[test]
fn parses_seo_thread_links() {
    let html = r#"
      <a href="other-games/758547-tbh-persistent-reward-item-generator.html">TBH Persistent Reward Item Generator</a>
      <a href="https://www.elitepvpers.com/forum/valorant/5249674-free-tracker-gg-profile-view-booster.html">[FREE] Tracker.gg Profile Booster</a>
    "#;

    let threads = parse_threads(html, "https://www.unknowncheats.me/forum/").unwrap();

    assert_eq!(threads.len(), 2);
    assert_eq!(threads[0].id, "other-games/758547-tbh-persistent-reward-item-generator.html");
    assert_eq!(threads[1].id, "valorant/5249674-free-tracker-gg-profile-view-booster.html");
}

#[test]
fn parses_posts() {
    let html = r#"
      <li class="postcontainer" id="post_10">
        <div class="postbody">
          <div class="content">Hello <b>forum</b></div>
        </div>
      </li>
      <li class="postcontainer" id="post_11">
        <div class="postbody">
          <div class="content">Second post</div>
        </div>
      </li>
    "#;

    let posts = parse_posts(html).unwrap();

    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].id, "post_10");
    assert_eq!(posts[0].body, "Hello forum");
}
