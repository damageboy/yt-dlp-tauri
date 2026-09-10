# MODIFIED Requirements

## Required installation and configuration

aria2c SHALL be required for tool readiness, metadata parsing and downloads regardless of the usage toggle. Missing or invalid configuration SHALL block readiness and direct the user to executable selection/PATH repair. Disabling usage SHALL NOT uninstall aria2c or waive required setup.

## Homebrew management

Both macOS targets SHALL include the `aria2` formula. Missing `aria2c` SHALL trigger `brew install aria2`; outdated aria2 SHALL participate in normal update handling; reinstall SHALL include aria2. Tool verification SHALL list aria2c with the other required tools even when usage is off.

## Optional usage

Only enabled usage SHALL add `--downloader` and validated `aria2c:-j N -x N -s N` arguments. Disabled usage and metadata SHALL retain their yt-dlp argument vectors while still checking required setup. Defaults and saved schema remain compatible.

## Recovery

Saving configuration SHALL refresh tool readiness. A configured executable failure SHALL NOT be advertised as a managed package failure when package installation cannot repair it. Remote revision detection SHALL NOT replace configuration guidance with an update action.
