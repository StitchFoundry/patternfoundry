# PatternFoundry — instructions for agents

PatternFoundry is **sewing-pattern design software** by **SiteSentrix**: draft garment patterns
from body measurements, adjust and grade them, add seam allowance, notches and grainlines, and
print them at true scale (home printer tiles, copy-shop A0, projector) or export them for cutting.
Siblings: [StitchFoundry](https://github.com/Stitchwright/StitchFoundry) (embroidery digitizing,
built on VectorCraft), LightFoundry (laser) and MakerFoundry (the marketplace, where patterns can
be sold too).

**Planned base:** like StitchFoundry, PatternFoundry is VectorCraft plus its own mode: a
clean-room, Rust-native vector illustration app (macOS, Windows, Linux, web/WASM). Pattern
pieces are paths; drafting, grading and printing are commands. Merging VectorCraft in is the
first task (`docs/plan.md`, PF-0). Once it is in, **every rule in
[`docs/vectorcraft-CLAUDE.md`](docs/vectorcraft-CLAUDE.md) applies here unchanged** (Adobe
clean-room, assets, no panics, everything is a command, layering, thin UI, Rust only, never
break wasm, `cargo xtask ci` before every commit, progress docs), with "StitchFoundry" read as
"PatternFoundry".

## Start every session here
1. Read [`docs/start-here.md`](docs/start-here.md) (the owner, what exists, decisions), then
   [`docs/plan.md`](docs/plan.md) (the product, the milestones and task ids) and
   [`docs/market.md`](docs/market.md) (the software already out there, as features to match).
2. Unless the user gives you a task, take the next unfinished one. Orient → plan → implement +
   test → verify → record → commit.

3. **Progress notes:** a PR never edits `PROGRESS.md`. Add one file
   `progress/<yyyy-mm-dd>-<issue-or-pr-number>.md` (done + next 3 steps); a separate roll-up PR folds
   them into `PROGRESS.md`. See [`progress/README.md`](progress/README.md) and `AGENTS.md`.

## Non-negotiables for pattern work
- **Clean room, no copying.** We build our own software that matches and beats what is out
  there; we never copy it. Behaviour comes only from public documentation (vendor help pages,
  manuals, published standards, public videos and forum posts), cited by URL. Never install,
  run, decompile or measure commercial pattern software (Gerber AccuMark, Lectra Modaris,
  Optitex, Tukatech, CLO, Wild Ginger, Garment Designer, PatternMaker…) to recreate it, and never
  copy their code, text, icons, block libraries or sample files. Never copy GPL/AGPL code:
  **Seamly2D and Valentina are GPL**, so read their public docs for ideas only. Never copy
  Roidery code. Sewing-pattern drafting methods from books and schools may be implemented from
  their published descriptions; their text and drawings are not copied.
- **True scale is sacred.** A pattern printed at 100% must measure exactly: every print and
  export path has a test that checks a 10 cm test square, and tiled pages carry alignment marks
  and the test square. Units are exact (mm internally; inches shown when chosen).
- **Measurements are personal data.** Body measurements stay on the user's computer unless they
  choose to share them; never log them.
- **Standards.** Exports follow the published specifications (DXF-AAMA/ASTM D6673 for cutting
  rooms, PDF/ISO 32000, SVG), tested against files written from those specifications.

## Debugging
When a test, a command or a print fails and the cause is not obvious, follow the `/debug` skill
(`.claude/skills/debug/SKILL.md`, based on Kuhlmann, *The Book of Debugging*, No Starch Press;
chapter notes in [`docs/debug-book-notes.md`](docs/debug-book-notes.md)). The rules that matter most:
- **Reproduce first, and make the loop fast**: the narrowest `cargo test -p <crate> <name>` or
  `patternfoundry-cli run` that fails, not a full `cargo xtask ci` per question. Check that you are
  running what you think (stale `target/`, another app on the control port).
- **Look at what the program actually did** before theorising: the command's JSON result,
  `ui.screenshot` / `ui.render`, the exported file, the log; for a scale bug, measure the 10 cm test
  square in the output file itself. Add a switchable probe or a command when
  you keep wanting to see something; never log body measurements.
- **State a hypothesis and how you will test it** before building; one change per run.
- **Do not give up on an idea after one bad run, and do not trust a baseline you have not checked.**
- **Change strategy** after two or three failed guesses (or about an hour of guessing): `git bisect` when it used to work, cut the
  case down to a minimal repro, follow the data to where it first goes wrong.
- **Fix the root cause, then verify**: reproduce, undo and see it return, re-apply and see it go;
  search for the same mistake elsewhere; turn the repro into a test.
- Keep a log of leads, experiments and ruled-out areas in `docs/debug-logs/YYYY-MM-DD-<topic>.md`.

## Commit authorship
Commits are made by **Claude**, with Bruce Wattendorf credited as co-author (owner's decision,
2026-10-21; it replaces the earlier rule that commits were authored as Bruce).
- `git config user.name "Claude"` and `git config user.email "noreply@anthropic.com"` before the
  first commit.
- End every commit message with: `Co-Authored-By: Bruce Wattendorf <bwattendorf@gmail.com>`
- One task id per commit (`PF-1.2: seam allowance`). No model names in commits, code or docs.

## Licence
MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). VectorCraft's own licence and brand terms
apply to the merged code: carry no ArtCraft name, wordmark or logo (saying in plain text that
PatternFoundry is based on VectorCraft by the ArtCraft team is fine).
