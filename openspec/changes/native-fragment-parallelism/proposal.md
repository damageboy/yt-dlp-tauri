# Share download parallelism across native fragments and aria2c

## Why

HLS/DASH uses yt-dlp native downloading even when aria2c is enabled. The saved parallelism must apply to native transfers too.

## What changes

Always pass `--concurrent-fragments N` using saved `parallelConnections` (1–16, default 16). Checked aria2c additionally retains its executable, `-j N -x N -s N`, and authenticated RPC settings. Unchecked aria2c adds no external downloader or RPC settings. Keep metadata commands, tool requirements and persistence unchanged. Update English/Chinese help.

## Acceptance criteria

- Both toggle states pass exactly one `--concurrent-fragments N` before the URL.
- Enabled aria2c keeps its existing arguments and RPC lifecycle.
- Native HLS fallback receives saved parallelism with aria2c enabled.
- Boundary values and a non-default value are covered; command fixture checks spawned arguments.
