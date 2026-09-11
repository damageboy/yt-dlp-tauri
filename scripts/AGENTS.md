# Toolchain records

| File | Purpose |
| --- | --- |
| `check-toolchain-freshness.mjs` | Checks owned archive URLs and live source availability; rejects foreign runtime mirrors; See change: owned-toolchain. |
| `download-tools.ps1` | Restores five Windows tools from verified manifest assets; requires aria2c; See change: owned-toolchain. |
| `prepare-toolchain-publication.mjs` | Builds publication metadata and inputs from verified PR bytes; excludes application-release dependency; See change: owned-toolchain. |
| `publish-toolchain.mjs` | Selects owned archive publication and rollback plans; verifies uploaded asset digests; See change: owned-toolchain. |
| `resolve-toolchain-artifact.mjs` | Resolves master merge to exact PR validation artifacts; accepts explicit candidate dispatch; See change: owned-toolchain. |
| `verify-windows-toolchain.ps1` | Installs owned five-tool revision in fresh temp root and runs native aria2 RPC lifecycle; See change: owned-toolchain. |
