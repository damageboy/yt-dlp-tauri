# Self-contained toolchain baseline 20260912.1

The current manifest depends on binary assets spread across three releases. Publish all four packages for five Windows tools in `toolchain-20260912.1`, preserving existing versions, original upstream identities, license evidence and every binary hash. Use existing candidate validation and publication workflows; no runtime or pipeline behavior changes.

Success requires all runtime URLs pointing to the new release, native Windows validation, stable-channel promotion, and verified Windows/macOS installers containing the new manifest. Only then delete the five previous revision releases and their tags, preserving `toolchain-stable` and `master-build`.
