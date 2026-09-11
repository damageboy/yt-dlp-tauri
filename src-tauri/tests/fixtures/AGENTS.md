# Native test fixtures

| File | Purpose |
| --- | --- |
| `aria2c_probe.rs` | Supplies compiled version, failure, timeout, argv, child-process and orphaned-parent fixtures; See change: aria2c-external-downloader. See change: aria2c-rpc-progress. |
| `rpc_media_server.py` | Generates disposable FFmpeg media and serves HTTP, unknown-length and broken-fragment fixtures; See change: aria2c-rpc-progress. |
