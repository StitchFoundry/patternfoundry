# Progress

- 2026-11-06 (issue 121): removed `/crates/affinity/fuzz` from the Dependabot cargo directories. It is a standalone workspace with a path dependency on its parent crate, which Dependabot cannot resolve alone, so the job failed every run. Config-only change; no Rust touched.
- 2026-11-07 (issue 122): `cargo update -p async-executor` moves Cargo.lock from the yanked 1.14.0 to 1.14.1. Lockfile-only; `cargo xtask ci` passes. The optional weekly `cargo audit` workflow is not added (needs review).
