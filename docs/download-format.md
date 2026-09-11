# Download output format

`index.html` places `#output-format` left of `#quality`, with labels above both controls. Options are `.mkv` and `.mp4`; MP4 preserves the previous default. `src/styles.css` caps Quality width at 240px and aligns 42px selectors with download buttons. Narrow windows place actions below selectors.

`downloadCurrentVideo` sends `output_format` alongside the selected quality's `format_selector`. Both selectors lock while busy. Selection remains available for the next download within the current session.

Rust `DownloadRequest` accepts only `mkv` and `mp4` through `OutputFormat`; an omitted field defaults to MP4. `video_download_command` passes the selection to both `--merge-output-format` and `--remux-video`. Remuxing covers sources that already contain audio and video and therefore bypass merging. The output template uses `%(ext)s`; the completion path comes from yt-dlp's `after_move` event.

Format selects the container without re-encoding. Existing quality selection stays unchanged. Unsupported codec/container combinations report yt-dlp's error rather than silently producing another container. See [yt-dlp post-processing options](https://github.com/yt-dlp/yt-dlp#post-processing-options).

See change: download-format-selector.
