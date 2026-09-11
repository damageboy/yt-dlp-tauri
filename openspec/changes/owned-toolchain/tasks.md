# Implementation plan: owned-toolchain

## Global constraints

Use only `damageboy/yt-dlp-tauri` for runtime toolchain release hosting. Preserve historical assets byte for byte and preserve license/source evidence. Install Windows x64 aria2 from the exact official 1.37.0 ZIP. Keep macOS Homebrew behavior and aria2 usage disabled by default. Do not touch the user's uncommitted UI changes. No new repository.

1. Mirror all original toolchain release assets, verify GitHub SHA-256 and sizes, add aria2 archive and licenses, and generate the owned lock/manifest. Verify downloaded copies after uploading.
2. Migrate Rust runtime trust and managed Windows aria2 discovery. Test rejection of foreign cached manifests and successful five-tool activation.
3. Migrate candidate/publishing workflows and scripts to same-repository master and pinned aria2. Test archive digests, collision refusal, and branch/token contracts.
4. Wire native Windows clean toolchain and RPC verification into release builds; update documentation and active project identity. Review the complete diff.
5. Publish the owned stable channel, integrate the verified branch, push master, and verify Windows/macOS CI releases. Remove the original Git remote. Record exact outcomes.
