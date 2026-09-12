# Workflow records

| File | Purpose |
| --- | --- |
| `ci.yml` | Tests frontend and Rust on pull requests and master pushes; verifies native macOS app; See change: fork-release-builds. |
| `release.yml` | Builds master/tag Windows and macOS installers; gates Windows on exact published toolchain, fresh install/RPC and installer lifecycle; See change: fork-release-builds. See change: owned-toolchain. |
| `toolchain-canary.yml` | Runs daily Windows checks against owned stable channel; See change: owned-toolchain. |
| `toolchain-discover.yml` | Creates reviewed weekly candidates on master with scoped repository token and explicit validation dispatch; See change: owned-toolchain. Creates English-only candidate commit messages; See change: english-only-app. |
| `toolchain-freshness.yml` | Checks tool availability and creates focused repair PRs with explicit validation dispatch; See change: owned-toolchain. Creates English-only repair commit messages; See change: english-only-app. |
| `toolchain-publish.yml` | Publishes native-validated candidate bytes as owned prereleases; refuses conflicting existing assets; promotes stable channel; See change: owned-toolchain. |
| `toolchain-validate.yml` | Validates candidate bundle on Windows from PR or explicit dispatch; revalidates exact bytes before publication; historical baseline uses inventoried own mirror and explicit four-tool compatibility; diagnostics retain candidate-only tools; See change: owned-toolchain. |
