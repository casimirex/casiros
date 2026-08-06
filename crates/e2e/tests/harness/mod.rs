//! Spawns the real CASIROS binaries and tears them down reliably.
//
// This module is included into each test binary rather than exported from a
// library, so its `pub` items are reachable only from the sibling test files.
// That is the intended shape for a test harness; the workspace's
// `unreachable_pub` lint has no better alternative to offer here.
#![allow(unreachable_pub)]
#![allow(dead_code)]
// Not every test binary uses every helper.
// The library crates use explicit returns for auditability; match that here.
#![allow(clippy::needless_return)]

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// API key used by every smoke test.
pub const API_KEY: &str = "smoke-key";

/// Admin key used by the admin-surface tests.
pub const ADMIN_KEY: &str = "smoke-admin";

/// Tenant the API key maps to.
pub const TENANT: &str = "tenant_smoke";

/// Workspace the API key maps to.
pub const WORKSPACE: &str = "workspace_smoke";

/// Counter making each test's tenant/database artifacts distinguishable.
static SEQ: AtomicU32 = AtomicU32::new(0);

/// Returns the Postgres URL the tests should use.
///
/// Falls back to the docker-compose default so a developer who has run
/// `docker compose up -d postgres` needs no further setup.
pub fn postgres_url() -> String {
    return std::env::var("CASIROS__POSTGRES__URL")
        .or_else(|_| std::env::var("CASIROS_POSTGRES__URL"))
        .unwrap_or_else(|_| "postgresql://casiros:casiros@localhost:5432/casiros".to_string());
}

/// Locates a binary built by this workspace.
///
/// Integration tests run from `target/<profile>/deps`, so the binaries sit two
/// directories up.
fn binary(name: &str) -> PathBuf {
    let mut dir = std::env::current_exe().expect("test executable path is known");
    dir.pop(); // deps/
    if dir.ends_with("deps") {
        dir.pop();
    }
    let path = dir.join(name);
    assert!(
        path.exists(),
        "{} not found at {}. Build it first: cargo build -p {}",
        name,
        path.display(),
        if name == "casiros-api" {
            "casiros-api"
        } else {
            "casiros-worker"
        }
    );
    return path;
}

/// Claims a free TCP port by binding to port 0 and reading the assignment.
///
/// There is an unavoidable race between releasing the port and the server
/// binding it, but the window is small and the alternative — a fixed port —
/// makes concurrent test runs collide outright.
/// Returns the workspace root, derived from the test binary's location.
///
/// The binary is at `<root>/target/<profile>/deps/<name>`; popping the file
/// name plus three directories reaches the root. Walking up until `web/` is
/// found instead keeps this correct if the layout ever changes.
fn repo_root() -> PathBuf {
    let mut dir = std::env::current_exe().expect("test executable path is known");
    while dir.pop() {
        if dir.join("web").join("index.html").is_file() {
            return dir;
        }
    }
    panic!("could not locate the workspace root from the test binary path");
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("can bind an ephemeral port");
    return listener
        .local_addr()
        .expect("listener has an address")
        .port();
}

/// A running `casiros-api` process bound to its own port.
///
/// The process is killed on drop, so a panicking test cannot leave a server
/// running and block the next one.
pub struct ApiServer {
    child: Child,
    /// Base URL, e.g. `http://127.0.0.1:41234`.
    pub base: String,
    /// Log lines captured from the process, so tests can assert on the
    /// backend the server actually selected.
    log: Arc<Mutex<Vec<String>>>,
}

impl ApiServer {
    /// Starts the API with the Postgres backend and a known key mapping.
    ///
    /// # Panics
    ///
    /// Panics if the binary is missing or does not become healthy in time.
    #[must_use]
    pub fn start_postgres() -> Self {
        return Self::start(&[
            ("CASIROS__SNAPSHOT__BACKEND", "postgres".to_string()),
            ("CASIROS__POSTGRES__URL", postgres_url()),
        ]);
    }

    /// Starts the API with the default in-memory backend.
    ///
    /// # Panics
    ///
    /// Panics if the binary is missing or does not become healthy in time.
    #[must_use]
    pub fn start_memory() -> Self {
        return Self::start(&[]);
    }

    /// Starts the API with its working directory set elsewhere.
    ///
    /// Used to prove that nothing the server serves depends on where it was
    /// launched from.
    ///
    /// # Panics
    ///
    /// Panics if the binary is missing or does not become healthy in time.
    #[must_use]
    pub fn start_from_dir(cwd: &std::path::Path) -> Self {
        return Self::start_inner(&[], Some(cwd));
    }

    /// Starts the API with extra environment variables layered on the defaults.
    ///
    /// # Panics
    ///
    /// Panics if the binary is missing or does not become healthy in time.
    #[must_use]
    pub fn start(extra: &[(&str, String)]) -> Self {
        return Self::start_inner(extra, None);
    }

    /// Shared spawn path for [`start`](Self::start) and
    /// [`start_from_dir`](Self::start_from_dir).
    fn start_inner(extra: &[(&str, String)], cwd: Option<&std::path::Path>) -> Self {
        // Retry on port collision. Concurrent tests can be handed the same
        // ephemeral port; the loser exits with AddrInUse and must try again
        // rather than attach itself to the winner's server.
        for attempt in 0..5 {
            if let Some(server) = Self::try_start(extra, cwd) {
                return server;
            }
            std::thread::sleep(Duration::from_millis(50 * (attempt + 1)));
        }
        panic!("casiros-api failed to bind a free port after 5 attempts");
    }

    /// One spawn attempt. Returns `None` if the process exited before becoming
    /// healthy, which in practice means the port was taken.
    fn try_start(extra: &[(&str, String)], cwd: Option<&std::path::Path>) -> Option<Self> {
        let port = free_port();
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);

        let mut cmd = Command::new(binary("casiros-api"));
        cmd.env("CASIROS__BIND_ADDR", format!("127.0.0.1:{port}"))
            .env("CASIROS__LOG_LEVEL", "info")
            .env("CASIROS_API_KEYS", API_KEY)
            .env(
                "CASIROS_API_KEY_TENANTS",
                format!("{API_KEY}:{TENANT}:{WORKSPACE}:100000"),
            )
            .env("CASIROS_ADMIN_KEY", ADMIN_KEY)
            .env("CASIROS_SEQ", seq.to_string())
            // Cargo runs test binaries from an arbitrary directory, so point
            // the server at the repo's web/ explicitly rather than relying on
            // the working directory.
            .env("CASIROS_WEB_DIR", repo_root().join("web"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (k, v) in extra {
            cmd.env(k, v);
        }
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn().expect("casiros-api starts");

        // `tracing_subscriber::fmt()` writes to stdout, not stderr. Reading the
        // wrong stream blocks forever, because nothing is ever written to it.
        //
        // Drain on a thread rather than inline: the pipe has a fixed buffer, so
        // a server that keeps logging past the lines we care about would block
        // on a full pipe if nobody kept reading.
        let stdout = child.stdout.take().expect("stdout is piped");
        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&log);
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                sink.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(line);
            }
        });

        let mut server = Self {
            child,
            base: format!("http://127.0.0.1:{port}"),
            log,
        };
        if server.await_healthy() {
            return Some(server);
        }
        let _ = server.child.kill();
        let _ = server.child.wait();
        return None;
    }

    /// Returns the startup log lines captured so far.
    #[must_use]
    pub fn startup_log(&self) -> Vec<String> {
        return self
            .log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
    }

    /// Blocks until this server's `/healthz` answers.
    ///
    /// Returns `false` if the child exits first — the signal that the port was
    /// already taken. Checking the child rather than only polling the socket
    /// matters: on a collision the port *is* serving, just not by us.
    fn await_healthy(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(30);
        let url = format!("{}/healthz", self.base);
        while Instant::now() < deadline {
            // An exited child (or an unreadable one) means this server never
            // took the port — almost always a collision with a sibling test.
            if !matches!(self.child.try_wait(), Ok(None)) {
                return false;
            }
            if let Ok(resp) = reqwest::blocking::get(&url)
                && resp.status().is_success()
            {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("casiros-api never became healthy at {}", self.base);
    }

    /// Returns true if `needle` is already present, without waiting.
    ///
    /// Use for negative assertions, where polling would cost the full timeout
    /// on every passing run.
    #[must_use]
    pub fn logged_now(&self, needle: &str) -> bool {
        return self.startup_log().iter().any(|l| l.contains(needle));
    }

    /// Returns true if any captured log line contains `needle`.
    ///
    /// Polls briefly: `/healthz` can answer before the reader thread has
    /// drained every startup line, so a bare check races the logger.
    #[must_use]
    pub fn logged(&self, needle: &str) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if self.startup_log().iter().any(|l| l.contains(needle)) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for ApiServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A running `casiros-worker` process.
pub struct Worker {
    child: Child,
}

impl Worker {
    /// Starts a worker pointed at the same database the API uses.
    ///
    /// # Panics
    ///
    /// Panics if the worker binary is missing.
    #[must_use]
    pub fn start() -> Self {
        let child = Command::new(binary("casiros-worker"))
            .env("CASIROS__POSTGRES__URL", postgres_url())
            .env("CASIROS__LOG_LEVEL", "info")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("casiros-worker starts");
        return Self { child };
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A blocking HTTP client carrying the smoke-test API key.
///
/// # Panics
///
/// Panics if the client cannot be constructed.
#[must_use]
pub fn client() -> reqwest::blocking::Client {
    return reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("HTTP client builds");
}

/// Builds a single-formula evaluate request, the shape most tests need.
#[must_use]
pub fn formula_request(kind: &serde_json::Value) -> serde_json::Value {
    return serde_json::json!({
        "nodes": [{"formula": {"name": "result", "kind": kind}}],
        "edges": [],
        "inputs": {}
    });
}
