# Toolchain records

| File | Purpose |
| --- | --- |
| `archive-channel.mjs` | Resolves owned prerelease channel; verifies exact URLs, sizes and digests without repository-wide immutability; See change: owned-toolchain. |
| `archive-contract.mjs` | Restricts archive descriptors and naming policy to damageboy/yt-dlp-tauri; See change: owned-toolchain. |
| `artifact-handoff.mjs` | Binds merged PR to successful native validation and artifact identity; accepts exact-head manual dispatch; See change: owned-toolchain. |
| `candidate-bundle.mjs` | Builds and verifies digest-addressed candidate bytes using owned archive descriptors; See change: owned-toolchain. |
| `channel.mjs` | Parses channel records and binds manifest assets to owned revision release; See change: owned-toolchain. |
| `generate-manifest.mjs` | Generates deterministic five-tool Windows manifest including aria2c with owned archive URLs; See change: owned-toolchain. |
| `migration-baseline.mjs` | Rebinds historical baseline downloads to owned mirror only when committed migration inventory size and SHA-256 match; See change: owned-toolchain. |
| `policy.mjs` | Validates archive policy and pinned source release plus reviewed asset digest; See change: owned-toolchain. |
| `publication-plan.mjs` | Plans verified prerelease publication without overwrites; retains digest-bound historical reuse and rollback; See change: owned-toolchain. |
| `resolve-lock.mjs` | Resolves upstream releases and inspected binaries; pinned selection enforces committed digest and version; See change: owned-toolchain. |
| `validation-report.mjs` | Requires all five Windows tools including aria2c and extracted hashes before publication; See change: owned-toolchain. |
