#![cfg(feature = "json-rpc")]

use std::error::Error;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn vim_follows_multiple_markdown_links() -> Result<(), Box<dyn Error>> {
    if Command::new("vim").arg("--version").output().is_err()
        || Command::new("curl").arg("--version").output().is_err()
    {
        return Ok(());
    }

    let test_dir = std::env::temp_dir().join(format!(
        "markdown-composer-vim-navigation-test-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::create_dir_all(&test_dir)?;

    let a = test_dir.join("a.md");
    let b = test_dir.join("b.md");
    let c = test_dir.join("c.md");
    fs::write(&a, "# A\n\n[B](b.md)\n")?;
    fs::write(&b, "# B\n\n[C](c.md)\n")?;
    fs::write(&c, "# C\n\nfinal\n")?;

    let port = free_port()?;
    let result = test_dir.join("result.txt");
    let messages = test_dir.join("messages.txt");
    let vimrc = test_dir.join("vimrc");
    let script = test_dir.join("test.vim");
    let vimerr = test_dir.join("vimerr.txt");
    let plugin_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binary = assert_cmd::cargo::cargo_bin(env!("CARGO_PKG_NAME"));

    fs::write(
        &vimrc,
        format!(
            "\
execute 'set runtimepath^=' . fnameescape({plugin_root})
execute 'set runtimepath+=' . fnameescape({plugin_after})
syntax enable
filetype plugin indent on
let g:markdown_composer_open_browser = 0
let g:markdown_composer_port = {port}
let g:markdown_composer_binary = {binary}
",
            plugin_root = vim_string(&plugin_root),
            plugin_after = vim_string(plugin_root.join("after")),
            port = port,
            binary = vim_string(binary)
        ),
    )?;

    fs::write(
        &script,
        format!(
            "\
sleep 1
call writefile(['start=' . expand('%:p')], {result}, 'a')
call system('curl -s ' . shellescape('http://localhost:{port}' . expand('%:p:h') . '/b.md') . ' >/dev/null')
sleep 1
call writefile(['after_b=' . expand('%:p')], {result}, 'a')
call system('curl -s ' . shellescape('http://localhost:{port}' . expand('%:p:h') . '/c.md') . ' >/dev/null')
sleep 1
call writefile(['after_c=' . expand('%:p')], {result}, 'a')
redir! > {messages_raw}
silent messages
redir END
qa!
",
            result = vim_string(&result),
            messages_raw = messages.display(),
            port = port
        ),
    )?;

    let output = Command::new("vim")
        .arg("-Nu")
        .arg(&vimrc)
        .arg("-n")
        .arg("-Es")
        .arg(&a)
        .arg("-S")
        .arg(&script)
        .output()?;
    fs::write(&vimerr, &output.stderr)?;

    let actual = fs::read_to_string(&result).unwrap_or_default();
    let expected = format!(
        "start={}\nafter_b={}\nafter_c={}\n",
        fs::canonicalize(&a)?.display(),
        fs::canonicalize(&b)?.display(),
        fs::canonicalize(&c)?.display()
    );

    assert_eq!(
        actual,
        expected,
        "vim stderr:\n{}\nvim messages:\n{}",
        String::from_utf8_lossy(&output.stderr),
        fs::read_to_string(&messages).unwrap_or_default()
    );

    fs::remove_dir_all(test_dir)?;

    Ok(())
}

fn free_port() -> Result<u16, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn vim_string(path: impl AsRef<Path>) -> String {
    format!("'{}'", path.as_ref().to_string_lossy().replace('\'', "''"))
}
