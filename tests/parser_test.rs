use unknowncheats_mcp::parser::{parse_forums, parse_posts, parse_threads};

#[test]
fn parses_forum_links() {
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
fn parses_thread_links() {
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
fn parses_posts() {
    // vBulletin uses <li class="postcontainer" id="post_123">
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
