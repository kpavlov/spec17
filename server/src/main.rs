mod doctor;
mod frontend;
mod mcp;
mod ops;
mod skill;

use axum::{Router, extract::State, http::{HeaderMap, StatusCode, header}, middleware::{self, Next}, response::{IntoResponse, Response}};
// reqwest (dev proxy to Vite) is debug-only and mime_guess (embedded UI) release-only: mark each as used in the other profile so `unused_dependencies` stays quiet.
#[cfg(not(debug_assertions))]
use reqwest as _;
#[cfg(debug_assertions)]
use mime_guess as _;
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use std::{path::PathBuf, process::ExitCode, sync::Arc};

/// The port `serve` and the `.mcp.json` entry use unless told otherwise.
const DEFAULT_PORT: u16 = 7411;

#[derive(Clone)]
pub struct AppState {
    pub root: Option<PathBuf>,
    pub dashboard: String,
    /// Port of /mcp, for the .mcp.json entry the install tools write.
    pub mcp_port: u16,
    /// The UI has its own port (`--ui-port`); the main port then carries `/mcp` only.
    pub split: bool,
    pub http_mcp: bool,
    /// `host:port` values a request may carry (DNS-rebinding guard).
    pub hosts: Arc<Vec<String>>,
    /// Cancelled on Ctrl-C so open `subscriptions/listen` streams end and the graceful shutdown can finish.
    pub shutdown: tokio_util::sync::CancellationToken,
    #[cfg(debug_assertions)]
    pub http: reqwest::Client,
}

// clap colors only when stdout/stderr is a terminal and NO_COLOR is unset.
const STYLES: clap::builder::Styles = clap::builder::Styles::styled()
    .header(clap::builder::styling::AnsiColor::Yellow.on_default().bold())
    .usage(clap::builder::styling::AnsiColor::Yellow.on_default().bold())
    .literal(clap::builder::styling::AnsiColor::Green.on_default().bold())
    .placeholder(clap::builder::styling::AnsiColor::Cyan.on_default());

const VERSION: &str = env!("SPEC17_VERSION");
const LONG_VERSION: &str = concat!(env!("SPEC17_VERSION"), "\nCopyright (c) 2026 Konstantin Pavlov, IT Staff OÜ. All rights reserved.\n", env!("CARGO_PKG_HOMEPAGE"), "\n", env!("CARGO_PKG_REPOSITORY"));

#[derive(Parser)]
#[command(
    name = "spec17",
    version = VERSION,
    about = "Deterministic spec-driven-development evidence: evidence, not truth.",
    long_version = LONG_VERSION,
    after_help = concat!("Copyright (c) 2026 Konstantin Pavlov, IT Staff OÜ. All rights reserved. · About: ", env!("CARGO_PKG_HOMEPAGE")),
    styles = STYLES
)]
struct Cli {
    /// Repo root (default: nearest ancestor with a specs folder).
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// Print JSON instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(flatten)]
    transports: TransportArgs,
    /// Without a command: `serve` for the repo in the current directory, and open the dashboard.
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Args)]
struct TransportArgs {
    /// Enable MCP over stdin and stdout.
    #[arg(long, global = true)]
    stdio_mcp: bool,
    /// Disable the web dashboard and skip opening the browser.
    #[arg(long, global = true)]
    no_ui: bool,
    /// Disable streamable HTTP MCP on every port.
    #[arg(long, global = true, alias = "no-http-mc")]
    no_http_mcp: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Whole-project status: coverage, problems, where to look next.
    Report {
        /// Lines per problem section.
        #[arg(long, default_value_t = 12)]
        limit: usize,
    },
    /// Per-spec summary, or one line per requirement with a list flag.
    Status(StatusArgs),
    /// One requirement in depth: spec, tests, citations, related code.
    Trace {
        id: String,
        /// Also run `codegraph explore` for the related code.
        #[arg(long)]
        codegraph: bool,
    },
    /// What changed against a git ref (HEAD by default) or in one commit: the changed files and the requirements the changed lines touch.
    Changes(ChangeArgs),
    /// Unified diff of one changed file, with the requirements each hunk touches.
    Diff {
        /// Repo-relative path of a file `changes` lists.
        path: String,
        #[command(flatten)]
        c: ChangeArgs,
    },
    /// Commits on a branch that are not on the base branch, newest first.
    History {
        /// Default: main, master, origin/main or origin/master.
        #[arg(long)]
        base: Option<String>,
        /// Default: HEAD.
        #[arg(long)]
        head: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[command(flatten)]
        c: ChangeArgs,
    },
    /// Install the spec17 skill into <repo>/.agents/skills and add the spec17 MCP server to <repo>/.mcp.json.
    Install {
        /// Install into ~/.agents/skills and ~/.mcp.json instead of the repo.
        #[arg(long)]
        global: bool,
        /// Port of `spec17 serve` that the MCP entry points at.
        #[arg(long, default_value_t = DEFAULT_PORT)]
        port: u16,
        /// Only add the CodeGraph MCP server to <repo>/.mcp.json (an existing entry is kept); the skill and the spec17 entry are left alone.
        #[arg(long, conflicts_with = "global")]
        codegraph: bool,
    },
    /// Remove what `install` added; other skills and MCP servers stay.
    Uninstall {
        /// Use the home directory instead of the repo. Without --only, remove the global skill.
        #[arg(long)]
        global: bool,
        /// Remove only the spec17 skill or only its entry in .mcp.json.
        #[arg(long, value_enum, conflicts_with = "codegraph")]
        only: Option<skill::InstallPart>,
        /// Only remove the CodeGraph MCP server from <repo>/.mcp.json, whatever it holds; the skill and the spec17 entry are left alone.
        #[arg(long, conflicts_with = "global")]
        codegraph: bool,
    },
    /// Check the repo setup: specs, tests, sources, git, the codegraph binary and index, skill and MCP entry.
    Doctor {
        /// Also build or refresh the CodeGraph index when the checks call for it (runs `codegraph init` / `sync`). Without this flag doctor only reads.
        #[arg(long)]
        fix: bool,
        /// Port of `spec17 serve` that the MCP entry should point at.
        #[arg(long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
    /// Serve the dashboard (`/`, `/api/info`) and the MCP server (`/mcp`) on 127.0.0.1.
    Serve {
        #[arg(long, default_value_t = DEFAULT_PORT)]
        port: u16,
        /// Serve the UI on this port instead (it also carries /mcp, which the UI uses); `--port` then carries only /mcp.
        #[arg(long)]
        ui_port: Option<u16>,
        /// Don't open the dashboard in the browser.
        #[arg(long)]
        no_open: bool,
    },
    /// Code covered by 2+ requirements (needs the CodeGraph index).
    Overlap {
        path: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Code no requirement covers (needs the CodeGraph index).
    Uncovered {
        path: Option<String>,
        #[arg(long, default_value_t = 5)]
        fanin: usize,
        #[arg(long = "min", default_value_t = 10)]
        min_lines: i64,
    },
}

/// The arguments of the `changes` tool.
#[derive(Args)]
struct ChangeArgs {
    /// The git ref the working tree is compared with (default HEAD).
    #[arg(long = "ref")]
    reference: Option<String>,
    /// Explain this one commit against its first parent (ignores --ref, --staged and untracked files).
    #[arg(long)]
    commit: Option<String>,
    /// Only what is staged.
    #[arg(long)]
    staged: bool,
    /// Leave untracked files out.
    #[arg(long)]
    no_untracked: bool,
    /// Leave out changes that only touch whitespace.
    #[arg(long)]
    ignore_whitespace: bool,
    /// Also files .aiignore, .spec17ignore and .agents hide.
    #[arg(long)]
    show_ignored: bool,
    /// Also files spec17 does not scan (kind other).
    #[arg(long)]
    show_all: bool,
}

impl ChangeArgs {
    fn json(&self) -> serde_json::Value {
        json!({"ref": self.reference, "commit": self.commit, "staged": self.staged, "include_untracked": !self.no_untracked, "ignore_whitespace": self.ignore_whitespace, "show_ignored": self.show_ignored, "show_all": self.show_all})
    }
}

trait Merged {
    fn merged(self, other: serde_json::Value) -> serde_json::Value;
}

impl Merged for serde_json::Value {
    fn merged(mut self, other: serde_json::Value) -> serde_json::Value {
        self.as_object_mut().unwrap().extend(other.as_object().unwrap().clone());
        self
    }
}

#[derive(Args)]
struct StatusArgs {
    #[arg(long)]
    spec: Option<String>,
    #[arg(long)]
    prefix: Option<String>,
    #[arg(long)]
    all: bool,
    #[arg(long)]
    missing: bool,
    #[arg(long, num_args = 0..=1, default_missing_value = "HEAD", value_name = "REF")]
    changed: Option<String>,
    /// With --changed (HEAD when not given): only what is staged.
    #[arg(long)]
    staged: bool,
    /// With --changed: leave untracked files out.
    #[arg(long)]
    no_untracked: bool,
    /// With --changed: leave out changes that only touch whitespace.
    #[arg(long)]
    ignore_whitespace: bool,
    #[arg(long)]
    built: bool,
    #[arg(long)]
    done: bool,
    #[arg(long)]
    unproven: bool,
    #[arg(long)]
    issues: bool,
    #[arg(long, default_value = "low", value_parser = ["low", "medium", "high"])]
    effort: String,
    #[arg(long, num_args = 0..=1, default_missing_value = "", value_name = "PATH")]
    overlap: Option<String>,
    #[arg(long, num_args = 0..=1, default_missing_value = "", value_name = "PATH")]
    uncovered: Option<String>,
}

impl StatusArgs {
    /// The arguments of the `status` tool; the CLI has no row cap.
    fn json(&self) -> serde_json::Value {
        json!({
            "spec": self.spec, "prefix": self.prefix, "all": self.all, "missing": self.missing, "changed": self.changed, "staged": self.staged,
            "include_untracked": !self.no_untracked, "ignore_whitespace": self.ignore_whitespace, "built": self.built, "done": self.done,
            "unproven": self.unproven, "issues": self.issues, "effort": self.effort, "limit": i64::MAX
        })
    }
}

// (API-02)
/// 403 unless the Host (and Origin, when sent) is one this server bound.
async fn guard(State(s): State<AppState>, headers: HeaderMap, req: axum::extract::Request, next: Next) -> Response {
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
    let origin_ok = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()).is_none_or(|o| o.strip_prefix("http://").is_some_and(|h| s.hosts.iter().any(|x| x == h)));
    if s.hosts.iter().any(|h| h == host) && origin_ok { next.run(req).await } else { StatusCode::FORBIDDEN.into_response() }
}

/// What the dashboard shows in its header: the repo and the evidence note. The one thing MCP does not carry.
async fn info(State(s): State<AppState>) -> Result<axum::Json<serde_json::Value>, (StatusCode, axum::Json<serde_json::Value>)> {
    ops::info(s.root.as_ref()).map(axum::Json).map_err(|e| (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({"error": e}))))
}

/// (main router, optional UI router). Without `ui_port` the main router serves everything; with it, `/mcp` only, and the UI router
/// serves the UI and `/mcp` too, since the dashboard talks to the server over MCP.
fn routers(state: AppState) -> (Router, Option<Router>) {
    let ui = || Router::new().route("/api/info", axum::routing::get(info)).route("/", axum::routing::get(frontend::handler)).route("/{*path}", axum::routing::get(frontend::handler));
    let mcp = || {
        if state.http_mcp {
            Router::new().nest_service("/mcp", mcp::service(state.clone(), &state.hosts))
        } else {
            Router::new().route("/mcp", axum::routing::any(|| async { StatusCode::NOT_FOUND }))
        }
    };
    let guarded = |r: Router<AppState>| -> Router {
        r.layer(middleware::from_fn_with_state(state.clone(), guard)).layer(tower_http::compression::CompressionLayer::new()).with_state(state.clone())
    };
    if state.dashboard.is_empty() {
        (guarded(mcp()), None)
    } else if state.split {
        (guarded(mcp()), Some(guarded(ui().merge(mcp()))))
    } else {
        (guarded(ui().merge(mcp())), None)
    }
}

fn app_state(root: Option<PathBuf>, ports: &[u16]) -> AppState {
    let hosts = ports.iter().flat_map(|p| [format!("127.0.0.1:{p}"), format!("localhost:{p}")]).collect();
    let dashboard = ports.last().map(|port| format!("http://127.0.0.1:{port}/")).unwrap_or_default();
    AppState { root, dashboard, mcp_port: ports.first().copied().unwrap_or(DEFAULT_PORT), split: ports.len() > 1, http_mcp: true, hosts: Arc::new(hosts), shutdown: Default::default(), #[cfg(debug_assertions)] http: reqwest::Client::new() }
}

/// Best effort: a failure to launch a browser is not an error.
fn open_browser(url: &str) {
    let (cmd, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") { ("open", vec![url]) } else if cfg!(windows) { ("cmd", vec!["/c", "start", "", url]) } else { ("xdg-open", vec![url]) };
    let _ = std::process::Command::new(cmd).args(args).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
}

// (API-22)
fn serve(root: Option<PathBuf>, port: u16, ui_port: Option<u16>, no_open: bool, transports: TransportArgs) -> Result<u8, String> {
    let TransportArgs { stdio_mcp, no_ui, no_http_mcp } = transports;
    if no_ui && no_http_mcp && !stdio_mcp {
        return Err("Enable --stdio-mcp when both --no-ui and --no-http-mcp are set.".into());
    }
    // the workspace is fixed once, here: the repo that holds the current directory (or --root)
    let root = spec17_core::scan::Project::open(root.as_deref())?.root;
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())?;
    let result = rt.block_on(async move {
        let bind = |p: u16| async move { tokio::net::TcpListener::bind(("127.0.0.1", p)).await.map_err(|e| format!("127.0.0.1:{p}: {e}")) };
        let ui_port = ui_port.filter(|_| !no_ui);
        let ports: Vec<u16> = if no_http_mcp {
            (!no_ui).then_some(ui_port.unwrap_or(port)).into_iter().collect()
        } else {
            std::iter::once(port).chain(ui_port).collect()
        };
        let mut state = app_state(Some(root.clone()), &ports);
        state.mcp_port = port;
        state.http_mcp = !no_http_mcp;
        if no_ui {
            state.dashboard.clear();
        }
        // (API-21) (API-23)
        if !stdio_mcp {
            eprintln!("spec17 {LONG_VERSION}\n\n  workspace {}", root.display());
            if !no_ui {
                eprintln!("  dashboard {}", state.dashboard);
            }
            if !no_http_mcp {
                eprintln!("  mcp       http://127.0.0.1:{port}/mcp");
                let setup_ui = if no_ui { "" } else { "  (or use Settings in the dashboard)" };
                match skill::status(Some(&root), port) {
                    Ok(st) => skill::suggestions(&st).iter().for_each(|t| eprintln!("  setup     {t}{setup_ui}")),
                    Err(e) => eprintln!("  setup     could not check the install: {e}"),
                }
            }
        }
        let stop = state.shutdown.clone();
        let signal_stop = stop.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            signal_stop.cancel();
        });
        let http = async {
            let [main_port, ..] = ports.as_slice() else { return Ok(()) };
            let (main, ui) = routers(state.clone());
            let run = |listener, app: Router| async {
                axum::serve(listener, app).with_graceful_shutdown(stop.clone().cancelled_owned()).await.map_err(|e| e.to_string())
            };
            let main_listener = bind(*main_port).await?;
            let second = match (ui, ports.get(1)) {
                (Some(ui), Some(port)) => Some((bind(*port).await?, ui)),
                _ => None,
            };
            if !no_open && !no_ui {
                open_browser(&state.dashboard); // after the bind, so the page is there when the browser asks
            }
            if let Some((listener, ui)) = second {
                tokio::try_join!(run(main_listener, main), run(listener, ui))?;
            } else {
                run(main_listener, main).await?;
            }
            Ok::<(), String>(())
        };
        let stdio = async {
            if stdio_mcp {
                let result = mcp::serve_stdio(state.clone()).await;
                stop.cancel();
                result?;
            }
            Ok::<(), String>(())
        };
        tokio::try_join!(http, stdio)?;
        Ok(0)
    });
    if stdio_mcp {
        // Tokio's stdin read may still be blocked after Ctrl-C on platforms without async pipes.
        rt.shutdown_background();
    }
    result
}

/// `port` = install, `None` = uninstall.
fn install(root: Option<&std::path::Path>, global: bool, port: Option<u16>, only: Option<skill::InstallPart>) -> Result<u8, String> {
    skill::apply(root, global, port, only, skill::McpTransport::Http)?.iter().for_each(|l| println!("{l}"));
    if port.is_some() {
        println!("next   spec17   # starts the MCP server and dashboard the skill talks to");
    }
    Ok(0)
}

fn run(cli: Cli) -> Result<u8, String> {
    let cmd = cli.cmd.unwrap_or(Cmd::Serve { port: DEFAULT_PORT, ui_port: None, no_open: false });
    if !matches!(cmd, Cmd::Serve { .. }) && (cli.transports.stdio_mcp || cli.transports.no_ui || cli.transports.no_http_mcp) {
        return Err("Transport flags apply only when starting the server.".into());
    }
    match cmd {
        Cmd::Serve { port, ui_port, no_open } => return serve(cli.root, port, ui_port, no_open, cli.transports),
        Cmd::Doctor { fix, port } => {
            let mut checks = doctor::run(cli.root.as_deref(), port);
            if fix && let Some(action) = doctor::suggested(&checks) {
                eprint!("{}", doctor::index(cli.root.as_deref(), action)?);
                checks = doctor::run(cli.root.as_deref(), port);
            }
            if cli.json { println!("{}", serde_json::to_string_pretty(&checks).unwrap()) } else { print!("{}", doctor::text(&checks)) }
            return Ok(u8::from(checks.iter().any(|k| k.level == "fail")));
        }
        Cmd::Install { codegraph: true, .. } => { skill::apply_codegraph(cli.root.as_deref(), true)?.iter().for_each(|l| println!("{l}")); return Ok(0) }
        Cmd::Uninstall { codegraph: true, .. } => { skill::apply_codegraph(cli.root.as_deref(), false)?.iter().for_each(|l| println!("{l}")); return Ok(0) }
        Cmd::Install { global, port, .. } => return install(cli.root.as_deref(), global, Some(port), None),
        Cmd::Uninstall { global, only, .. } => return install(cli.root.as_deref(), global, None, only),
        _ => {}
    }
    // (API-14) the CLI is one more client of the tools: what MCP returns, printed
    let (name, args) = match &cmd {
        Cmd::Report { limit } => ("report", json!({"limit": limit})),
        Cmd::Status(s) => match (&s.overlap, &s.uncovered) {
            (Some(p), _) => ("overlap", json!({"path": p})),
            (_, Some(p)) => ("uncovered", json!({"path": p})),
            _ => ("status", s.json()),
        },
        Cmd::Trace { id, .. } => ("trace", json!({"id": id})),
        Cmd::Changes(c) => ("changes", c.json()),
        Cmd::Diff { path, c } => ("diff", json!({"path": path}).merged(c.json())),
        Cmd::History { base, head, limit, c } => ("history", json!({"base": base, "head": head, "limit": limit}).merged(c.json())),
        Cmd::Overlap { path, all } => ("overlap", json!({"path": path, "all": all})),
        Cmd::Uncovered { path, fanin, min_lines } => ("uncovered", json!({"path": path, "fanin": fanin, "min": min_lines})),
        Cmd::Serve { .. } | Cmd::Install { .. } | Cmd::Uninstall { .. } | Cmd::Doctor { .. } => return Err("not a tool command".into()),
    };
    let (text, data) = match ops::call(cli.root.as_deref(), name, &args, "", DEFAULT_PORT) {
        Ok(r) => r,
        // a requirement that is not there is an answer, not a failure of the tool
        Err(e) if name == "trace" => {
            println!("{e}");
            return Ok(1);
        }
        Err(e) => return Err(e.to_string()),
    };
    // `overlap --json` has always printed the rows themselves
    let data = if name == "overlap" { data["rows"].clone() } else { data };
    if cli.json { println!("{}", serde_json::to_string_pretty(&data).unwrap()) } else { print!("{text}") }
    if let Cmd::Trace { codegraph: true, .. } = cmd && let Some(q) = data["query"].as_str().filter(|q| !q.is_empty()) {
        let root = spec17_core::scan::Project::open(cli.root.as_deref())?.root;
        let _ = std::process::Command::new("codegraph").args(["explore", q]).current_dir(root).status();
    }
    Ok(0)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let quiet = cli.transports.stdio_mcp; // (API-23)
    match run(cli) {
        Ok(c) => ExitCode::from(c),
        Err(e) => {
            if !quiet {
                eprintln!("{e}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use serde_json::json;
    use tower::ServiceExt;

    fn repo() -> PathBuf {
        let d = std::env::temp_dir().join(format!("spec17-srv-{}", std::process::id()));
        std::fs::create_dir_all(d.join("specs")).unwrap();
        std::fs::write(d.join("specs/01-a.md"), "- **AAA-01** MUST — one.\n- **AAA-02** MUST — two.\n").unwrap();
        std::fs::create_dir_all(d.join("RunSolo/RunSolo")).unwrap();
        std::fs::write(d.join("RunSolo/RunSolo/A.swift"), "struct A {}\n").unwrap();
        d
    }

    async fn send(app: &Router, req: Request<Body>) -> (StatusCode, String) {
        let r = app.clone().oneshot(req).await.unwrap();
        let st = r.status();
        (st, String::from_utf8_lossy(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).into_owned())
    }

    fn post(uri: &str, host: &str, body: &str) -> Request<Body> {
        Request::post(uri).header("host", host).header("content-type", "application/json").header("accept", "application/json, text/event-stream").body(Body::from(body.to_string())).unwrap()
    }

    /// A tool call over `/mcp`: what the CLI runs, and what the dashboard sends.
    fn tool(name: &str, args: &str, host: &str) -> Request<Body> {
        post("/mcp", host, &format!(r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{name}","arguments":{args}}}}}"#))
    }

    #[doc = "[API-22]"]
    #[tokio::test]
    async fn http_routes_can_be_disabled_independently() {
        let host = "127.0.0.1:7411";
        let get = |path: &str| Request::get(path).header("host", host).body(Body::empty()).unwrap();
        let mut state = app_state(Some(repo()), &[7411]);
        state.http_mcp = false;
        let (app, _) = routers(state);
        assert_eq!(send(&app, tool("report", "{}", host)).await.0, StatusCode::NOT_FOUND);
        assert_eq!(send(&app, get("/mcp")).await.0, StatusCode::NOT_FOUND);
        assert_eq!(send(&app, get("/api/info")).await.0, StatusCode::OK);

        let mut state = app_state(Some(repo()), &[7411]);
        state.dashboard.clear();
        let (app, ui) = routers(state);
        assert!(ui.is_none());
        assert_eq!(send(&app, get("/")).await.0, StatusCode::NOT_FOUND);
        assert_eq!(send(&app, get("/api/info")).await.0, StatusCode::NOT_FOUND);
        assert_eq!(send(&app, tool("report", "{}", host)).await.0, StatusCode::OK);
    }

    #[doc = "[SETUP-13]"]
    #[tokio::test]
    async fn setup_tools_manage_transports_independently() {
        let root = std::env::temp_dir().join(format!("spec17-setup-transports-{}", std::process::id()));
        std::fs::create_dir_all(root.join("specs")).unwrap();
        let config = root.join(".mcp.json");
        std::fs::write(&config, r#"{"mcpServers":{"another":{"command":"another"}},"custom":true}"#).unwrap();
        let (app, _) = routers(app_state(Some(root.clone()), &[7411]));
        let action = |name, args| tool(name, args, "127.0.0.1:7411");
        let stdio = r#"{"only":"mcp","transport":"stdio"}"#;
        let http = r#"{"only":"mcp","transport":"http"}"#;
        for args in [http, stdio] {
            let (status, body) = send(&app, action("install", args)).await;
            assert!(status == StatusCode::OK && !body.contains(r#""isError":true"#), "{body}");
        }
        let read = || serde_json::from_slice::<serde_json::Value>(&std::fs::read(&config).unwrap()).unwrap();
        let settings = read();
        let args = settings["mcpServers"]["spec17-stdio"]["args"].as_array().unwrap();
        assert_eq!(args, &vec![json!("--stdio-mcp"), json!("--no-http-mcp"), json!("--no-ui"), json!("--root"), json!(root.canonicalize().unwrap())]);
        assert_eq!(settings["mcpServers"]["spec17"]["url"], "http://127.0.0.1:7411/mcp");
        assert!(!root.join(".agents").exists());

        let (_, body) = send(&app, action("uninstall", http)).await;
        assert!(body.contains(r#""mcp_stdio":"ok""#), "{body}");
        assert!(read()["mcpServers"].get("spec17").is_none());
        send(&app, action("uninstall", stdio)).await;
        assert_eq!(read(), json!({"mcpServers":{"another":{"command":"another"}},"custom":true}));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[doc = "[API-14]"]
    #[test]
    fn cli_commands_run_the_tools() {
        // its own repo: it runs git and edits a spec, which the shared `repo()` fixture must not see
        let d = std::env::temp_dir().join(format!("spec17-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("specs")).unwrap();
        std::fs::write(d.join("specs/01-a.md"), "- **AAA-01** MUST — one.\n- **AAA-02** MUST — two.\n").unwrap();
        let git = |args: &[&str]| assert!(std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).current_dir(&d).output().unwrap().status.success(), "{args:?}");
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "one"]);
        std::fs::write(d.join("specs/01-a.md"), "- **AAA-01** MUST — one.\n- **AAA-02** MUST — two, edited.\n").unwrap();
        let go = |args: &[&str]| run(Cli::parse_from(["spec17", "--root", d.to_str().unwrap()].into_iter().chain(args.iter().copied())));
        for ok in [&["report"][..], &["status", "--all"], &["trace", "AAA-01"], &["changes", "--no-untracked", "--ref", "HEAD"], &["diff", "specs/01-a.md"], &["history", "--limit", "5"]] {
            assert_eq!(go(ok), Ok(0), "{ok:?}");
        }
        assert_eq!(go(&["trace", "NOPE-1"]), Ok(1));
        assert!(go(&["diff", "nope.md"]).is_err(), "a file `changes` does not list is refused");
    }

    #[doc = "[API-07]"]
    #[tokio::test]
    async fn mcp_2026_07_28_needs_no_handshake() {
        let (app, _) = routers(app_state(Some(repo()), &[7411]));
        let read = |uri: &str| {
            let meta = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"t","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}"#;
            let body = format!(r#"{{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{{"uri":"{uri}",{meta}}}}}"#);
            let mut r = post("/mcp", "127.0.0.1:7411", &body);
            r.headers_mut().insert("mcp-protocol-version", "2026-07-28".parse().unwrap());
            r.headers_mut().insert("mcp-method", "resources/read".parse().unwrap());
            r.headers_mut().insert("mcp-name", uri.parse().unwrap());
            r
        };
        let (st, body) = send(&app, read("spec17://tests")).await;
        assert!(st == StatusCode::OK && body.contains("total"), "{st} {body}");
        let mut d = post("/mcp", "127.0.0.1:7411", r#"{"jsonrpc":"2.0","id":2,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"t","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}}"#);
        d.headers_mut().insert("mcp-protocol-version", "2026-07-28".parse().unwrap());
        d.headers_mut().insert("mcp-method", "server/discover".parse().unwrap());
        let (st, body) = send(&app, d).await;
        assert!(st == StatusCode::OK && body.contains("2026-07-28"), "{st} {body}");
    }

    #[doc = "[API-25]"]
    #[tokio::test]
    async fn mcp_skills_extension_serves_embedded_skill() {
        let (app, _) = routers(app_state(Some(repo()), &[7411]));
        let call = async |method: &str, params: serde_json::Value| {
            let mut params = params;
            params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28", "io.modelcontextprotocol/clientInfo":{"name":"t","version":"1"}, "io.modelcontextprotocol/clientCapabilities":{}});
            let body = json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params}).to_string();
            let mut req = post("/mcp", "127.0.0.1:7411", &body);
            req.headers_mut().insert("mcp-protocol-version", "2026-07-28".parse().unwrap());
            req.headers_mut().insert("mcp-method", method.parse().unwrap());
            if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                req.headers_mut().insert("mcp-name", uri.parse().unwrap());
            }
            let (status, body) = send(&app, req).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            serde_json::from_str::<serde_json::Value>(&body).unwrap()
        };
        let discover = call("server/discover", json!({})).await;
        assert!(discover["result"]["capabilities"]["extensions"]["io.modelcontextprotocol/skills"].is_object());
        let list = call("skills/list", json!({})).await;
        for cursor in ["invalid", ""] {
            let invalid = call("skills/list", json!({"cursor":cursor})).await;
            assert_eq!(invalid["error"]["code"], -32602);
        }
        let entry = &list["result"]["skills"][0];
        let uri = "skill://spec17/SKILL.md";
        assert_eq!(entry["uri"], uri);
        assert_eq!(entry["frontmatter"]["name"], "spec17");
        let resources = call("resources/list", json!({})).await;
        assert!(resources["result"]["resources"].as_array().unwrap().iter().any(|r| r["uri"] == uri));
        let get = call("skills/get", json!({"uri":uri})).await;
        assert_eq!(get["result"]["skill"], *entry);
        let read = call("resources/read", json!({"uri":uri})).await;
        let contents = &read["result"]["contents"][0];
        assert_eq!(contents["mimeType"], "text/markdown");
        assert_eq!(contents["text"], skill::SKILL);
        let served = contents["text"].as_str().unwrap();
        let frontmatter = served.split_inclusive('\n').skip(1).take_while(|line| *line != "---\n").collect::<String>();
        assert_eq!(entry["frontmatter"], yaml_serde::from_str::<serde_json::Value>(&frontmatter).unwrap());
        let manifest = entry["resources"].as_array().unwrap();
        assert_eq!(manifest.len(), skill::FILES.len());
        for (m, (path, text)) in manifest.iter().zip(skill::FILES) {
            let m: rmcp::model::SkillResource = serde_json::from_value(m.clone()).unwrap();
            let file_uri = format!("skill://spec17/{path}");
            let read = call("resources/read", json!({"uri": file_uri})).await;
            let served = read["result"]["contents"][0]["text"].as_str().unwrap();
            assert_eq!(served, text);
            m.verify(served.as_bytes()).unwrap();
        }
        let unknown = call("skills/get", json!({"uri":"skill://missing/SKILL.md"})).await;
        assert_eq!(unknown["error"]["code"], -32602);
        let unknown = call("resources/read", json!({"uri":"skill://spec17/missing.md"})).await;
        assert_eq!(unknown["error"]["code"], -32602);
    }

    #[doc = "[API-08]"]
    #[tokio::test]
    async fn subscription_notifies_when_the_repo_changes() {
        use http_body_util::BodyExt;
        use std::time::Duration;
        let d = std::env::temp_dir().join(format!("spec17-sub-{}", std::process::id()));
        std::fs::create_dir_all(d.join("specs")).unwrap();
        std::fs::write(d.join("specs/01-a.md"), "- **AAA-01** MUST — one.\n").unwrap();
        let state = app_state(Some(d.clone()), &[7411]);
        let stop = state.shutdown.clone();
        let (app, _) = routers(state);
        let meta = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"t","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}"#;
        let body = format!(r#"{{"jsonrpc":"2.0","id":1,"method":"subscriptions/listen","params":{{{meta},"notifications":{{"resourceSubscriptions":["spec17://report","spec17://nope"]}}}}}}"#);
        let mut req = post("/mcp", "127.0.0.1:7411", &body);
        req.headers_mut().insert("mcp-protocol-version", "2026-07-28".parse().unwrap());
        req.headers_mut().insert("mcp-method", "subscriptions/listen".parse().unwrap());
        let mut stream = app.clone().oneshot(req).await.unwrap().into_body();
        // reads frames until `want` shows up in what has arrived so far
        let mut seen = String::new();
        let mut until = async |want: &str, seen: &mut String| {
            tokio::time::timeout(Duration::from_secs(8), async {
                while !seen.contains(want) {
                    match stream.frame().await {
                        Some(Ok(f)) => seen.push_str(&String::from_utf8_lossy(f.data_ref().map_or(&[][..], |b| &b[..]))),
                        _ => break,
                    }
                }
            })
            .await
            .unwrap();
            assert!(seen.contains(want), "{want} not in {seen}");
        };
        until("acknowledged", &mut seen).await;
        assert!(seen.contains("spec17://report") && !seen.contains("spec17://nope"), "{seen}");
        seen.clear();
        tokio::time::sleep(Duration::from_millis(500)).await; // the listener takes its first stamp after the ack
        std::fs::write(d.join("specs/01-a.md"), "- **AAA-01** MUST — one.\n- **AAA-02** MUST — two.\n").unwrap();
        until("notifications/resources/updated", &mut seen).await;
        assert!(seen.contains("spec17://report"), "{seen}");
        seen.clear();
        stop.cancel(); // shutdown ends the stream with the graceful result
        until("complete", &mut seen).await;
        // (API-08) the view is readable too, beside its tool
        let (_, body) = send(&app, post("/mcp", "127.0.0.1:7411", r#"{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"spec17://report"}}"#)).await;
        assert!(body.contains("text/plain") && body.contains("SDD STATUS"), "{body}");
    }



    #[doc = "[API-09]"]
    #[test]
    fn tools_declare_an_output_schema_whose_required_keys_their_results_have() {
        let d = repo();
        let args = |n: &str| match n {
            "trace" => json!({"id": "AAA-01"}),
            "source" => json!({"path": "RunSolo/RunSolo/A.swift"}),
            _ => json!({}),
        };
        let mut checked = 0;
        for t in ops::tools().iter() {
            let schema = ops::output_schema(t.name).unwrap_or_else(|| panic!("{} has no output schema", t.name));
            assert_eq!(schema["type"], "object", "{}", t.name);
            // portable: nullable fields are `anyOf` of single types, never `"type": [.., "null"]`
            assert!(!schema.to_string().contains(r#""type":["#), "{}: {schema}", t.name);
            // Writing tools and tools that need a different fixture are exercised in their own tests.
            if matches!(t.name, "install" | "uninstall" | "codegraph" | "module" | "overlap" | "uncovered" | "changes" | "diff" | "history" | "commits" | "blame" | "search_requirements") {
                continue;
            }
            let (_, data) = ops::call(Some(&d), t.name, &args(t.name), "http://x/", 7411).unwrap();
            for k in schema["required"].as_array().unwrap() {
                assert!(data.get(k.as_str().unwrap()).is_some(), "{}: `{k}` missing from {data}", t.name);
            }
            checked += 1;
        }
        assert!(checked >= 6, "{checked}");
    }

    #[doc = "[API-13, API-15, API-16, SCAN-19]"]
    #[tokio::test]
    async fn changes_lists_files_and_requirements_and_needs_git() {
        let d = std::env::temp_dir().join(format!("spec17-chg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("specs")).unwrap();
        std::fs::write(d.join("specs/01-a.md"), "- **AAA-01** MUST — one.\n").unwrap();
        let (app, _) = routers(app_state(Some(d.clone()), &[7411]));
        let (_, body) = send(&app, tool("changes", "{}", "127.0.0.1:7411")).await;
        assert!(body.contains("git"), "{body}");
        let git = |a: &[&str]| assert!(std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(a).current_dir(&d).output().unwrap().status.success());
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-qm", "x"]);
        let (_, body) = send(&app, tool("blame", r#"{"path":"specs/01-a.md","lines":[1,5]}"#, "127.0.0.1:7411")).await;
        assert!(body.contains(r#""subject":"x""#) && !body.contains(r#""5":"#), "{body}");
        let (_, body) = send(&app, tool("blame", r#"{"path":"/etc/passwd","lines":[1]}"#, "127.0.0.1:7411")).await;
        assert!(body.contains("not a scanned file"), "{body}");
        std::fs::write(d.join("specs/01-a.md"), "- **AAA-01** MUST — one, edited.\n").unwrap();
        std::fs::write(d.join("new.txt"), "x\n").unwrap();
        let (_, body) = send(&app, tool("changes", "{}", "127.0.0.1:7411")).await;
        assert!(!body.contains("new.txt") && body.contains("AAA-01") && body.contains("modified"), "files spec17 does not scan stay hidden: {body}");
        let (_, body) = send(&app, tool("changes", r#"{"show_all":true}"#, "127.0.0.1:7411")).await;
        assert!(body.contains("untracked") && body.contains("added") && body.contains("new.txt"), "{body}");
        let (_, body) = send(&app, tool("changes", r#"{"show_all":true,"include_untracked":false}"#, "127.0.0.1:7411")).await;
        assert!(!body.contains("new.txt") && body.contains("specs/01-a.md"), "{body}");
        let (_, body) = send(&app, tool("diff", r#"{"path":"specs/01-a.md"}"#, "127.0.0.1:7411")).await;
        assert!(body.contains("-- **AAA-01** MUST — one.") && body.contains("+- **AAA-01** MUST — one, edited."), "{body}");
        let (_, body) = send(&app, tool("diff", r#"{"path":"new.txt","show_all":true}"#, "127.0.0.1:7411")).await;
        assert!(body.contains("+x") && body.contains("added"), "{body}");
        let (_, body) = send(&app, tool("history", "{}", "127.0.0.1:7411")).await;
        assert!(body.contains("commits") && body.contains("uncommitted"), "{body}");
        let (_, body) = send(&app, tool("diff", r#"{"path":"/etc/passwd"}"#, "127.0.0.1:7411")).await;
        assert!(body.contains("not a changed file"), "{body}");
        let (_, body) = send(&app, tool("changes", r#"{"staged":true}"#, "127.0.0.1:7411")).await;
        assert!(body.contains(r#""files":[]"#), "{body}");
        let (_, body) = send(&app, tool("commits", "{}", "127.0.0.1:7411")).await;
        assert!(body.contains(r#"commits"#) && !body.contains("AAA-01 "), "AAA-01 is edited, not committed: {body}");
        let rpc = |id: u32, method: &str, params: &str| post("/mcp", "127.0.0.1:7411", &format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#));
        let (_, body) = send(&app, rpc(5, "resources/read", r#"{"uri":"spec17://changes.json"}"#)).await;
        assert!(body.contains("AAA-01"), "{body}");
    }

    #[doc = "[API-10]"]
    #[tokio::test]
    async fn json_resources_name_the_schema_that_types_them() {
        let (app, _) = routers(app_state(Some(repo()), &[7411]));
        let rpc = |id: u32, method: &str, params: &str| post("/mcp", "127.0.0.1:7411", &format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#));
        let init = r#"{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"1"}}"#;
        send(&app, rpc(1, "initialize", init)).await;
        let (_, list) = send(&app, rpc(2, "resources/list", "{}")).await;
        for n in ["report", "status", "uncovered", "overlap", "modules", "tests"] {
            assert!(list.contains(&format!("spec17://schemas/{n}")), "{n}: {list}");
        }
        assert!(list.contains(r#""schema":"spec17://schemas/report""#) && list.contains("spec17://report.json"), "{list}");
        let (_, body) = send(&app, rpc(3, "resources/read", r#"{"uri":"spec17://report.json"}"#)).await;
        assert!(body.contains("application/json") && body.contains("requirements"), "{body}");
        let (_, body) = send(&app, rpc(4, "resources/read", r#"{"uri":"spec17://schemas/modules"}"#)).await;
        assert!(body.contains("application/schema+json") && body.contains("properties"), "{body}");
        let (_, tools) = send(&app, rpc(5, "tools/list", "{}")).await;
        assert!(tools.contains("outputSchema"), "{tools}");
    }

    #[doc = "[API-01, API-02, API-03]"]
    #[tokio::test]
    async fn api_and_guard() {
        let (app, ui) = routers(app_state(Some(repo()), &[7411]));
        assert!(ui.is_none());
        let (st, body) = send(&app, tool("report", "{}", "127.0.0.1:7411")).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert!(body.contains("SDD STATUS") && body.contains("\"none\":2"), "{body}");
        let (st, body) = send(&app, tool("trace", r#"{"id":"aaa-01"}"#, "localhost:7411")).await;
        assert!(st == StatusCode::OK && body.contains("AAA-01"), "{body}");
        let (_, body) = send(&app, tool("file", "{}", "127.0.0.1:7411")).await;
        assert!(body.contains("01-a.md"), "{body}");
        let (st, body) = send(&app, tool("file", r#"{"name":"specs/01-a.md"}"#, "127.0.0.1:7411")).await;
        assert!(st == StatusCode::OK && body.contains("AAA-02"), "{body}");
        for bad in ["../Cargo.toml", "/etc/passwd", "01-a"] {
            let (_, body) = send(&app, tool("file", &format!(r#"{{"name":"{bad}"}}"#), "127.0.0.1:7411")).await;
            assert!(body.contains(r#""isError":true"#), "{bad}: {body}");
        }
        let (st, body) = send(&app, tool("source", r#"{"path":"RunSolo/RunSolo/A.swift"}"#, "127.0.0.1:7411")).await;
        assert!(st == StatusCode::OK && body.contains("struct A"), "{body}");
        for bad in ["../Cargo.toml", "/etc/passwd", "specs/01-a.md", "RunSolo/RunSolo/../../specs/01-a.md"] {
            let (_, body) = send(&app, tool("source", &format!(r#"{{"path":"{bad}"}}"#), "127.0.0.1:7411")).await;
            assert!(body.contains(r#""isError":true"#), "{bad}: {body}");
        }
        assert!(send(&app, tool("nope", "{}", "127.0.0.1:7411")).await.1.contains(r#""isError":true"#));
        assert_eq!(send(&app, tool("report", "{}", "evil.test")).await.0, StatusCode::FORBIDDEN);
        let mut r = tool("report", "{}", "127.0.0.1:7411");
        r.headers_mut().insert("origin", "http://evil.test".parse().unwrap());
        assert_eq!(send(&app, r).await.0, StatusCode::FORBIDDEN);
    }

    #[doc = "[API-11]"]
    #[doc = "[API-12]"]
    #[test]
    fn module_tool_hides_and_shows_a_module_folder_and_refuses_the_rest() {
        // its own copy: the shared fixture must keep every file listed for the other tests
        let d = std::env::temp_dir().join(format!("spec17-ign-{}", std::process::id()));
        std::fs::create_dir_all(d.join("specs")).unwrap();
        std::fs::write(d.join("specs/01-a.md"), "- **AAA-01** MUST — one.\n").unwrap();
        std::fs::create_dir_all(d.join("RunSolo/RunSolo")).unwrap();
        std::fs::write(d.join("RunSolo/RunSolo/A.swift"), "struct A {}\n").unwrap();
        let (_, mods) = ops::read_resource(Some(&d), "spec17://modules").unwrap().unwrap();
        let mods: serde_json::Value = serde_json::from_str(&mods).unwrap();
        let m = mods["modules"].as_array().unwrap().iter().map(|m| m["path"].as_str().unwrap()).find(|p| *p != ".").expect("a module").to_string();
        std::fs::write(d.join(".spec17ignore"), "*.tmp").unwrap(); // no trailing newline
        let (text, _) = ops::call(Some(&d), "module", &json!({"path": m, "hidden": true}), "", 0).unwrap();
        assert!(text.contains(".spec17ignore"), "{text}");
        assert_eq!(std::fs::read_to_string(d.join(".spec17ignore")).unwrap(), format!("*.tmp\n/{m}/\n"));
        let refused = |path: &str, hidden: bool| ops::call(Some(&d), "module", &json!({"path": path, "hidden": hidden}), "", 0).is_err();
        for (bad, hidden) in [(".", true), ("*", true), ("nope", true)] {
            assert!(refused(bad, hidden), "{bad}");
        }
        // (API-12) the module is listed as hidden, and unignore removes just its line
        let (_, mods) = ops::read_resource(Some(&d), "spec17://modules").unwrap().unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&mods).unwrap()["hidden"], json!([m]));
        for bad in ["*.tmp", "nope", "."] {
            assert!(refused(bad, false), "{bad}");
        }
        let (text, _) = ops::call(Some(&d), "module", &json!({"path": m, "hidden": false}), "", 0).unwrap();
        assert!(text.contains(".spec17ignore"), "{text}");
        assert_eq!(std::fs::read_to_string(d.join(".spec17ignore")).unwrap(), "*.tmp\n");
        std::fs::remove_dir_all(&d).ok();
    }

    #[doc = "[SETUP-01, SETUP-12, WEB-42]"]
    #[tokio::test]
    async fn install_tools_write_the_repo_and_report_status() {
        let repo = repo();
        let (app, _) = routers(app_state(Some(repo.clone()), &[7411]));
        let call = |n: &str, a: &str| tool(n, a, "127.0.0.1:7411");
        let (_, body) = send(&app, call("install_status", "{}")).await;
        assert!(body.contains(r#""skill":"missing""#) && body.contains("spec17 install"), "{body}");
        let response: serde_json::Value = serde_json::from_str(&body).unwrap();
        let cli = <Cli as clap::CommandFactory>::command();
        assert_eq!(response["result"]["structuredContent"]["version"], VERSION, "{body}");
        assert_eq!(cli.render_version().lines().next(), cli.render_long_version().lines().next());
        std::fs::write(repo.join(".mcp.json"), r#"{"mcpServers":{"other":{"command":"x"}}}"#).unwrap();
        let (st, body) = send(&app, call("install", "{}")).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert!(body.contains(r#""skill":"current""#) && body.contains(r#""mcp":"ok""#), "{body}");
        let mcp = std::fs::read_to_string(repo.join(".mcp.json")).unwrap();
        assert!(mcp.contains("\"other\"") && mcp.contains("127.0.0.1:7411/mcp"), "{mcp}");
        let skill = repo.join(".agents/skills/spec17/SKILL.md");
        for invalid in [
            r#"{"only":"unknown"}"#,
            r#"{"only":"mcp","global":"true"}"#,
            r#"{"only":"mcp","server":"codegrph"}"#,
            r#"{"only":null}"#,
            r#"{"server":"codegraph","global":true}"#,
        ] {
            let (_, body) = send(&app, call("uninstall", invalid)).await;
            assert!(body.contains(r#""isError":true"#) && skill.exists(), "{invalid}: {body}");
            assert_eq!(std::fs::read_to_string(repo.join(".mcp.json")).unwrap(), mcp);
        }
        let (_, body) = send(&app, call("uninstall", r#"{"only":"skill"}"#)).await;
        assert!(body.contains(r#""skill":"missing""#) && !skill.exists(), "{body}");
        assert_eq!(std::fs::read_to_string(repo.join(".mcp.json")).unwrap(), mcp);
        send(&app, call("install", r#"{"port":7500}"#)).await;
        let (_, body) = send(&app, call("uninstall", r#"{"only":"mcp"}"#)).await;
        assert!(body.contains(r#""mcp":"missing""#) && skill.exists(), "{body}");
        let mcp = std::fs::read_to_string(repo.join(".mcp.json")).unwrap();
        assert!(mcp.contains("\"other\"") && !mcp.contains("\"spec17\""), "{mcp}");
        let (_, body) = send(&app, call("uninstall", "{}")).await;
        assert!(body.contains(r#""skill":"missing""#), "{body}");
        assert!(!repo.join(".agents/skills/spec17/SKILL.md").exists());
        assert!(std::fs::read_to_string(repo.join(".mcp.json")).unwrap().contains("\"other\""));
    }

    #[doc = "[SCAN-28, SETUP-06, SETUP-10]"]
    #[tokio::test]
    async fn a_repo_with_no_specs_serves_every_tool_and_installs_codegraph() {
        let d = std::env::temp_dir().join(format!("spec17-srv-nospecs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/a.py"), "def a():\n    pass\n").unwrap();
        let (app, _) = routers(app_state(Some(d.clone()), &[7411]));
        let call = |n: &str, a: &str| tool(n, a, "127.0.0.1:7411");
        for (name, args) in [("report", "{}"), ("status", r#"{"all":true}"#), ("doctor", "{}"), ("install_status", "{}")] {
            let (st, body) = send(&app, call(name, args)).await;
            assert!(st == StatusCode::OK && !body.contains(r#""isError":true"#), "{name}: {body}");
        }
        let (_, body) = send(&app, call("doctor", "{}")).await;
        assert!(body.contains("no specs found") && !body.contains(r#"\"level\":\"fail\""#), "{body}");
        let (_, body) = send(&app, call("install", r#"{"server":"codegraph"}"#)).await;
        assert!(body.contains(r#""codegraph_mcp":"ok""#) && !body.contains(r#""isError":true"#), "{body}");
        let mcp = std::fs::read_to_string(d.join(".mcp.json")).unwrap();
        assert!(mcp.contains("\"codegraph\"") && !mcp.contains("spec17"), "{mcp}");
        assert!(!d.join(".agents").exists(), "server=codegraph must not install the skill");
        let (_, body) = send(&app, call("uninstall", r#"{"server":"codegraph"}"#)).await;
        assert!(body.contains(r#""codegraph_mcp":"missing""#) && !body.contains(r#""isError":true"#), "{body}");
        let mcp: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(d.join(".mcp.json")).unwrap()).unwrap();
        assert_eq!(mcp, json!({"mcpServers": {}}));
        send(&app, call("install", r#"{"server":"codegraph"}"#)).await;
        let (_, body) = send(&app, call("codegraph", r#"{"action":"refresh"}"#)).await;
        assert!(body.contains("nothing to refresh"), "{body}");
    }

    #[doc = "[API-01, API-04, API-07, API-18, API-19, API-24]"]
    #[tokio::test]
    async fn mcp_tools_and_ui_resource() {
        let (app, _) = routers(app_state(Some(repo()), &[7411]));
        let rpc = |id: u32, method: &str, params: &str| post("/mcp", "127.0.0.1:7411", &format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#));
        let init = r#"{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"1"}}"#;
        let (st, body) = send(&app, rpc(1, "initialize", init)).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        // (API-24)
        let info: serde_json::Value = serde_json::from_str(&body).unwrap();
        let server = &info["result"]["serverInfo"];
        assert_eq!((server["name"].as_str(), server["version"].as_str(), server["websiteUrl"].as_str()), (Some("spec17"), Some(env!("CARGO_PKG_VERSION")), Some("https://spec17.dev")), "{body}");
        let (_, body) = send(&app, rpc(2, "tools/list", "{}")).await;
        assert!(body.contains("report") && body.contains("ui://spec17/dashboard"), "{body}");
        // (API-18)
        let listed: serde_json::Value = serde_json::from_str(&body).unwrap();
        for tool in listed["result"]["tools"].as_array().unwrap() {
            let name = tool["name"].as_str().unwrap();
            assert_eq!(tool["annotations"]["readOnlyHint"], !matches!(name, "install" | "uninstall" | "codegraph" | "module"), "{name}");
            if name == "search_tests" {
                assert_eq!(tool["inputSchema"]["properties"]["requirement_id"]["type"], "string");
            }
        }
        assert_eq!(body.matches("ui://spec17/dashboard").count(), 1, "only `dashboard` opens the app: {body}");
        let (_, body) = send(&app, rpc(5, "tools/call", r#"{"name":"dashboard","arguments":{"view":"/modules?module=core"}}"#)).await;
        assert!(body.contains("127.0.0.1:7411/modules?module=core"), "{body}");
        let (_, body) = send(&app, rpc(3, "tools/call", r#"{"name":"status","arguments":{"missing":true}}"#)).await;
        assert!(body.contains("AAA-02"), "{body}");
        let (_, body) = send(&app, rpc(4, "resources/read", r#"{"uri":"ui://spec17/dashboard"}"#)).await;
        assert!(body.contains("text/html;profile=mcp-app"), "{body}");
        // (API-07)
        let (_, body) = send(&app, rpc(6, "resources/list", "{}")).await;
        assert!(body.contains("spec17://modules") && body.contains("spec17://tests"), "{body}");
        let (_, body) = send(&app, rpc(7, "resources/read", r#"{"uri":"spec17://modules"}"#)).await;
        assert!(body.contains("application/json") && body.contains("modules"), "{body}");
        let (_, body) = send(&app, rpc(8, "resources/read", r#"{"uri":"spec17://tests"}"#)).await;
        assert!(body.contains("not run") && body.contains("total"), "{body}");
        let (_, body) = send(&app, rpc(10, "tools/call", r#"{"name":"search_tests","arguments":{"state":"untagged","query":"no such test"}}"#)).await;
        assert!(body.contains("not run") && body.contains("total") && body.contains('0'), "{body}");
        let (_, body) = send(&app, rpc(9, "resources/read", r#"{"uri":"spec17://nope"}"#)).await;
        assert!(body.contains("unknown resource"), "{body}");
        let mut bad = rpc(5, "tools/list", "{}");
        bad.headers_mut().insert("host", "evil.test".parse().unwrap());
        assert_eq!(send(&app, bad).await.0, StatusCode::FORBIDDEN);
    }

    #[doc = "[API-20]"]
    #[tokio::test]
    async fn search_requirements_by_test_file_and_line() {
        let d = std::env::temp_dir().join(format!("spec17-requirement-search-{}", std::process::id()));
        std::fs::create_dir_all(d.join("specs")).unwrap();
        std::fs::create_dir_all(d.join("tests")).unwrap();
        let specs = (1..=4).map(|i| format!("- **AAA-{i:02}** MUST — example {i}.\n")).collect::<String>();
        std::fs::write(d.join("specs/example.md"), specs).unwrap();
        let file = "tests/example_test.py";
        std::fs::write(d.join(file), concat!(
            "# Test cases\n\n",
            "def test_first(): # [AAA\x2D01, AAA\x2D02]\n    assert True # [AAA\x2D03] body tag\n\n",
            "def test_plain():\n    assert True\n\n",
            "# [AAA\x2D04]\ndef test_last(): # [AAA\x2D02, AAA\x2D99]\n    assert True\n",
        )).unwrap();
        let (app, _) = routers(app_state(Some(d.clone()), &[7411]));
        let (st, body) = send(&app, tool("search_requirements", &json!({"test_file": file, "line": 4}).to_string(), "127.0.0.1:7411")).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        let response: serde_json::Value = serde_json::from_str(&body).unwrap();
        let data = &response["result"]["structuredContent"];
        assert_eq!(data["rows"][0]["id"], "AAA-01", "{body}");
        let schema = ops::output_schema("search_requirements").unwrap();
        assert_eq!(schema["required"], json!(["rows"]));

        for (location, expected) in [
            (json!({}), vec!["AAA-01", "AAA-02", "AAA-03", "AAA-04"]),
            (json!({"line": 1}), vec![]),
            (json!({"line": 4}), vec!["AAA-01", "AAA-02", "AAA-03"]),
            (json!({"line": 6}), vec![]),
            (json!({"line": 10}), vec!["AAA-02", "AAA-04"]),
            (json!({"line": 1, "end_line": 3}), vec!["AAA-01", "AAA-02", "AAA-03"]),
            (json!({"line": 4, "end_line": 9}), vec!["AAA-01", "AAA-02", "AAA-03"]),
            (json!({"line": 4, "end_line": 10}), vec!["AAA-01", "AAA-02", "AAA-03", "AAA-04"]),
        ] {
            let mut args = location;
            args["test_file"] = json!(file);
            let (_, data) = ops::call(Some(&d), "search_requirements", &args, "", 7411).unwrap();
            let ids: Vec<_> = data["rows"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
            assert_eq!(ids, expected, "{args}");
        }
        for bad in [json!({}), json!({"test_file":"../example_test.py"}), json!({"test_file":file,"line":0}), json!({"test_file":file,"end_line":4}), json!({"test_file":file,"line":4,"end_line":3})] {
            assert!(ops::call(Some(&d), "search_requirements", &bad, "", 7411).is_err(), "{bad}");
        }
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[doc = "[API-07, API-19]"]
    #[test]
    fn search_detected_tests() {
        let d = std::env::temp_dir().join(format!("spec17-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for dir in ["alpha/nested/src", "alpha/nested/tests", "beta/tests"] {
            std::fs::create_dir_all(d.join(dir)).unwrap();
        }
        std::fs::write(d.join("alpha/nested/src/lib.py"), "def value():\n    return 1\n").unwrap();
        std::fs::write(d.join("beta/pyproject.toml"), "").unwrap();
        let plain_file = "alpha/nested/tests/example_test.py";
        let plain_test = "def test_plain():\n    assert True\n";
        std::fs::write(d.join(plain_file), plain_test).unwrap();
        std::fs::write(d.join("beta/tests/example_test.py"), "def test_tagged(): # [AAA\x2D01]\n    assert True\n").unwrap();
        let (_, all) = crate::ops::call(Some(&d), "search_tests", &serde_json::json!({}), "", 7411).unwrap();
        assert_eq!(all["total"], 2);
        let (_, plain) = crate::ops::call(Some(&d), "search_tests", &serde_json::json!({"state":"untagged", "query":"PLAIN"}), "", 7411).unwrap();
        assert_eq!(plain["total"], 1);
        let (_, none) = crate::ops::call(Some(&d), "search_tests", &serde_json::json!({"state":"tagged", "query":"plain"}), "", 7411).unwrap();
        assert_eq!(none["total"], 0);
        let (_, beta_tests) = crate::ops::call(Some(&d), "search_tests", &serde_json::json!({"module":"beta"}), "", 7411).unwrap();
        assert_eq!(beta_tests["total"], 1);
        assert_eq!(beta_tests["modules"][0]["module"], "beta");
        let (_, by_id) = crate::ops::call(Some(&d), "search_tests", &serde_json::json!({"requirement_id":"aaa-01", "query":"tagged"}), "", 7411).unwrap();
        assert_eq!(by_id["total"], 1);
        assert_eq!(by_id["modules"][0]["module"], "beta");
        let (_, partial_id) = crate::ops::call(Some(&d), "search_tests", &serde_json::json!({"requirement_id":"AAA-0"}), "", 7411).unwrap();
        assert_eq!(partial_id["total"], 0);
        let (_, other_module) = crate::ops::call(Some(&d), "search_tests", &serde_json::json!({"requirement_id":"AAA-01", "module":"alpha"}), "", 7411).unwrap();
        assert_eq!(other_module["total"], 0);
        let (_, found) = crate::ops::call(Some(&d), "search_modules", &serde_json::json!({"query":"BETA"}), "", 7411).unwrap();
        assert_eq!(found["modules"].as_array().unwrap().len(), 1);
        assert_eq!(found["modules"][0]["path"], "beta");
        let (_, nested) = crate::ops::call(Some(&d), "search_modules", &serde_json::json!({"query":"nested"}), "", 7411).unwrap();
        assert_eq!(nested["modules"][0]["path"], "alpha/nested");

        let git = |args: &[&str]| assert!(std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).current_dir(&d).output().unwrap().status.success(), "{args:?}");
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-qm", "baseline"]);
        std::fs::write(d.join(plain_file), format!("{plain_test}    assert 1 == 1\n")).unwrap();

        let (_, tests) = crate::ops::call(Some(&d), "search_tests", &serde_json::json!({"changed":true}), "", 7411).unwrap();
        assert_eq!(tests["total"], 1);
        assert_eq!(tests["modules"][0]["files"][0]["file"], plain_file);
        let (_, modules) = crate::ops::call(Some(&d), "search_modules", &serde_json::json!({"changed":true}), "", 7411).unwrap();
        assert_eq!(modules["modules"].as_array().unwrap().len(), 1);
        assert_eq!(modules["modules"][0]["path"], "alpha");
    }

    #[doc = "[API-05]"]
    #[tokio::test]
    async fn split_ports_separate_mcp_from_ui() {
        let (mcp_app, ui) = routers(app_state(Some(repo()), &[7411, 7412]));
        let ui = ui.unwrap();
        // the UI port serves the info route and MCP (the dashboard talks MCP); the main port serves MCP only
        let get = |host: &str| Request::get("/api/info").header("host", host).body(Body::empty()).unwrap();
        assert_eq!(send(&ui, get("127.0.0.1:7412")).await.0, StatusCode::OK);
        let (st, body) = send(&ui, tool("report", "{}", "127.0.0.1:7412")).await;
        assert!(st == StatusCode::OK && body.contains("SDD STATUS"), "{body}");
        assert_eq!(send(&mcp_app, get("127.0.0.1:7411")).await.0, StatusCode::NOT_FOUND);
        assert!(send(&mcp_app, tool("report", "{}", "127.0.0.1:7411")).await.1.contains("SDD STATUS"));
    }

    /// Release builds only: the embedded UI, SPA fallback, 404 for a missing asset.
    #[doc = "[API-06]"]
    #[cfg(not(debug_assertions))]
    #[tokio::test]
    async fn embedded_ui() {
        let (app, _) = routers(app_state(Some(repo()), &[7411]));
        let get = |uri: &str| Request::get(uri).header("host", "127.0.0.1:7411").body(Body::empty()).unwrap();
        let (st, body) = send(&app, get("/")).await;
        assert!(st == StatusCode::OK && body.contains("<div id=\"app\">"), "{st} {body:.200}");
        assert_eq!(send(&app, get("/requirements/AAA-01")).await.1, body); // deep link -> index.html
        assert_eq!(send(&app, get("/assets/nope.js")).await.0, StatusCode::NOT_FOUND);
    }
}
