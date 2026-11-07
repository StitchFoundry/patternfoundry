# Progress

- 2026-11-06 (issue 121): removed `/crates/affinity/fuzz` from the Dependabot cargo directories. It is a standalone workspace with a path dependency on its parent crate, which Dependabot cannot resolve alone, so the job failed every run. Config-only change; no Rust touched.
- 2026-11-07 (nightly sweep): no reported bugs awaited triage; `cargo audit` shows no vulnerabilities, only "unmaintained" notices and #122's yanked async-executor; no new findings filed.
- 2026-11-07 (issue 131): `cargo update -p tokio --precise 1.53.3` clears RUSTSEC-2026-0386 (lockfile only). The full test run filled the session disk (~32 GB target), so this change was checked with `cargo xtask wasm` and clippy; CI runs the full gate.
