# Progress

- 2026-11-06 (issue 121): removed `/crates/affinity/fuzz` from the Dependabot cargo directories. It is a standalone workspace with a path dependency on its parent crate, which Dependabot cannot resolve alone, so the job failed every run. Config-only change; no Rust touched.
- 2026-11-07 (nightly sweep): no reported bugs awaited triage; `cargo audit` shows no vulnerabilities, only "unmaintained" notices and #122's yanked async-executor; no new findings filed.
- 2026-11-07 (issue 131): `cargo update -p tokio --precise 1.53.3` clears RUSTSEC-2026-0386 (lockfile only). The full test run filled the session disk (~32 GB target), so this change was checked with `cargo xtask wasm` and clippy; CI runs the full gate.
- 2026-11-09 (issue 128): repo links, the Pages URL and the update-check owner now point at StitchFoundry (app ids `com.sitesentrix.*` unchanged on purpose).
- 2026-11-09 (issue 127): customer-facing "SiteSentrix" is now "StitchFoundry"/"Foundry" in the About box, account text, i18n catalogues, AAMA author string, README and packaging metadata. URLs, app ids and legal/copyright lines were left alone (they wait for Bruce). Checked with pf-aama tests only; the full gate did not fit the session disk, so CI runs it.
- 2026-11-11 (issue 141): `?services=staging` switch for the web app: compiled-in staging cloud + MakerFoundry hosts (exact match, https), tab-session only, banner, `?open=` accepts the staging host only in staging mode; Publish Name field labelled. Test: `staging_switches_the_addresses_and_nothing_else_does` in pf-ai.
