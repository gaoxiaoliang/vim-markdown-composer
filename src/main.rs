//! A simple client that listens for RPC requests and renders them as markdown.
//!
//! The markdown is rendered on an arbitrary port on localhost, which is then automatically opened
//! in a browser. As new messages are received through stdin, the markdown is asynchronously
//! rendered in the browser (no refresh is required).

use std::default::Default;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::mem;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use clap::{crate_authors, crate_version, Arg, Command};
use log::*;

use aurelius::Server;
use serde::Deserialize;
use shlex::Shlex;

static ABOUT: &str = r"
Creates a static server for serving markdown previews. Reads RPC requests from stdin.

Supported procedures:

    send_data(data: String)     Pushes a markdown string to the rendering server.
    open_browser()              Opens the user default browser, or the browser specified by
                                `--browser`.
    chdir(path: String)         Changes the directory that the server serves static files from.
";

/// Represents an RPC request.
///
/// Assumes that the request's parameters are always `String`s.
#[derive(Debug)]
pub struct Rpc {
    /// The type of msgpack request. Should always be notification.
    #[cfg(feature = "msgpack")]
    msg_type: u64,

    /// The ID of the JSON rpc request.
    #[cfg(feature = "json-rpc")]
    id: u64,

    pub method: String,
    pub params: Vec<String>,
}

#[cfg(feature = "msgpack")]
impl<'de> Deserialize<'de> for Rpc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Unexpected};

        const NOTIFICATION_MESSAGE_TYPE: u64 = 2;

        let (msg_type, method, params) = <(u64, String, Vec<String>)>::deserialize(deserializer)?;

        debug!("<- [{}, {}, {:?}]", msg_type, method, params);

        if msg_type != NOTIFICATION_MESSAGE_TYPE {
            return Err(Error::invalid_value(
                Unexpected::Unsigned(msg_type),
                &format!("notification message type ({})", NOTIFICATION_MESSAGE_TYPE).as_str(),
            ));
        }

        Ok(Rpc {
            msg_type,
            method,
            params,
        })
    }
}

// FIXME: Workaround for rust-lang/rust#55779. Move back to the impl when fixed.
#[derive(Debug, Deserialize)]
#[allow(unused)]
struct InnerRpc {
    method: String,
    params: Vec<String>,
}

#[cfg(feature = "json-rpc")]
impl<'de> Deserialize<'de> for Rpc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (id, rpc): (u64, InnerRpc) = Deserialize::deserialize(deserializer)?;

        debug!("<- [{}, {:?}]", id, rpc);

        Ok(Rpc {
            id: id,
            method: rpc.method,
            params: rpc.params,
        })
    }
}

fn read_rpc(reader: impl Read, mut server: Server, browser: Option<&str>) -> Result<()> {
    #[cfg(feature = "msgpack")]
    let mut deserializer = rmp_serde::Deserializer::new(std::io::BufReader::new(reader));

    #[cfg(feature = "json-rpc")]
    let mut deserializer = serde_json::Deserializer::new(serde_json::de::IoRead::new(reader));

    loop {
        let mut rpc = match Rpc::deserialize(&mut deserializer) {
            Ok(rpc) => rpc,
            #[cfg(feature = "msgpack")]
            Err(rmp_serde::decode::Error::InvalidMarkerRead(_)) => {
                // In this case, the remote client probably just hung up.
                break;
            }
            #[cfg(feature = "json-rpc")]
            Err(err) if err.is_eof() => {
                break;
            }
            Err(err) => panic!("{}", err),
        };

        let res = match &rpc.method[..] {
            "send_data" => {
                let markdown = mem::replace(&mut rpc.params[0], String::new());
                server.send(markdown)
            }
            "open_browser" => match browser {
                Some(browser) => server.open_specific_browser(ProcessCommand::new(browser)),
                None => server.open_browser(),
            },
            "chdir" => {
                let cwd = &rpc.params[0];
                info!("changing working directory: {}", cwd);
                server.set_static_root(cwd);
                Ok(())
            }
            method => panic!("Received unknown command: {}", method),
        };

        // TODO: Return error to the client instead of exiting the process.
        res?;
    }

    Ok(())
}

fn main() -> Result<()> {
    log_panics::init();
    log4rs::init_file("config/log.yaml", Default::default()).unwrap();

    let matches = Command::new("markdown_composer")
        .author(crate_authors!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::new("no-auto-open")
                .long("no-auto-open")
                .help("Don't open the web browser automatically.")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("browser")
                .long("browser")
                .value_name("executable")
                .help(
                    "Specify a browser that the program should open. If not supplied, the program \
                   will determine the user's default browser.",
                )
                .num_args(1),
        )
        .arg(
            Arg::new("theme")
                .long("highlight-theme")
                .help(
                    "The theme to use for syntax highlighting. All highlight.js themes are \
                   supported.",
                )
                .default_value("github"),
        )
        .arg(
            Arg::new("working-directory")
                .long("working-directory")
                .value_name("dir")
                .help(
                    "The directory that static files should be served out of. All relative links \
                   in the markdown will be served relative to this directory.",
                )
                .num_args(1),
        )
        .arg(
            Arg::new("css")
                .long("custom-css")
                .value_name("url/path")
                .help(
                    "CSS that should be used to style the markdown output. Defaults to \
                   GitHub-like CSS.",
                )
                .num_args(1)
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("external-renderer")
                .long("external-renderer")
                .help("An external process that should be used for rendering markdown.")
                .num_args(1),
        )
        .arg(
            Arg::new("markdown-file")
                .help("A markdown file that should be rendered by the server on startup."),
        )
        .arg(
            Arg::new("address")
                .long("address")
                .help("The address that this server will listen on. The default value is `localhost`.")
                .num_args(1),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .help("The port number that this server will listen on. The default value is `0 (ephemeral)`.")
                .num_args(1),
        )
        .get_matches();

    let mut server = Server::bind(format!(
        "{}:{}",
        matches
            .get_one::<String>("address")
            .map(|s| s.as_str())
            .unwrap_or("localhost"),
        matches
            .get_one::<String>("port")
            .map(|s| s.as_str())
            .unwrap_or("0")
    ))?;
    let (markdown_navigation_tx, markdown_navigation_rx) = mpsc::channel();
    server.set_markdown_navigation_channel(markdown_navigation_tx);
    spawn_markdown_navigation_writer(markdown_navigation_rx);

    if let Some(external_renderer) = matches.get_one::<String>("external-renderer") {
        server.set_external_renderer(parse_command(external_renderer));
    }

    if let Some(highlight_theme) = matches.get_one::<String>("theme") {
        server.set_highlight_theme(highlight_theme.to_string());
    }

    if let Some(working_directory) = matches.get_one::<String>("working-directory") {
        server.set_static_root(working_directory);
    }

    if let Some(custom_css) = matches.get_many::<String>("css") {
        server.set_custom_css(custom_css.map(|s| s.to_string()).collect())?;
    }

    if let Some(file_name) = matches.get_one::<String>("markdown-file") {
        server.send(fs::read_to_string(file_name)?)?;
    }

    let browser = matches.get_one::<String>("browser");

    if !matches.get_flag("no-auto-open") {
        if let Some(browser) = browser {
            server.open_specific_browser(parse_command(browser))?;
        } else {
            server.open_browser()?;
        };
    }

    let stdin = io::stdin();
    let stdin_lock = stdin.lock();

    read_rpc(stdin_lock, server, browser.as_ref().map(|s| s.as_str()))?;

    Ok(())
}

fn parse_command(s: &str) -> ProcessCommand {
    let words = Shlex::new(s).collect::<Vec<_>>();
    let (command, args) = words.split_first().expect("command was empty");
    let mut command = ProcessCommand::new(command);
    command.args(args);
    command
}

fn spawn_markdown_navigation_writer(receiver: mpsc::Receiver<std::path::PathBuf>) {
    thread::spawn(move || {
        for path in receiver {
            if let Err(err) = send_open_file_message(&path) {
                error!("could not send markdown navigation message: {}", err);
            }
        }
    });
}

#[cfg(feature = "msgpack")]
fn send_open_file_message(path: &Path) -> io::Result<()> {
    let command = format!("edit {}", vim_fnameescape(path));
    let message = (2u64, "nvim_command", vec![command]);
    let bytes = rmp_serde::to_vec(&message).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("could not encode msgpack-rpc notification: {}", err),
        )
    })?;

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(&bytes)?;
    stdout.flush()
}

#[cfg(feature = "json-rpc")]
fn send_open_file_message(path: &Path) -> io::Result<()> {
    let message = ("open_file", path.to_string_lossy().into_owned());

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    serde_json::to_writer(&mut stdout, &message)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

#[cfg(feature = "msgpack")]
fn vim_fnameescape(path: &Path) -> String {
    let path = path.to_string_lossy();
    let mut escaped = String::with_capacity(path.len());

    for ch in path.chars() {
        match ch {
            ' ' | '\t' | '\n' | '*' | '[' | ']' | '?' | '`' | '$' | '\\' | '%' | '#' | '\''
            | '"' | '|' | '!' | '<' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }

    escaped
}
