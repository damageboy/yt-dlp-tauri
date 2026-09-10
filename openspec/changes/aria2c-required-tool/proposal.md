# Require aria2c installation, keep usage optional

## Why

The user requires aria2c as part of tool setup regardless of whether downloads use it. The previous optional-installation boundary was incorrect.

## What changes

Homebrew manages `aria2` alongside yt-dlp, FFmpeg and Deno. Verification and metadata/download preflight require aria2c with either toggle state. Only enabled usage adds external-downloader arguments. Existing executable selection and 1–16 parallelism settings remain compatible. Saving refreshes readiness. Current Windows archives still require a configured executable; managed Windows distribution is pending user clarification.

## Discipline Skills

`doubt-driven-review`, `observability-instrumentation`; eng-disciplines package is unavailable. Apply concrete regression tests and independent review using available tools.
