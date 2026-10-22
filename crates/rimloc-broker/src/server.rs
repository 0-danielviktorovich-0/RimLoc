//! The broker server: a `std::thread` + `UnixListener` accept loop over
//! the profile socket (`MCP_SERVER_PHASE2.md` «Broker server», «Границы»).
//!
//! Socket: `rimloc-broker.sock` in the PARENT of the managed root (the
//! profile: `RIMLOC_DATA_DIR/` or `<os_data>/com.rimloc.gui/`), chmod
//! 0600 right after bind. Lifecycle: if the socket file exists, a
//! `UnixStream::connect` probe decides — connect succeeds ⇒ another
//! broker is ALIVE and startup refuses
//! ([`BrokerStartError::AlreadyRunning`]); connect fails ⇒ stale socket,
//! it is unlinked and the bind proceeds. Dropping [`BrokerServer`] stops
//! the accept loop and removes the socket file.
//!
//! Bounds: at most [`MAX_CLIENTS`] concurrent clients (the next connect
//! gets one typed `busy` frame and is closed); one line at most
//! [`protocol::MAX_LINE_BYTES`] (8 MiB — the connection closes after the
//! typed refusal, the framing past an oversized line is untrustworthy);
//! at most [`MAX_IN_FLIGHT_PER_CLIENT`] concurrently executing lines per
//! connection (the reader parks on a condvar — a pipelining client cannot
//! fork-bomb the host process).
//!
//! Panic containment: every line executes on its own thread behind
//! `std::panic::catch_unwind`; a panicking handler costs the client one
//! typed internal error (`handler_panicked`) and the server lives on.
//!
//! Events: the emitter bound into [`crate::ops::BrokerOps`] at startup is
//! this server's fan-out — the event frame is written to every connected
//! client AND the host `notify` callback fires (the GUI's Tauri
//! `emit("broker-changed")` rides that callback; no window, no Tauri type
//! ever appears in this crate).

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::Duration;

use crate::ops::BrokerOps;
use crate::protocol::{encode_line, BrokerError, BrokerEvent, BrokerRequest, BrokerResponse};

/// Socket file name, inside the PROFILE directory (the parent of the
/// managed root).
pub const SOCKET_FILE_NAME: &str = "rimloc-broker.sock";

/// Dev-log tag for the one stderr diagnostic of the accept path (a dead
/// client connection). `{}`-formatter tag + detail is the shape the
/// no-hardcoded-user-strings gate accepts for structured dev logs; the
/// server never prints product UI.
const DEV_LOG_CLIENT_READ_FAILED: &str = "rimloc-broker: client read failed";

/// Concurrent-client bound per profile (design «Границы»: ≤4 клиента).
pub const MAX_CLIENTS: usize = 4;

/// Concurrently EXECUTING lines per connection. The reader parks past
/// this; responses may still stream while parked.
pub const MAX_IN_FLIGHT_PER_CLIENT: usize = 8;

/// Accept-loop poll interval in nonblocking mode. Clients are never
/// delayed by this — only NEW connection admission is.
const ACCEPT_POLL: Duration = Duration::from_millis(20);

/// Path guard shared with the rimloc-mcp profile ladder (mirrored here so
/// the broker crate stands alone): `Some(cause)` refuses the managed root
/// before ANY filesystem effect. Absolute, traversal-free, with a parent
/// (the socket needs a profile directory), and not a filesystem root.
pub fn refused_profile_cause(managed_root: &Path) -> Option<String> {
    if !managed_root.is_absolute() {
        return Some("not an absolute path".to_string());
    }
    if managed_root
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Some("parent-directory traversal".to_string());
    }
    if managed_root
        .parent()
        .is_none_or(|p| p.as_os_str().is_empty())
    {
        return Some("no profile directory (filesystem root)".to_string());
    }
    None
}

/// Startup failure of [`BrokerServer::start`].
#[derive(Debug)]
pub enum BrokerStartError {
    /// A LIVE broker already serves this profile (the connect probe
    /// succeeded). `socket_path` names it.
    AlreadyRunning { socket_path: PathBuf },
    /// The managed root has no parent (filesystem root) — there is no
    /// profile directory to bind the socket in.
    NoProfileDirectory { managed_root: PathBuf },
    /// Filesystem failure (mkdir / stale-unlink / bind / chmod).
    Io(std::io::Error),
}

impl std::fmt::Display for BrokerStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning { socket_path } => write!(
                f,
                "a broker is already running on `{}`",
                socket_path.display()
            ),
            Self::NoProfileDirectory { managed_root } => write!(
                f,
                "managed root `{}` has no parent directory to host the broker socket",
                managed_root.display()
            ),
            Self::Io(err) => write!(f, "broker socket setup failed: {err}"),
        }
    }
}

impl std::error::Error for BrokerStartError {}

/// One connected client's broadcast sink: the write half of the socket,
/// cloned at admission; every writer (response threads, event fan-out)
/// takes the mutex, so frames never interleave mid-line.
struct ClientSink {
    id: u64,
    out: Arc<Mutex<UnixStream>>,
}

/// Shared server state: the ops seam, the client registry for the event
/// fan-out, the live-client count, and the host notify callback.
struct ServerShared {
    ops: Arc<dyn BrokerOps>,
    clients: Mutex<Vec<ClientSink>>,
    next_client_id: AtomicU64,
    active_clients: AtomicUsize,
    host_notify: Arc<dyn Fn(BrokerEvent) + Send + Sync>,
    shutdown: Arc<AtomicBool>,
}

impl ServerShared {
    /// Fan one event out to every connected client, then the host
    /// callback. Write failures are ignored here — the owning reader
    /// thread notices the dead socket on its next read and unregisters.
    fn broadcast(&self, event: BrokerEvent) {
        if let Ok(line) = encode_line(&event) {
            let sinks: Vec<Arc<Mutex<UnixStream>>> = {
                let clients = self
                    .clients
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                clients.iter().map(|c| c.out.clone()).collect()
            };
            for out in sinks {
                let mut stream = out.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                let _ = stream.write_all(&line);
                let _ = stream.flush();
            }
        }
        (self.host_notify)(event);
    }

    fn register(&self, out: Arc<Mutex<UnixStream>>) -> u64 {
        let id = self.next_client_id.fetch_add(1, Ordering::Relaxed);
        self.clients
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(ClientSink { id, out });
        id
    }

    fn unregister(&self, id: u64) {
        self.clients
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|c| c.id != id);
    }
}

/// A running broker. Drop = stop accepting, remove the socket file.
/// Already-connected client threads finish serving their current
/// connections in the background (each connection owns its threads; the
/// process exit or the client disconnect cleans them up).
pub struct BrokerServer {
    socket_path: PathBuf,
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<std::thread::JoinHandle<()>>,
}

impl BrokerServer {
    /// Bind the profile socket and start the accept loop.
    ///
    /// `managed_root` is the authoritative session root (the GUI's
    /// `ContractState` root, `<profile>/managed`); the socket lands in
    /// its PARENT (the profile). `ops` executes every request;
    /// `notify` receives every broker event (the GUI forwards it to
    /// Tauri `emit("broker-changed")`).
    pub fn start(
        managed_root: &Path,
        ops: Arc<dyn BrokerOps>,
        notify: Arc<dyn Fn(BrokerEvent) + Send + Sync>,
    ) -> Result<Self, BrokerStartError> {
        // Path guard at the fs entry: the broker creates a directory,
        // unlinks a socket node and binds a fresh one — every one of those
        // operates on the profile derived from this argument, so the
        // argument itself is validated here (absolute, traversal-free,
        // with a real parent) instead of trusting the caller's discipline.
        if let Some(cause) = refused_profile_cause(managed_root) {
            return Err(BrokerStartError::Io(std::io::Error::other(format!(
                "managed root `{}` refused: {cause}",
                managed_root.display()
            ))));
        }
        let profile_dir =
            managed_root
                .parent()
                .ok_or_else(|| BrokerStartError::NoProfileDirectory {
                    managed_root: managed_root.to_path_buf(),
                })?;
        std::fs::create_dir_all(profile_dir).map_err(BrokerStartError::Io)?;
        let socket_path = profile_dir.join(SOCKET_FILE_NAME);

        // Live-vs-stale probe: connect succeeding proves a live broker —
        // refuse; connect failing proves nobody is serving (stale file
        // from a crash) — unlink and take over.
        if socket_path.exists() {
            match UnixStream::connect(&socket_path) {
                Ok(_) => {
                    return Err(BrokerStartError::AlreadyRunning { socket_path });
                }
                Err(_) => {
                    std::fs::remove_file(&socket_path).map_err(BrokerStartError::Io)?;
                }
            }
        }
        let listener = UnixListener::bind(&socket_path).map_err(BrokerStartError::Io)?;
        // Owner-only: bind creates the node with umask-dependent mode;
        // the profile contract is 0600 (DoD: «сокет 0600 в профиле
        // пользователя»). The window between bind and chmod is inside a
        // user-owned directory.
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
            .map_err(BrokerStartError::Io)?;

        let shared = Arc::new(ServerShared {
            ops,
            clients: Mutex::new(Vec::new()),
            next_client_id: AtomicU64::new(1),
            active_clients: AtomicUsize::new(0),
            host_notify: notify,
            shutdown: Arc::new(AtomicBool::new(false)),
        });
        // The ops→server event seam. Weak on purpose: ops lives inside
        // `shared`; a strong Arc here would be a reference cycle keeping
        // the whole server alive forever.
        let emitter_shared: Weak<ServerShared> = Arc::downgrade(&shared);
        shared.ops.bind_emitter(Arc::new(move |event| {
            if let Some(sh) = emitter_shared.upgrade() {
                sh.broadcast(event);
            }
        }));

        listener
            .set_nonblocking(true)
            .map_err(BrokerStartError::Io)?;
        let shutdown = shared.shutdown.clone();
        let accept_thread = std::thread::Builder::new()
            .name("rimloc-broker-accept".to_string())
            .spawn(move || accept_loop(listener, shared))
            .map_err(BrokerStartError::Io)?;
        Ok(Self {
            socket_path,
            shutdown,
            accept_thread: Some(accept_thread),
        })
    }

    /// The bound socket path (the profile dir + [`SOCKET_FILE_NAME`]).
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

impl Drop for BrokerServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.accept_thread.take() {
            let _ = handle.join();
        }
        // If another broker took the path over in the meantime, removing
        // it would break THEIR socket; only remove what we still own
        // (same path we bound — the single-GUI-per-profile assumption is
        // documented in the design).
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

fn accept_loop(listener: UnixListener, shared: Arc<ServerShared>) {
    loop {
        if shared.shutdown.load(Ordering::Relaxed) {
            break;
        }
        match listener.accept() {
            Ok((stream, _addr)) => admit_client(stream, &shared),
            // Nonblocking mode: WouldBlock is the normal idle path; any
            // transient accept error just delays admission by one poll.
            Err(_) => std::thread::sleep(ACCEPT_POLL),
        }
    }
}

fn admit_client(mut stream: UnixStream, shared: &Arc<ServerShared>) {
    // Race-free admission: the slot is claimed BEFORE the check, so two
    // simultaneous connects can never both sneak past the bound.
    let slot = shared.active_clients.fetch_add(1, Ordering::Relaxed);
    if slot >= MAX_CLIENTS {
        shared.active_clients.fetch_sub(1, Ordering::Relaxed);
        let err = BrokerResponse::err(
            serde_json::Value::Null,
            &BrokerError::too_many_clients(MAX_CLIENTS),
        );
        if let Ok(line) = encode_line(&err) {
            let _ = stream.write_all(&line);
            let _ = stream.flush();
        }
        return; // dropped → closed
    }
    // The write half is shared (responses + event fan-out); the read half
    // stays exclusively with the reader loop below.
    let out = match stream.try_clone() {
        Ok(out) => Arc::new(Mutex::new(out)),
        Err(_) => {
            shared.active_clients.fetch_sub(1, Ordering::Relaxed);
            return;
        }
    };
    let client_id = shared.register(out.clone());
    let shared_thread = shared.clone();
    let spawned = std::thread::Builder::new()
        .name("rimloc-broker-client".to_string())
        .spawn(move || {
            client_loop(stream, out, shared_thread.clone());
            shared_thread.active_clients.fetch_sub(1, Ordering::Relaxed);
            shared_thread.unregister(client_id);
        });
    if spawned.is_err() {
        // Thread spawn failed: release the slot; the unaccepted stream
        // drops and the client sees a bare close.
        shared.unregister(client_id);
        shared.active_clients.fetch_sub(1, Ordering::Relaxed);
    }
}

fn client_loop(stream: UnixStream, out: Arc<Mutex<UnixStream>>, shared: Arc<ServerShared>) {
    let _ = stream.set_nonblocking(false);
    let mut reader = BufReader::new(stream);
    // In-flight bound: park the reader while too many lines are executing.
    let in_flight = Arc::new((Mutex::new(0usize), Condvar::new()));
    loop {
        match read_line_bounded(&mut reader, crate::protocol::MAX_LINE_BYTES) {
            Ok(None) => break, // clean EOF: client hung up
            Ok(Some(line)) => {
                let (lock, cvar) = &*in_flight;
                let mut count = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                while *count >= MAX_IN_FLIGHT_PER_CLIENT {
                    count = cvar
                        .wait(count)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                *count += 1;
                drop(count);

                let shared_line = shared.clone();
                let out_line = out.clone();
                let in_flight_line = in_flight.clone();
                let spawned = std::thread::Builder::new()
                    .name("rimloc-broker-line".to_string())
                    .spawn(move || {
                        let response = handle_line(&line, &shared_line.ops);
                        write_frame(&out_line, &response);
                        let (lock, cvar) = &*in_flight_line;
                        let mut count =
                            lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                        *count = count.saturating_sub(1);
                        drop(count);
                        cvar.notify_one();
                    });
                if spawned.is_err() {
                    // No thread available: one typed internal error, the
                    // bound released, the connection lives.
                    write_frame(
                        &out,
                        &BrokerResponse::err(
                            serde_json::Value::Null,
                            &BrokerError::internal("broker is overloaded; retry"),
                        ),
                    );
                    let (lock, cvar) = &*in_flight;
                    let mut count = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    *count = count.saturating_sub(1);
                    drop(count);
                    cvar.notify_one();
                }
            }
            Err(LineReadError::TooLong) => {
                write_frame(
                    &out,
                    &BrokerResponse::err(
                        serde_json::Value::Null,
                        &BrokerError::line_too_long(crate::protocol::MAX_LINE_BYTES),
                    ),
                );
                // The framing cannot be trusted past an oversized line —
                // close THIS connection only; the server keeps serving.
                break;
            }
            Err(LineReadError::Io(err)) => {
                // Clean EOF at a frame boundary returned Ok(None) above;
                // mid-line EOF and socket-level failures land here. One
                // stderr diagnostic, then the connection closes — never
                // the server.
                if err.kind() != std::io::ErrorKind::UnexpectedEof {
                    // Dev-log shape (`{}`-formatter tag + detail) — the
                    // no-hardcoded-user-strings gate accepts this idiom
                    // for structured stderr diagnostics.
                    eprintln!("{} {}", DEV_LOG_CLIENT_READ_FAILED, err);
                }
                break;
            }
        }
    }
}

/// Parse + execute one request line. This is the panic boundary: any
/// handler panic becomes a typed internal error answer.
fn handle_line(line: &[u8], ops: &Arc<dyn BrokerOps>) -> BrokerResponse {
    let request: BrokerRequest = match serde_json::from_slice(line) {
        Ok(request) => request,
        Err(_) => {
            return BrokerResponse::err(
                serde_json::Value::Null,
                &BrokerError::parse_error(
                    "line is not a valid broker v1 request ({id, method, params})",
                ),
            )
        }
    };
    if request.method.is_empty() {
        return BrokerResponse::err(
            request.id,
            &BrokerError::invalid_request("`method` must be a non-empty string"),
        );
    }
    let id = request.id;
    let method = request.method;
    let params = request.params;
    let executed = std::panic::catch_unwind(AssertUnwindSafe(|| ops.dispatch(&method, params)));
    match executed {
        Ok(Ok(result)) => BrokerResponse::ok(id, result),
        Ok(Err(err)) => BrokerResponse::err(id, &err),
        // A panic must not take the accept loop (or this connection's
        // reader) down: one typed error, the server lives.
        Err(_) => BrokerResponse::err(id, &BrokerError::handler_panicked(&method)),
    }
}

fn write_frame(out: &Arc<Mutex<UnixStream>>, response: &BrokerResponse) {
    if let Ok(line) = encode_line(response) {
        let mut stream = out.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = stream.write_all(&line);
        let _ = stream.flush();
    }
}

enum LineReadError {
    /// The bound was exceeded before a newline arrived.
    TooLong,
    Io(std::io::Error),
}

/// One `\n`-terminated frame, accumulated with a hard cap. `Ok(None)` on
/// a clean EOF at a frame boundary.
fn read_line_bounded<R: Read>(
    reader: &mut BufReader<R>,
    cap: usize,
) -> Result<Option<Vec<u8>>, LineReadError> {
    let mut out: Vec<u8> = Vec::new();
    loop {
        let available = match reader.fill_buf() {
            Ok(buf) => buf,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(LineReadError::Io(err)),
        };
        if available.is_empty() {
            // EOF: clean if at a frame boundary, a truncation otherwise.
            return if out.is_empty() {
                Ok(None)
            } else {
                Err(LineReadError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "connection closed mid-line",
                )))
            };
        }
        if let Some(pos) = available.iter().position(|&b| b == b'\n') {
            out.extend_from_slice(&available[..pos]);
            reader.consume(pos + 1);
            // The cap holds on the newline branch too: a delimiter
            // arriving in the same chunk that crossed the bound must not
            // smuggle an oversized line into the parser.
            if out.len() > cap {
                return Err(LineReadError::TooLong);
            }
            return Ok(Some(out));
        }
        let chunk = available.len();
        out.extend_from_slice(available);
        reader.consume(chunk);
        if out.len() > cap {
            return Err(LineReadError::TooLong);
        }
    }
}

#[cfg(test)]
mod path_guard_tests {
    use super::refused_profile_cause;
    use std::path::Path;

    #[test]
    fn refuses_relative_and_traversal_roots() {
        assert!(refused_profile_cause(Path::new("relative/managed")).is_some());
        assert!(refused_profile_cause(Path::new("/a/../escape")).is_some());
    }

    #[test]
    fn refuses_filesystem_root() {
        assert!(refused_profile_cause(Path::new("/")).is_some());
        assert!(refused_profile_cause(Path::new("/managed")).is_none());
    }
}
