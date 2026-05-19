use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc;
use std::time::Duration;

use aurelius::Server;

#[test]
fn local_markdown_links_redirect_to_live_preview() -> Result<(), Box<dyn Error>> {
    let test_dir = std::env::temp_dir().join(format!(
        "markdown-composer-navigation-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&test_dir)?;

    let linked_file = test_dir.join("linked.md");
    fs::write(&linked_file, "# Linked\n\nMarkdown content\n")?;

    let (tx, rx) = mpsc::channel();
    let mut server = Server::bind("localhost:0")?;
    server.set_static_root(&test_dir);
    server.set_markdown_navigation_channel(tx);

    let mut conn = TcpStream::connect(server.addr())?;
    write!(conn, "GET /linked.md HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
    conn.flush()?;

    let mut response = String::new();
    conn.read_to_string(&mut response)?;

    assert!(response.starts_with("HTTP/1.1 303 See Other"));
    assert!(response.contains("\r\nLocation: /\r\n"));
    assert_eq!(rx.recv_timeout(Duration::from_secs(1))?, linked_file);

    fs::remove_dir_all(test_dir)?;

    Ok(())
}
