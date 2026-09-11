use super::*;

fn counts(active: u64, waiting: u64, stopped: u64, speed: u64) -> QueueCounts {
    serde_json::from_value(serde_json::json!({
        "numActive": active.to_string(), "numWaiting": waiting.to_string(),
        "numStoppedTotal": stopped.to_string(), "numStopped": stopped.to_string(), "downloadSpeed": speed.to_string()
    })).unwrap()
}

fn transfer(gid: &str, status: &str, done: u64, total: u64) -> Transfer {
    serde_json::from_value(serde_json::json!({
        "gid": gid, "status": status, "totalLength": total.to_string(),
        "completedLength": done.to_string()
    }))
    .unwrap()
}

#[test]
fn shutdown_requires_terminal_work_and_empty_active_and_waiting_queues() {
    assert!(!counts(0, 0, 0, 0).ready_to_shutdown().unwrap());
    assert!(!counts(1, 0, 1, 0).ready_to_shutdown().unwrap());
    assert!(!counts(0, 1, 1, 0).ready_to_shutdown().unwrap());
    assert!(counts(0, 0, 1, 0).ready_to_shutdown().unwrap());
}

#[test]
fn progress_reports_bytes_speed_eta_without_raw_data() {
    let p = SessionTracker::default()
        .progress(&counts(1, 0, 0, 25), vec![transfer("a", "active", 25, 100)])
        .unwrap();
    assert_eq!(p.percent, Some(25.0));
    assert_eq!(p.speed.as_deref(), Some("25 B/s"));
    assert_eq!(p.eta.as_deref(), Some("00:03"));
    assert!(p.raw.is_none());
    assert_eq!(p.status, "Downloading");
}

#[test]
fn batch_includes_waiting_and_completed_records_and_deduplicates_gids() {
    let mut tracker = SessionTracker::default();
    let records = vec![
        transfer("a", "complete", 100, 100),
        transfer("b", "active", 50, 100),
        transfer("c", "waiting", 0, 100),
    ];
    assert_eq!(
        tracker
            .progress(&counts(1, 1, 1, 25), records.clone())
            .unwrap()
            .percent,
        Some(50.0)
    );
    assert_eq!(
        tracker
            .progress(&counts(1, 1, 1, 25), records)
            .unwrap()
            .percent,
        Some(50.0)
    );
    // Completed a falls out of aria2's retained results but remains in our ledger.
    assert_eq!(
        tracker
            .progress(
                &counts(1, 0, 2, 25),
                vec![
                    transfer("b", "complete", 100, 100),
                    transfer("c", "active", 25, 100)
                ]
            )
            .unwrap()
            .percent,
        Some(75.0)
    );
}

#[test]
fn unknown_totals_missing_history_and_failed_items_do_not_fabricate_percent() {
    for (q, records) in [
        (counts(1, 0, 0, 0), vec![transfer("a", "active", 25, 0)]),
        (counts(1, 1, 0, 20), vec![transfer("a", "active", 25, 100)]),
        (
            counts(1, 0, 2, 20),
            vec![
                transfer("a", "active", 25, 100),
                transfer("b", "complete", 100, 100),
            ],
        ),
        (
            counts(1, 0, 1, 20),
            vec![
                transfer("a", "active", 25, 100),
                transfer("b", "error", 0, 100),
            ],
        ),
    ] {
        let p = SessionTracker::default().progress(&q, records).unwrap();
        assert!(p.percent.is_none());
        assert!(p.eta.is_none());
    }
}

#[test]
fn invalid_numbers_and_overflow_are_rejected() {
    let mut q = counts(1, 0, 0, 25);
    q.num_active = "-1".into();
    assert!(q.ready_to_shutdown().is_err());
    assert!(SessionTracker::default()
        .progress(
            &counts(2, 0, 0, 25),
            vec![
                transfer("a", "active", 0, u64::MAX),
                transfer("b", "active", 0, 100)
            ]
        )
        .is_err());
}

#[test]
fn zero_speed_omits_eta_and_finished_transfer_is_not_whole_job_completion() {
    let p = SessionTracker::default()
        .progress(
            &counts(0, 0, 1, 0),
            vec![transfer("a", "complete", 100, 100)],
        )
        .unwrap();
    assert_eq!(p.percent, Some(100.0));
    assert!(p.eta.is_none());
    assert_ne!(p.status, "Completed");
}

use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

struct Server {
    config: Option<RpcConfig>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(mut reply: impl FnMut(Value) -> Value + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let request: Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(request["params"][0], "token:test-secret");
                let body = serde_json::to_vec(&reply(request)).unwrap();
                // Timeout tests deliberately close the client before this reply.
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(&body);
            }
        });
        Self {
            config: Some(RpcConfig {
                port,
                secret: "test-secret".into(),
            }),
            stop,
            worker: Some(worker),
        }
    }
    fn monitor(&mut self) -> RpcMonitor {
        RpcMonitor::new(self.config.take().unwrap()).unwrap()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}
fn reply(request: &Value, result: Value) -> Value {
    json!({"jsonrpc":"2.0", "id":request["id"], "result":result})
}
fn wire_counts(active: u64, waiting: u64, stopped: u64) -> Value {
    json!({"numActive":active.to_string(),"numWaiting":waiting.to_string(),"numStopped":stopped.to_string(),"numStoppedTotal":stopped.to_string(),"downloadSpeed":"25"})
}
fn wire_transfer(gid: &str, status: &str, done: u64) -> Value {
    json!({"gid":gid,"status":status,"completedLength":done.to_string(),"totalLength":"100"})
}

#[test]
fn runtime_credentials_are_fresh_and_redacted() {
    let a = RpcConfig::new().unwrap();
    let b = RpcConfig::new().unwrap();
    assert_ne!(a.secret, b.secret);
    assert!(a.secret.len() >= 32);
    assert!(!a.redact(&a.arguments()).contains(&a.secret));
    assert!(a.arguments().contains("--rpc-listen-all=false"));
    assert!(a.arguments().contains("--enable-rpc=true"));
}

#[test]
fn authenticated_monitor_reports_progress_then_shuts_down_and_accepts_next_session() {
    let phase = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let server_phase = phase.clone();
    let shutdowns = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let server_shutdowns = shutdowns.clone();
    let mut server = Server::new(move |req| {
        let phase = server_phase.load(Ordering::SeqCst);
        let result = match req["method"].as_str().unwrap() {
            "aria2.getSessionInfo" => json!({"sessionId":if phase < 2 {"first"} else {"second"}}),
            "aria2.getGlobalStat" => wire_counts(u64::from(phase != 1), 0, u64::from(phase == 1)),
            "aria2.tellActive" => {
                if phase == 1 {
                    json!([])
                } else {
                    json!([wire_transfer(
                        if phase == 2 { "b" } else { "a" },
                        "active",
                        if phase == 2 { 10 } else { 25 }
                    )])
                }
            }
            "aria2.tellWaiting" => json!([]),
            "aria2.tellStopped" => {
                if phase == 1 {
                    json!([wire_transfer("a", "complete", 100)])
                } else {
                    json!([])
                }
            }
            "aria2.shutdown" => {
                server_shutdowns.fetch_add(1, Ordering::SeqCst);
                json!("OK")
            }
            other => panic!("Unexpected RPC {other}"),
        };
        reply(&req, result)
    });
    let mut monitor = server.monitor();
    assert_eq!(monitor.poll().unwrap().unwrap().percent, Some(25.0));
    assert!(monitor.active());
    phase.store(1, Ordering::SeqCst);
    monitor.poll().unwrap();
    assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
    assert!(!monitor.active());
    phase.store(2, Ordering::SeqCst);
    assert_eq!(monitor.poll().unwrap().unwrap().percent, Some(10.0));
    assert!(monitor.active());
}

#[test]
fn fast_failure_before_first_poll_still_shuts_down_without_publishing_success() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let flag = shutdown.clone();
    let mut server = Server::new(move |req| {
        let value = match req["method"].as_str().unwrap() {
            "aria2.getSessionInfo" => json!({"sessionId":"failed"}),
            "aria2.getGlobalStat" => wire_counts(0, 0, 1),
            "aria2.tellActive" | "aria2.tellWaiting" => json!([]),
            "aria2.tellStopped" => json!([wire_transfer("a", "error", 0)]),
            "aria2.shutdown" => {
                flag.store(true, Ordering::SeqCst);
                json!("OK")
            }
            other => panic!("Unexpected RPC {other}"),
        };
        reply(&req, value)
    });
    let mut monitor = server.monitor();
    let p = monitor.poll().unwrap();
    assert!(shutdown.load(Ordering::SeqCst));
    assert!(p.is_none_or(|p| p.status != "Completed" && p.percent != Some(100.0)));
}

#[test]
fn invalid_or_unauthorized_server_fails_without_leaking_its_reply() {
    for result in [
        json!({"jsonrpc":"2.0","id":1,"error":{"code":1,"message":"test-secret"}}),
        json!({"jsonrpc":"2.0","id":1,"result":{"notSessionId":"test-secret"}}),
    ] {
        let mut server = Server::new(move |_| result.clone());
        let err = server
            .monitor()
            .poll()
            .expect_err("must reject invalid reply");
        assert!(!err.contains("test-secret"));
    }
}

#[test]
fn absent_rpc_is_normal_before_connection_but_not_after_an_active_session() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut monitor = RpcMonitor::new(RpcConfig {
        port,
        secret: "test-secret".into(),
    })
    .unwrap();
    // Windows can take over one second to report a refused loopback connection.
    monitor.request_timeout = Duration::from_secs(2);
    assert!(monitor.poll().unwrap().is_none());
    monitor.session = Some("connected".into());
    monitor.last_success = Instant::now() - Duration::from_secs(20);
    assert!(monitor.poll().is_err());
    monitor.shutdown_since = Some(Instant::now());
    assert!(monitor.poll().unwrap().is_none());
}

#[test]
fn enabled_arguments_preserve_parallelism_and_disabled_arguments_ignore_rpc() {
    use crate::aria2c::{aria2c_downloader_args, Aria2cConfig, Aria2cStatus};
    let rpc = RpcConfig::new().unwrap();
    let mut config = Aria2cConfig {
        enabled: true,
        parallel_connections: 4,
        ..Default::default()
    };
    let path = std::env::temp_dir().join("tools with spaces/aria2c");
    let status = Aria2cStatus {
        available: true,
        executable_path: Some(path.clone()),
        ..Default::default()
    };
    let args = aria2c_downloader_args(&config, &status, Some(&rpc)).unwrap();
    assert_eq!(args[1], path.as_os_str());
    let options = args[3].to_str().unwrap();
    assert!(options.starts_with("aria2c:-j 4 -x 4 -s 4 "));
    assert!(options.contains(&format!("--rpc-listen-port={}", rpc.port)));
    assert!(options.contains(&format!("--rpc-secret={}", rpc.secret)));
    config.enabled = false;
    assert!(aria2c_downloader_args(&config, &status, Some(&rpc))
        .unwrap()
        .is_empty());
}

#[test]
fn progress_sources_switch_per_session_and_stop_blocks_late_updates() {
    let router = ProgressRouter::default();
    let mut events = Vec::new();
    let p = || DownloadProgress {
        percent: Some(25.0),
        status: "Downloading".into(),
        speed: None,
        eta: None,
        raw: None,
    };
    router.native(p(), |p| events.push(p));
    router.rpc(true, Some(p()), |p| events.push(p));
    router.native(p(), |p| events.push(p));
    assert_eq!(events.len(), 2, "RPC owns progress only while active");
    router.rpc(false, None, |p| events.push(p));
    router.native(p(), |p| events.push(p));
    assert_eq!(events.len(), 3, "native fallback resumes");
    router.stop();
    router.native(p(), |p| events.push(p));
    router.rpc(true, Some(p()), |p| events.push(p));
    assert_eq!(events.len(), 3, "finished/cancelled jobs must not publish");
}

#[test]
fn shutdown_timeout_does_not_disable_failure_deadline() {
    let mut server = Server::new(|req| {
        thread::sleep(Duration::from_millis(60));
        reply(&req, json!({"sessionId":"stuck"}))
    });
    let mut monitor = server.monitor();
    monitor.request_timeout = Duration::from_millis(10);
    monitor.session = Some("stuck".into());
    monitor.shutdown_since = Some(Instant::now() - Duration::from_secs(20));
    monitor.last_success = Instant::now() - Duration::from_secs(20);
    assert!(
        monitor.poll().is_err(),
        "an unresponsive RPC server must not bypass shutdown deadline"
    );
}

#[test]
fn large_queues_do_not_require_hundreds_of_requests_before_reporting() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = calls.clone();
    let mut server = Server::new(move |req| {
        counter.fetch_add(1, Ordering::SeqCst);
        let value = match req["method"].as_str().unwrap() {
            "aria2.getSessionInfo" => json!({"sessionId":"large"}),
            "aria2.getGlobalStat" => wire_counts(1, 10_000, 0),
            "aria2.tellActive" => json!([wire_transfer("active", "active", 25)]),
            "aria2.tellWaiting" => json!([]),
            other => panic!("Unexpected RPC {other}"),
        };
        reply(&req, value)
    });
    let mut monitor = server.monitor();
    let progress = monitor.poll().unwrap().unwrap();
    assert!(progress.percent.is_none());
    assert!(progress.speed.is_some());
    assert!(
        calls.load(Ordering::SeqCst) <= 8,
        "pagination must continue across polling cycles"
    );
}

#[test]
fn new_session_gets_a_fresh_retry_window_after_slow_extraction() {
    let mut server = Server::new(|req| {
        if req["method"] == "aria2.getGlobalStat" {
            thread::sleep(Duration::from_millis(60));
        }
        reply(
            &req,
            if req["method"] == "aria2.getSessionInfo" {
                json!({"sessionId":"new"})
            } else {
                wire_counts(1, 0, 0)
            },
        )
    });
    let mut monitor = server.monitor();
    monitor.request_timeout = Duration::from_millis(10);
    monitor.last_success = Instant::now() - Duration::from_secs(20);
    assert!(monitor.poll().unwrap().is_none());
}

#[test]
#[cfg(unix)]
fn failed_parent_exit_cleans_pipe_inheriting_descendant_before_returning() {
    use crate::{
        download_process_command, kill_process_tree, process_group_exists, wait_for_download,
    };
    let root = crate::test_support::TestDirectory::new();
    let exe = root.fixture("orphan");
    let mut child = download_process_command(exe)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert!(line.starts_with("child-ready:"));
    let result = wait_for_download(&mut child, None, &ProgressRouter::default(), |_| {});
    let leaked = process_group_exists(pid);
    let _ = kill_process_tree(pid);
    assert!(!result.unwrap().success());
    assert!(
        !leaked,
        "reader joins would hang on orphan's inherited pipes"
    );
}

#[test]
fn oversized_and_mismatched_rpc_replies_are_rejected() {
    for response in [
        json!({"jsonrpc":"2.0","id":999,"result":{"sessionId":"wrong-request"}}),
        json!({"jsonrpc":"2.0","id":1,"result":{"sessionId":"x".repeat(1024*1024+1)}}),
    ] {
        let mut server = Server::new(move |_| response.clone());
        let error = match server.monitor().poll() {
            Err(error) => error,
            Ok(_) => panic!("invalid reply accepted"),
        };
        assert!(
            error == "Invalid aria2c RPC response identity."
                || error == "aria2c RPC response too large."
        );
    }
}
