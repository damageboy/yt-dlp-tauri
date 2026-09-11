use super::*;
use crate::{
    aria2c::{Aria2cConfig, Aria2cStatus},
    test_support::TestDirectory,
    *,
};
use std::{
    io::Read,
    process::{Child, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "requires installed yt-dlp, aria2c, ffmpeg, ffprobe and python3"]
fn real_rpc_download_lifecycle() {
    let root = TestDirectory::new();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rpc_media_server.py");
    let mut server = Process(
        background_command("python3")
            .arg(fixture)
            .arg(root.0.join("media"))
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut line = String::new();
    BufReader::new(server.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let server_info: Value = serde_json::from_str(&line).unwrap();
    let base = server_info["url"].as_str().unwrap();
    let ffmpeg = background_command("python3")
        .args(["-c", "import shutil; print(shutil.which('ffmpeg'))"])
        .output()
        .unwrap();
    let ffmpeg = PathBuf::from(String::from_utf8(ffmpeg.stdout).unwrap().trim());
    let tools = ToolPaths {
        root: root.0.clone(),
        yt_dlp: "yt-dlp".into(),
        ffmpeg: ffmpeg.clone(),
        ffmpeg_dir: ffmpeg.parent().unwrap().into(),
        ffprobe: "ffprobe".into(),
        deno: "deno".into(),
    };
    for case in [
        "native",
        "http",
        "separate",
        "hls",
        "native-fallback",
        "unknown",
        "failure",
        "broken-hls",
        "retry-hls",
        "cancel",
        "fast",
    ] {
        let output = root.0.join(case);
        fs::create_dir(&output).unwrap();
        let enabled = case != "native";
        let rpc = enabled.then(RpcConfig::new).transpose().unwrap();
        let port = rpc.as_ref().map(|r| r.port);
        let mut monitor = rpc.clone().map(RpcMonitor::new).transpose().unwrap();
        let aria = Aria2cConfig {
            enabled,
            parallel_connections: 4,
            ..Default::default()
        };
        let aria_status = Aria2cStatus {
            available: true,
            executable_path: Some("aria2c".into()),
            ..Default::default()
        };
        let extra = aria2c::aria2c_downloader_args(&aria, &aria_status, rpc.as_ref()).unwrap();
        let request = DownloadRequest {
            url: format!("{base}/video.mp4"),
            format_selector: if case == "separate" { "v+a" } else { "best" }.into(),
            label: case.into(),
            output_format: OutputFormat::Mp4,
        };
        let mut cmd = video_download_command(&tools, &output, &request, None, &extra);
        if matches!(case, "hls" | "broken-hls" | "retry-hls" | "native-fallback") {
            // Replace the URL while retaining production arguments.
            let args: Vec<_> = cmd.get_args().map(|a| a.to_owned()).collect();
            cmd = download_process_command(&tools.yt_dlp);
            cmd.args(&args[..args.len() - 1]).arg(format!(
                "{base}/{}",
                match case {
                    "broken-hls" => "broken.m3u8",
                    "retry-hls" => "flaky.m3u8",
                    _ => "index.m3u8",
                }
            ));
            cmd.args([
                "--abort-on-unavailable-fragments",
                "--fragment-retries",
                "1",
            ]);
            if case == "native-fallback" {
                cmd.args(["--downloader", "m3u8:native"]);
            }
        } else {
            let formats = if case == "separate" {
                json!([
                    {"format_id":"v","url":format!("{base}/video.mp4"),"ext":"mp4","vcodec":"h264","acodec":"none"},
                    {"format_id":"a","url":format!("{base}/audio.m4a"),"ext":"m4a","vcodec":"none","acodec":"aac"}
                ])
            } else {
                json!([{"format_id":"v","url":format!("{base}/{}", match case { "failure"=>"missing.mp4", "unknown"=>"unknown.mp4", _=>"video.mp4" }),"ext":"mp4","vcodec":"h264","acodec":"none"}])
            };
            let info = output.join("input.json");
            fs::write(&info, serde_json::to_vec(&json!({"id":case,"title":case,"extractor":"generic","webpage_url":request.url,"formats":formats})).unwrap()).unwrap();
            cmd.arg("--load-info-json").arg(info);
        }
        if case != "fast" {
            cmd.args(["--limit-rate", "32K"]);
        }
        cmd.args(["--proxy", "", "--retries", "0"]);
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let pid = child.id();
        let router = Arc::new(ProgressRouter::default());
        let events = Arc::new(Mutex::new(Vec::new()));
        let (out_events, out_router) = (events.clone(), router.clone());
        let stdout = child.stdout.take().unwrap();
        let stdout = thread::spawn(move || {
            let mut path = None;
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(p) = parse_progress_line(&line) {
                    out_router.native(p, |p| out_events.lock().unwrap().push(("native", p)));
                }
                if let Some(p) = line.strip_prefix(OUTPUT_PATH_PREFIX) {
                    path = Some(p.to_owned());
                }
            }
            path
        });
        let mut stderr = child.stderr.take().unwrap();
        let stderr = thread::spawn(move || {
            let mut s = String::new();
            stderr.read_to_string(&mut s).unwrap();
            s
        });
        let finished = Arc::new(AtomicBool::new(false));
        let watchdog_done = finished.clone();
        let watch_router = router.clone();
        let cancel = case == "cancel";
        let watchdog = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(if cancel { 2 } else { 40 });
            while !watchdog_done.load(Ordering::SeqCst) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(20));
            }
            if !watchdog_done.load(Ordering::SeqCst) {
                watch_router.stop();
                let _ = kill_process_tree(pid);
                true
            } else {
                false
            }
        });
        let result = wait_for_download(&mut child, monitor.as_mut(), &router, |p| {
            events.lock().unwrap().push(("rpc", p))
        });
        router.stop();
        if result.is_err() {
            let killer = thread::spawn(move || kill_process_tree(pid));
            let _ = child.wait();
            let _ = killer.join();
        }
        finished.store(true, Ordering::SeqCst);
        let timed_out = watchdog.join().unwrap();
        let saved = stdout.join().unwrap();
        let err = stderr.join().unwrap();
        assert!(
            !timed_out || cancel,
            "{case}: timed out: {}",
            rpc.as_ref().map_or(err.clone(), |r| r.redact(&err))
        );
        let status = result.unwrap_or_else(|e| {
            panic!(
                "{case}: {e}; {}",
                rpc.as_ref().map_or(err.clone(), |r| r.redact(&err))
            )
        });
        assert_eq!(
            status.success(),
            !matches!(case, "failure" | "broken-hls" | "cancel"),
            "{case}: {err}"
        );
        let events = events.lock().unwrap();
        let rpc_count = events
            .iter()
            .filter(|(source, p)| {
                *source == "rpc" && p.percent.is_some_and(|n| n > 0.0 && n < 100.0)
            })
            .count();
        if matches!(case, "http" | "separate") {
            assert!(rpc_count > 0, "{case}: no live numeric RPC progress");
        }
        if case == "native" {
            assert!(events.iter().any(|(source, p)| *source == "native"
                && p.percent.is_some_and(|n| n > 0.0 && n < 100.0)));
        }
        if case == "native-fallback" {
            assert!(monitor.as_ref().unwrap().session.is_none());
            assert!(events.iter().any(|(source, _)| *source == "native"));
        }
        if case == "separate" {
            let p = saved.as_ref().unwrap();
            let probe = background_command("ffprobe")
                .args([
                    "-v",
                    "error",
                    "-show_entries",
                    "stream=codec_type",
                    "-of",
                    "json",
                    p,
                ])
                .output()
                .unwrap();
            let probe: Value = serde_json::from_slice(&probe.stdout).unwrap();
            assert_eq!(probe["streams"].as_array().unwrap().len(), 2);
        }
        if let Some(port) = port {
            assert!(
                std::net::TcpStream::connect(("127.0.0.1", port)).is_err(),
                "{case}: listener left running"
            );
        }
        #[cfg(unix)]
        assert!(
            !process_group_exists(pid),
            "{case}: child process left running"
        );
        eprintln!(
            "{case}: status={status}, live_rpc_updates={rpc_count}, saved={}",
            saved.is_some()
        );
    }
}
