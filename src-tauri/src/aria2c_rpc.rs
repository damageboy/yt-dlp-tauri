use crate::DownloadProgress;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use std::{io::Read, net::TcpListener};

#[derive(Clone)]
pub(crate) struct RpcConfig {
    port: u16,
    secret: String,
}

impl RpcConfig {
    pub(crate) fn new() -> Result<Self, String> {
        let listener =
            TcpListener::bind("127.0.0.1:0").map_err(|_| "Cannot allocate aria2c RPC port.")?;
        let port = listener
            .local_addr()
            .map_err(|_| "Cannot read aria2c RPC port.")?
            .port();
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| "Cannot generate aria2c RPC token.")?;
        let secret = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Self { port, secret })
    }
    pub(crate) fn arguments(&self) -> String {
        format!("--enable-rpc=true --rpc-listen-all=false --rpc-listen-port={} --rpc-secret={} --stop-with-process={}", self.port, self.secret, std::process::id())
    }
    pub(crate) fn redact(&self, line: &str) -> String {
        line.replace(&self.secret, "[redacted]")
    }
}

pub(crate) struct RpcMonitor {
    config: RpcConfig,
    client: reqwest::blocking::Client,
    session: Option<String>,
    tracker: SessionTracker,
    last_success: Instant,
    shutdown_since: Option<Instant>,
    active: bool,
    failure_timeout: Duration,
    request_timeout: Duration,
    poll_deadline: Instant,
    page_offsets: [u64; 2],
}

impl RpcMonitor {
    pub(crate) fn new(config: RpcConfig) -> Result<Self, String> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(1))
            .build()
            .map_err(|_| "Cannot create aria2c RPC client.")?;
        Ok(Self {
            config,
            client,
            session: None,
            tracker: SessionTracker::default(),
            last_success: Instant::now(),
            shutdown_since: None,
            active: false,
            failure_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(1),
            poll_deadline: Instant::now(),
            page_offsets: [0, 0],
        })
    }
    pub(crate) fn poll(&mut self) -> Result<Option<DownloadProgress>, String> {
        self.poll_deadline = Instant::now() + Duration::from_secs(2);
        let result = self.poll_session();
        self.handle_poll_result(result)
    }
    fn handle_poll_result(
        &mut self,
        result: Result<Option<DownloadProgress>, RpcError>,
    ) -> Result<Option<DownloadProgress>, String> {
        match result {
            Ok(progress) => {
                self.last_success = Instant::now();
                Ok(progress)
            }
            // A closed listener is expected between aria2c processes. A timeout is not.
            Err(RpcError::Disconnected)
                if self.shutdown_since.is_some() || self.session.is_none() =>
            {
                Ok(None)
            }
            Err(RpcError::Unavailable) if self.session.is_none() => Ok(None),
            Err(RpcError::Unavailable | RpcError::Disconnected)
                if self.last_success.elapsed() < self.failure_timeout =>
            {
                Ok(None)
            }
            Err(RpcError::Unavailable | RpcError::Disconnected) => {
                Err("Lost aria2c progress connection. Download stopped.".into())
            }
            Err(RpcError::Invalid(message)) => Err(message),
        }
    }
    pub(crate) fn active(&self) -> bool {
        self.active
    }

    fn call<T: DeserializeOwned>(&self, method: &str, args: Vec<Value>) -> Result<T, RpcError> {
        let remaining = self.poll_deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(RpcError::Unavailable);
        }
        let mut params = vec![json!(format!("token:{}", self.config.secret))];
        params.extend(args);
        let body =
            serde_json::to_vec(&json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params}))
                .map_err(|_| invalid("Cannot encode aria2c RPC request."))?;
        let response = self
            .client
            .post(format!("http://127.0.0.1:{}/jsonrpc", self.config.port))
            .timeout(remaining.min(self.request_timeout))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .map_err(|error| {
                if error.is_connect() && !error.is_timeout() {
                    RpcError::Disconnected
                } else {
                    RpcError::Unavailable
                }
            })?;
        if !response.status().is_success() {
            return Err(invalid("aria2c RPC HTTP request rejected."));
        }
        const MAX_RESPONSE: u64 = 1024 * 1024;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE)
        {
            return Err(invalid("aria2c RPC response too large."));
        }
        let mut data = Vec::new();
        response
            .take(MAX_RESPONSE + 1)
            .read_to_end(&mut data)
            .map_err(|_| RpcError::Unavailable)?;
        if data.len() as u64 > MAX_RESPONSE {
            return Err(invalid("aria2c RPC response too large."));
        }
        let response: Value =
            serde_json::from_slice(&data).map_err(|_| invalid("Invalid aria2c RPC response."))?;
        if response.get("error").is_some() {
            return Err(invalid(
                "aria2c RPC request rejected; check RPC support or port conflict.",
            ));
        }
        if response["jsonrpc"] != "2.0" || response["id"] != 1 {
            return Err(invalid("Invalid aria2c RPC response identity."));
        }
        serde_json::from_value(response["result"].clone())
            .map_err(|_| invalid("Invalid aria2c RPC result."))
    }

    fn session_id(&self) -> Result<String, RpcError> {
        #[derive(Deserialize)]
        struct Session {
            #[serde(rename = "sessionId")]
            id: String,
        }
        let session: Session = self.call("aria2.getSessionInfo", vec![])?;
        if session.id.is_empty() {
            return Err(invalid("Missing aria2c RPC session ID."));
        }
        Ok(session.id)
    }

    fn poll_session(&mut self) -> Result<Option<DownloadProgress>, RpcError> {
        let session = self.session_id()?;
        if self.session.as_ref() != Some(&session) {
            self.last_success = Instant::now();
            self.session = Some(session.clone());
            self.tracker = SessionTracker::default();
            self.shutdown_since = None;
            self.active = false;
            self.page_offsets = [0, 0];
        }
        if let Some(since) = self.shutdown_since {
            if since.elapsed() >= self.failure_timeout {
                return Err(invalid("aria2c did not shut down after finishing."));
            }
            // Retry an uncertain shutdown response without extending its deadline.
            let _: String = self.call("aria2.shutdown", vec![])?;
            return Ok(None);
        }
        let counts: QueueCounts = self.call("aria2.getGlobalStat", vec![])?;
        let keys = json!(["gid", "status", "totalLength", "completedLength"]);
        let mut transfers: Vec<Transfer> = self.call("aria2.tellActive", vec![keys.clone()])?;
        for (index, (method, count)) in [
            ("aria2.tellWaiting", &counts.num_waiting),
            ("aria2.tellStopped", &counts.num_stopped),
        ]
        .into_iter()
        .enumerate()
        {
            let count = number(count).map_err(RpcError::Invalid)?;
            if count > 100_000 {
                return Err(invalid("aria2c RPC queue too large."));
            }
            // One page per queue per cycle bounds work; the GID ledger spans cycles.
            if count > 0 {
                let offset = self.page_offsets[index] % count;
                let page: Vec<Transfer> =
                    self.call(method, vec![json!(offset), json!(128), keys.clone()])?;
                transfers.extend(page);
                self.page_offsets[index] = if offset + 128 >= count {
                    0
                } else {
                    offset + 128
                };
            }
        }
        let progress = self
            .tracker
            .progress(&counts, transfers)
            .map_err(RpcError::Invalid)?;
        // Check generation again: the process may have exited/restarted during requests.
        if self.session_id()? != session {
            self.active = false;
            return Ok(None);
        }
        self.active = number(&counts.num_active).map_err(RpcError::Invalid)? > 0
            || number(&counts.num_waiting).map_err(RpcError::Invalid)? > 0;
        if counts.ready_to_shutdown().map_err(RpcError::Invalid)? {
            let confirm: QueueCounts = self.call("aria2.getGlobalStat", vec![])?;
            if confirm.ready_to_shutdown().map_err(RpcError::Invalid)? {
                self.shutdown_since = Some(Instant::now());
                self.active = false;
                let _: String = self.call("aria2.shutdown", vec![])?;
            }
        }
        Ok((self.active || self.shutdown_since.is_some()).then_some(progress))
    }
}

enum RpcError {
    Disconnected,
    Unavailable,
    Invalid(String),
}
fn invalid(message: &str) -> RpcError {
    RpcError::Invalid(message.into())
}

#[derive(Default)]
pub(crate) struct ProgressRouter {
    state: std::sync::Mutex<(bool, bool)>, // (RPC owns transfer progress, operation stopped)
}
impl ProgressRouter {
    pub(crate) fn native(&self, progress: DownloadProgress, emit: impl FnOnce(DownloadProgress)) {
        if let Ok(state) = self.state.lock() {
            if !state.0 && !state.1 {
                emit(progress);
            }
        }
    }
    pub(crate) fn rpc(
        &self,
        active: bool,
        progress: Option<DownloadProgress>,
        emit: impl FnOnce(DownloadProgress),
    ) {
        if let Ok(mut state) = self.state.lock() {
            state.0 = active;
            if !state.1 {
                if let Some(progress) = progress {
                    emit(progress);
                }
            }
        }
    }
    pub(crate) fn stop(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.1 = true;
        }
    }
}

fn number(value: &str) -> Result<u64, String> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Invalid aria2c RPC number.".into());
    }
    value
        .parse()
        .map_err(|_| "Invalid aria2c RPC number.".into())
}

fn add(a: u64, b: u64) -> Result<u64, String> {
    a.checked_add(b)
        .ok_or_else(|| "aria2c RPC count overflow.".into())
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueueCounts {
    num_active: String,
    num_waiting: String,
    num_stopped_total: String,
    num_stopped: String,
    download_speed: String,
}

impl QueueCounts {
    fn ready_to_shutdown(&self) -> Result<bool, String> {
        let active = number(&self.num_active)?;
        let waiting = number(&self.num_waiting)?;
        let stopped = number(&self.num_stopped_total)?;
        Ok(active == 0 && waiting == 0 && stopped > 0)
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transfer {
    gid: String,
    status: String,
    total_length: String,
    completed_length: String,
}

#[derive(Default)]
struct SessionTracker {
    transfers: BTreeMap<String, Transfer>,
}

impl SessionTracker {
    fn progress(
        &mut self,
        counts: &QueueCounts,
        transfers: Vec<Transfer>,
    ) -> Result<DownloadProgress, String> {
        let expected = add(
            add(number(&counts.num_active)?, number(&counts.num_waiting)?)?,
            number(&counts.num_stopped_total)?,
        )?;
        let speed = number(&counts.download_speed)?;
        for transfer in transfers {
            self.transfers.insert(transfer.gid.clone(), transfer);
        }
        let mut known = self.transfers.len() as u64 == expected;
        let (mut done, mut total) = (0, 0);
        for transfer in self.transfers.values() {
            let size = number(&transfer.total_length)?;
            let completed = number(&transfer.completed_length)?;
            known &= size > 0
                && matches!(
                    transfer.status.as_str(),
                    "active" | "waiting" | "paused" | "complete"
                );
            total = add(total, size)?;
            done = add(done, completed.min(size))?;
        }
        let percent =
            (known && total > 0).then(|| (done as f64 / total as f64 * 100.0).clamp(0.0, 100.0));
        let eta = if percent.is_some() && speed > 0 {
            let seconds = (total - done) / speed;
            Some(if seconds >= 3600 {
                format!(
                    "{:02}:{:02}:{:02}",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60
                )
            } else {
                format!("{:02}:{:02}", seconds / 60, seconds % 60)
            })
        } else {
            None
        };
        let speed = Some(if speed < 1024 {
            format!("{speed} B/s")
        } else if speed < 1024 * 1024 {
            format!("{:.2} KiB/s", speed as f64 / 1024.0)
        } else {
            format!("{:.2} MiB/s", speed as f64 / (1024.0 * 1024.0))
        });
        Ok(DownloadProgress {
            percent,
            status: "Downloading".into(),
            speed,
            eta,
            raw: None,
        })
    }
}

#[cfg(test)]
#[path = "aria2c_rpc_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "aria2c_rpc_smoke_tests.rs"]
mod smoke_tests;
