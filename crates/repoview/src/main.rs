use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, ExitCode, Stdio};
use std::sync::Arc;

use clap::{Args, Parser, Subcommand, ValueEnum};
use repoview::assets::EmbeddedAssets;
use repoview::browser::{RedirectPage, opener};
use repoview::project::{discover, snapshot};
use repoview::server::{AppState, bind, new_token, router};
use repoview_sources::{Availability, Env, Section, all};

/// A read-only browser view of the project in the working directory.
///
/// Without a subcommand, `repoview` is `repoview open`.
#[derive(Parser)]
#[command(name = "repoview", version, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    open: OpenArgs,
}

#[derive(Subcommand)]
enum Command {
    /// Serve the project on 127.0.0.1 and open the browser at a URL carrying the run token.
    Open(OpenArgs),
    /// Print the whole read model and exit.
    Snapshot(SnapshotArgs),
    /// List each source: detected or not, tool path and version, or why not.
    Doctor(RootArg),
}

#[derive(Args)]
struct RootArg {
    /// Project root; overrides discovery (the Git top level above the working directory, else
    /// the working directory).
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
}

#[derive(Args)]
struct OpenArgs {
    /// Port on 127.0.0.1; 0 picks a free one.
    #[arg(long, default_value_t = 0)]
    port: u16,
    /// Print the URL without opening a browser.
    #[arg(long)]
    no_browser: bool,
    #[command(flatten)]
    root: RootArg,
}

#[derive(Args)]
struct SnapshotArgs {
    #[arg(long, value_enum, default_value_t = Format::Json)]
    format: Format,
    #[command(flatten)]
    root: RootArg,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command.unwrap_or(Command::Open(cli.open)) {
        Command::Open(args) => open(args),
        Command::Snapshot(args) => print_snapshot(args),
        Command::Doctor(args) => doctor(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("repoview: {message}");
            ExitCode::FAILURE
        }
    }
}

fn project_root(arg: &RootArg) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|error| format!("working directory: {error}"))?;
    discover(&cwd, arg.root.as_deref()).map_err(|error| match &arg.root {
        Some(root) => format!("--root {}: {error}", root.display()),
        None => format!("project discovery from {}: {error}", cwd.display()),
    })
}

fn print_snapshot(args: SnapshotArgs) -> Result<(), String> {
    let Format::Json = args.format;
    let root = project_root(&args.root)?;
    let document = snapshot(&Env::new(root));
    let text = serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?;
    println!("{text}");
    Ok(())
}

fn doctor(args: RootArg) -> Result<(), String> {
    let env = Env::new(project_root(&args)?);
    for source in all() {
        let section = source.read(&env);
        let detail = doctor_detail(&section, source.detects());
        println!(
            "{:<8} {:<12} {detail}",
            section.source_id,
            section.availability.as_str()
        );
    }
    Ok(())
}

fn doctor_detail(section: &Section, detects: &str) -> String {
    match section.availability {
        Availability::Absent => format!("not found: {detects}"),
        Availability::ToolMissing | Availability::Failed => {
            one_line(section.diagnostic.as_deref().unwrap_or("no diagnostic"))
        }
        Availability::Present => match (&section.tool_path, &section.tool_version) {
            (Some(path), Some(version)) => format!("{path} {}", one_line(version)),
            (Some(path), None) => format!("{path} (version not reported)"),
            _ => "(no tool)".to_owned(),
        },
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn open(args: OpenArgs) -> Result<(), String> {
    let root = project_root(&args.root)?;
    let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    runtime.block_on(serve(root, args.port, !args.no_browser))
}

async fn serve(root: PathBuf, port: u16, browser: bool) -> Result<(), String> {
    let listener = bind(port)
        .await
        .map_err(|error| format!("bind 127.0.0.1:{port}: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let token = new_token();
    let project = root.clone();
    // The API page modules read the project through this request extension.
    let env = Env::new(root.clone());
    let state = AppState {
        token: token.clone(),
        port,
        assets: Arc::new(EmbeddedAssets),
        snapshot: Arc::new(move || {
            serde_json::to_value(snapshot(&Env::new(root.clone())))
                .expect("a snapshot serialises to JSON")
        }),
    };
    let mut signals = Signals::install().map_err(|error| format!("signal handlers: {error}"))?;
    let url = format!("http://127.0.0.1:{port}/?token={token}");
    let mut stdout = std::io::stdout();
    writeln!(stdout, "{url}")
        .and_then(|()| stdout.flush())
        .map_err(|error| error.to_string())?;
    let page = if browser {
        launch_browser(&url, &project)
    } else {
        None
    };
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        signals.next().await;
        drop(page);
        let _ = stop.send(());
        tokio::select! {
            () = tokio::time::sleep(SHUTDOWN_GRACE) => {}
            () = signals.next() => {}
        }
        std::process::exit(0);
    });
    axum::serve(listener, router(state).layer(axum::Extension(env)))
        .with_graceful_shutdown(async move {
            let _ = stopped.await;
        })
        .await
        .map_err(|error| error.to_string())
}

/// How long open connections may take to finish after the first signal.
const SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

/// SIGINT, SIGTERM, SIGHUP and SIGQUIT, installed before the URL is printed. The first starts a
/// graceful shutdown bounded by [`SHUTDOWN_GRACE`]; a second exits at once. Both delete the
/// redirect page first.
struct Signals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
    hangup: tokio::signal::unix::Signal,
    quit: tokio::signal::unix::Signal,
}

impl Signals {
    fn install() -> std::io::Result<Signals> {
        use tokio::signal::unix::{SignalKind, signal};
        Ok(Signals {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
            hangup: signal(SignalKind::hangup())?,
            quit: signal(SignalKind::quit())?,
        })
    }

    async fn next(&mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {}
            _ = self.terminate.recv() => {}
            _ = self.hangup.recv() => {}
            _ = self.quit.recv() => {}
        }
    }
}

/// Open the browser on a private redirect page to `url`, so the token never appears in the
/// opener's argv. A failure is reported and the server keeps running; the page lives until the
/// returned value is dropped.
fn launch_browser(url: &str, project: &Path) -> Option<RedirectPage> {
    let Some(program) = opener() else {
        eprintln!("repoview: no browser opener found ($BROWSER, xdg-open or open); open {url}");
        return None;
    };
    let page = match RedirectPage::create(url, project) {
        Ok(page) => page,
        Err(error) => {
            eprintln!("repoview: could not write the browser redirect page ({error}); open {url}");
            return None;
        }
    };
    let spawned = Process::new(&program)
        .arg(page.url())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match spawned {
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
            Some(page)
        }
        Err(error) => {
            eprintln!(
                "repoview: could not start {} ({error}); open {url}",
                program.display()
            );
            None
        }
    }
}
