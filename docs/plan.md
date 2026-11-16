# PatternFoundry: the plan

**Status:** 2026-10-20. PF-0 to PF-4 done and PF-5 started: PatternFoundry is VectorCraft renamed,
with pattern pieces, true-scale printing, drafting from measurements, bodice, skirt, sleeve and
trouser blocks, and the first design tools (dart rotation, splitting, slash and spread, seam
matching, facings, knife pleats: a princess-seam bodice from the block). Owner: Bruce Wattendorf
(SiteSentrix).

## What it is
Software for designing sewing patterns, for home sewists, small pattern designers and small
studios: draft from measurements, change the design, grade sizes, and print or export at true
scale. Free and open source; AI help paid with StitchFoundry-style credits.

## How it is built
VectorCraft (github.com/storytold/vectorcraft), the clean-room Rust vector editor StitchFoundry
is built on, plus a pattern mode, the way StitchFoundry adds embroidery:
- pattern pieces are paths with pattern data (seam allowance per edge, notches, grainline,
  piece name, cut count, fabric);
- drafting, grading, truing and printing are engine commands (`piece.*` and later families;
  VectorCraft already uses `pattern.*` for fill patterns), so the menus, the control channel, the
  CLI and MCP all reach them;
- a pattern core crate (measurements, formulas, blocks, grading) below the UI layers, as
  StitchFoundry's `sf-core` is.
- reuse before rewriting: CADCraft (same licence; see `docs/start-here.md`) has the constraint solver
  for formulas (PF-3, PF-5), the DXF reader/writer for cutting-room export (PF-7) and layout plotting
  (PF-2).

## Milestones and task ids
| Id | What | Done when |
|---|---|---|
| PF-0 | Merge VectorCraft in (as StitchFoundry did, with `upstream` = storytold/vectorcraft); rename the app to PatternFoundry; CI green | the app opens as PatternFoundry on all platforms and the web |
| PF-1 | Pattern pieces: piece data on paths, seam allowance (per edge, corners mitred/squared), notches, grainline, labels, a Pieces panel | a piece prints with its allowance and marks |
| PF-2 | True-scale printing: tiled Letter/A4 with alignment marks and the 10 cm test square, A0/36-inch copy-shop PDF, layers per size | a printed tile measures exactly |
| PF-3 | Measurements: size charts and personal measurement sets (kept on the computer), formulas in point positions so a draft follows the measurements | changing the bust redraws the bodice |
| PF-4 | Blocks: drafted bodice, skirt, trouser and sleeve blocks from published drafting methods | a bodice block drafts from measurements |
| PF-5 | Design tools: dart move/rotate, slash and spread, add fullness, pleats, facings, collars, pockets; truing (walk seams, match lengths, smooth curves across seams) | a princess-seam bodice from the block |
| PF-6 | Grading: by rules per point and by redrafting from each size; nested size view | a size range from one pattern |
| PF-7 | Export for cutting: DXF-AAMA/ASTM D6673, SVG; projector mode | a DXF opens in a cutting-room tool |
| PF-8 | AI help (StitchFoundry's AI layer): draft from a description, fit advice from muslin photos, step-by-step help | "a wrap dress for my measurements" drafts |
| PF-9 | Marker making / nesting; selling patterns on MakerFoundry | a marker for 2 m of fabric |
| PF-10 | 3D fit preview (from public documentation only): avatar from measurements, sewing lines, cloth simulation, drape view beside the 2D pattern; glTF/OBJ export for game and 3D artists | a skirt drapes on an avatar of the user's size |
| PF-11 | Publish to MakerFoundry: File › Publish to MakerFoundry… sends the document with the Foundry Account session as a private draft (MakerFoundry docs/api.md, "How publishing from an app works"): SVG now, listed as a vector (laser) design; true-scale and tiled PDF (PF-2), pieces (PF-1) and DXF-AAMA (PF-7) once MakerFoundry has a sewing-pattern kind | the desktop app publishes a draft and opens its page |
| PF-12 | Open from a MakerFoundry link: the web app's `?open=` (MakerFoundry docs/api.md, the file address), checked to https://makerfoundry.app/api/v1/designs/…/file/… only, fetched without cookies or redirects | the web app opens a design from its MakerFoundry address |

## Decided
- Domain: **patternfoundry.app**, bought by the owner (2026-10-19). Installers and the app link to
  the GitHub repository until the site is live; switching them is a one-line change each.

- Hosting (2026-10-19): the app stays on **GitHub Pages** (the web build) and **GitHub Releases**
  (the desktop downloads), as StitchFoundry does. No server of its own, no Railway.
- Accounts and AI (2026-10-19): PatternFoundry uses the shared **Foundry Account** (one sign-in and
  one credit balance, "Foundry credits", across all SiteSentrix apps), served by the StitchFoundry
  cloud service at https://cloud.stitchfoundry.app. PatternFoundry never holds an Anthropic key:
  AI requests go through that service, which adds the owner's key and charges the user's credits.
  The Foundry Account is the only way to pay for AI; there is no own-key option (#140). Details for PF-8
  are in `docs/start-here.md`.

- Drafting method (2026-10-19): **Aldrich** (Winifred Aldrich, *Metric Pattern Cutting for
  Women's Wear*), British metric, first. Blocks are built from public descriptions of the method
  (cited by URL in `crates/pf-core/src/block.rs`), never from the book's text or drawings.
- Sleeve method (2026-10-20): **Natalie Bray**'s one-piece sleeve, from a public tutorial
  (cited in `crates/pf-core/src/block.rs`): no public description of Aldrich's sleeve was found,
  and her book's own text is not copied.
- Trouser method (2026-10-20): **The Shapes of Fabric**'s basic pants block (metric, women's;
  the same author as the skirt block), from her public tutorial, cited in
  `crates/pf-core/src/block.rs`. Fashion Capital's was the other candidate (more detailed curves,
  but a step cut off in its text).

- App or mode (2026-10-30, replacing the decision of 2026-10-23 that it become a mode inside
  StitchFoundry): PatternFoundry is **its own app** and stays in this repository; it is **not
  combined into StitchFoundry**. PF-11 (the move) is dropped.
- Sleeve ease (2026-10-23): the sleeve block drafted where there is a bodice block is set into its
  armhole with the middle of a classic sleeve's ease (PF-4.9), not drafted from the top arm table
  alone.

- MakerFoundry (2026-10-31, the owner's decision): publish the document as **SVG now**
  (PF-11), listed as a generic vector design of MakerFoundry's laser kind, its title from the
  file name; written so true-scale and tiled PDF (PF-2), pieces (PF-1) and DXF-AAMA (PF-7) can be
  sent later. A **sewing-pattern kind** (true-scale PDF, SVG, DXF-AAMA/ASTM D6673, size range,
  garment type, tile sizes) is queued as MakerFoundry issue #47; the owner decides when.
  Opening from a link (PF-12) follows MakerFoundry's file address contract. Only the `sfs_`
  session token is ever sent (never an Anthropic key); the draft page opens only if it is on the
  site the request went to. The web app's origin (https://stitchfoundry.github.io) has to be added
  to MakerFoundry's MF_API_ORIGINS before the web app can publish or open (the desktop app is
  unaffected); a content security policy, if one is added, names both MakerFoundry hosts in
  connect-src, with a test that every inline script's SHA-256 is in script-src. The owner agreed
  to a live check: a throwaway Foundry account and one tiny test draft.

## Open decisions (the owner's)
- None at present.

## Progress log
- 2026-11-01, **PF-8.1**: the Foundry Account's "plain HTTP only on this computer" check
  (`pf_ai::cloud::clean_server`) compares the host whole instead of the address's start, and
  refuses an address with a user name or password in it: `http://localhost:80@evil.com` passed
  before, and would have sent the session token and password in clear text to evil.com.
- 2026-11-01, **PF-13**: links to the MakerFoundry YouTube channel and the Buy Me a Coffee page.
  Help › MakerFoundry on YouTube (`help.youtube`) and Help › Support PatternFoundry (Buy Me a
  Coffee) (`help.donate`) return their URL as `help.github` does and open it from the menu; the
  About and Home links row shows both. The Windows installer's last page, the macOS DMG window and
  the Linux AppStream data (a donation URL) name them, and the README has a "Videos and support"
  section.
  The first v0.1.1 release build failed on Windows: the installer file already set the finish-page
  text, and WiX refuses a second one (WIX0091). Fixed with one combined text; a workspace test
  (`xtask/src/wix.rs`) now fails on any duplicate WiX `Id`, so this is caught in PR CI.
- 2026-10-31, **PF-12**: the web app opens MakerFoundry links:
  `https://stitchfoundry.github.io/patternfoundry/?open=<URL-encoded design file address>`. The
  address must be `https://makerfoundry.app/api/v1/…` (or `www.`), a path of `a-z 0-9 - . /`
  without dot segments, and at most `?t=` with 48 lowercase hex digits; it is rebuilt from the
  checked parts (`pf-ai::makerfoundry::open_address`) and taken out of the page address once
  read. The fetch sends no cookies, follows no redirect, stops after 60 s and reads at most 32 MB;
  the file is named from `Content-Disposition` or the address, must be SVG or DXF, and opens as a
  dropped file does. Refusals show MakerFoundry's own sentence. The web publish request (PF-11)
  now also sends no cookies and follows no redirect. Tested in Chromium against a mocked
  MakerFoundry; `apps/vectorcraft-web/src/csp.rs` checks any future content security policy
  (both MakerFoundry hosts in `connect-src`, every inline script's SHA-256 in `script-src`).
  Waiting on MakerFoundry: `https://stitchfoundry.github.io` in its `MF_API_ORIGINS`, then the button.
- 2026-10-31, **PF-11**: File › Publish to MakerFoundry… (`ui.publishToMakerFoundry`). The
  document goes as SVG, named by the person (the draft's title comes from the file name), with
  their Foundry Account session (`Bearer sfs_…`, never another key) to
  `https://makerfoundry.app/api/v1/designs`; the window shows the file, its size, the kind, where
  it goes and the account before anything is sent. The draft page opens only when it is on
  makerfoundry.app; MakerFoundry's own sentence is shown for a refusal (one line, at most 300
  characters). The desktop app no longer follows redirects for requests carrying a key or session.
  Request building and answer reading are in `pf-ai::makerfoundry` (no network code, so the web
  app shares them); property tests in `crates/pf-ai/tests/makerfoundry_props.rs`; each protection
  (host, address characters, token prefix and characters, size, draft address, sentence tidying,
  window open, empty document) was removed in turn and a test failed. Later, through the same
  request: true-scale and tiled PDF (PF-2), pieces (PF-1), DXF-AAMA (PF-7), once MakerFoundry has a
  sewing-pattern kind (its issue #47).
- 2026-10-30, **Dependabot #98**: 27 of its 33 bumps taken (moxcms, hayro-ccitt, harfrust, aes, fax, jpeg-encoder, usvg fixed for); egui 0.37 (craft-ui pins 0.36) and vello 0.3 (root blends no longer reach the page) left out and ignored.
  `.github/dependabot.yml`: weekly on Mondays, at most 5 PRs, minor and patch updates grouped, each major update on its own.
- 2026-10-19, **PF-0**: VectorCraft 0.13.1 (upstream `a466625c`) merged with its history
  (`upstream` = storytold/vectorcraft). The programs are `patternfoundry` and `patternfoundry-cli`
  (the Cargo packages keep VectorCraft's names, so `cargo run -p vectorcraft` still works), the app
  id is `com.sitesentrix.patternfoundry`, documents save as `.patternfoundry` (`.vectorcraft` and
  `.drawcraft` still open), and settings live in their own PatternFoundry folder (VectorCraft's
  are read once). User-visible text, its 22 translations, the installers (MSI with new GUIDs,
  DMG, deb/rpm/AppImage, Flatpak, FreeBSD, web) and the MCP server say PatternFoundry. No ArtCraft
  name, logo or link remains (`cargo xtask brands` now rejects them); the About box credits
  VectorCraft in plain text. An original placeholder icon (a bodice pattern piece over a tape
  measure). `.github/workflows/ci.yml` runs `cargo xtask ci`; VectorCraft's own workflows run only
  in its repository. Not done: a release pipeline for PatternFoundry (signing secrets are the
  owner's), and the VectorCraft progress docs (ROADMAP, parity docs) still describe VectorCraft.
- 2026-10-19, **PF-1**: pattern pieces. `crates/pf-core` (no workspace dependencies, like
  StitchFoundry's `sf-core`) holds the piece data (name, cut count, mirrored pairs, fabric, seam
  allowance per edge in mm, mitred or squared corners, notches, grainline) and the geometry: the
  cutting line, notches (slit, double, V; 6 mm deep, never past the seam line) and the grainline
  arrow. **Pattern › Make Pattern Piece** (`piece.make`) turns each selected closed path into a
  piece: a group whose seam line is the path (dashed), with a solid cutting line, the grainline
  and a label ("Front Bodice / Cut 2 (mirrored pairs) · Linen") generated as locked children, so
  they print and export as ordinary art. Every edit redraws a piece whose seam line or data
  changed (as dimensions follow their art); moving a piece moves its grainline. `piece.set`,
  `piece.addNotch`, `piece.removeNotch`, `piece.info`, `piece.list`, `piece.release`; the
  **Pattern Pieces** panel (Pattern and Window menus) edits name, fabric, cut, mirrored pairs and
  the allowance; every new label translated in all 23 languages. Tests: 15 in `pf-core`, 8 engine
  tests (`tests_piece.rs`: geometry at true scale, following edits, undo, save, export, bad input).
  Not yet: notches and per-edge allowances from the panel (commands only), notches placed by
  clicking, a dock icon for the panel, self-intersection cleanup where inward curves are tighter
  than the allowance, and translations of the commands' error messages.
- 2026-10-19, **PF-2**: true-scale printing, built on VectorCraft's own print tiling (Tile Full
  Pages / Tile Imageable Areas, overlap, margins, tile range, the Print Tiling tool), which
  already existed. New: **Pattern Marks** in File › Print (`patternMarks`): every page carries a
  10 cm and a 1 in test square; every tile its name (rows as letters, columns as numbers: A1 is
  top left, with the grid's size), its printable area's edge and the lines where the next pages
  begin, to trim and tape by (`pf_core::tiling`). Copy-shop paper: A2, A1, A0 and the 36 in roll
  (36 × 48 in). A warning when Tile Full Pages would leave the printer's margins unprinted
  between pages (tile the imageable areas instead). Tests read the PDFs back
  (`crates/pdf/src/tests_truescale.rs`): a 10 cm square measures 10 cm at 100% on Letter, on
  every tile and on A0, A1 and the roll, both test squares are on every page, pages are their
  paper or artboard sizes, and exported PDFs (`document.exportPdf`) keep artboards at 100%.
  Not yet: layers per size (waits for grading, PF-6); the printer driver's own "fit to page"
  can't be checked from here, so the test squares are how a sewist checks it.
- 2026-10-19, **PF-3**: measurements and drafting. The document's measurement set (values in
  mm, and formulas over them: `quarter = (bust + ease) / 4`), a size chart read from a table
  (comma, semicolon or tab separated; mm, cm or in) whose sizes fill in the set, and point
  formulas on any path or a piece's seam line (`x`, `y` in mm from the draft's origin, which
  moves with the path). Every edit places drafted points again, then pieces redraw: changing the
  bust redraws the bodice and its cutting line, in one undo step. `measure.set`, `measure.info`,
  `measure.clear`, `measure.importChart`, `measure.useSize`, `draft.setPoint`, `draft.info`,
  `draft.clear`; the **Measurements** panel (Pattern and Window menus). The formula language is
  CADCraft's `expr.rs`, adapted (same licence; NOTICE). **Personal data:** measurements live in
  the document only; the copies that PDF, SVG and EPS exports embed (Preserve Editing) leave the
  measured values out unless the set's `share` is on (formulas stay); nothing logs them. Tests:
  10 more in `pf-core`, 5 engine tests (`tests_measure.rs`). Not yet: point formulas from the
  panel or the canvas (commands only), personal sets kept apart from documents in the settings
  folder, curves' handles placed by formulas, standard size charts bundled (charts are the
  user's own: standards bodies' tables can't be copied in).
- 2026-10-19, **PF-4**: blocks, Aldrich-style (British metric). **Draft Bodice Block**
  (`block.bodice`: back and front close-fitting half bodices from bust, waist, nape to waist,
  back width, chest, shoulder, neck and armscye depth; 5 cm ease, shoulder and waist darts) and
  **Draft Skirt Block** (`block.skirt`: back and front straight skirt from waist, hip, waist to
  hip and skirt length; 1 cm hip ease, the waist reduction two thirds at the side seams, the rest
  in back and front darts), in the Pattern menu; `block.measurements` names what a block needs
  and what the set lacks. Each part is a path that remembers its block: when a measurement
  changes every edit draws it again (inside a pattern piece too, whose cutting line follows),
  where it was moved to, in one undo step. Built from public tutorials of the method, cited in
  `pf_core::block`; the owner can check them against the book. Tests: 4 in `pf-core`, 5 engine
  tests (`tests_block.rs`). Not yet: sleeve and trouser blocks (PF-4.1, PF-4.2), a block panel
  with the method's options (ease, dart sizes), the bodice's armhole checked against the sleeve
  head once the sleeve exists.
- 2026-10-19/20, **PF-5** (design tools, first parts). **Rotate Dart** (`design.rotateDart`: the
  pivot method; the side of the outline with fewer points swings round the dart's point so the
  dart opens elsewhere on the outline, at a point or anchor; all of it or a share, leaving two
  darts) and **Split at Points** (`design.split`: a closed path cut in two between two points,
  each part closed by a straight seam; at a point two darts share, no seam is added), in the
  Pattern menu (with two or one selected points); `design.length` measures along an outline. Works
  on a piece's seam line too (the piece redraws; a split needs the piece released first). A block
  part changed this way stops following the measurements. Geometry in `pf_core::design`. The
  milestone: a princess-seam bodice from the front block (bust dart into the armhole, split
  through the bust point). **Slash and spread** (`design.spread`, PF-5.1): the outline cut
  straight between two points (anchors, or points on it) and opened by an amount, hinged at the
  far end (flare: a skirt widened at the hem, its waist kept) or parallel (even fullness, room for
  a pleat); a command only for now (it needs an amount: a dialog later). **Truing**
  (`design.matchLength`, PF-5.2): a seam's end point moves along the seam until the seam is a
  given length or as long as another seam (the princess seams then match exactly). **Facings**
  (`design.facing`, PF-5.3; Pattern › Make Facing with two selected points): a new path whose
  outer edge is a run of the outline (a neckline, an armhole) and whose inner edge is that run
  moved inward by the width (5 cm unless given; curves cut into pieces that turn at most 1/32 of a
  turn, within 0.05 mm of the true offset), its ends along the seams either side. **Knife pleats**
  (`design.pleat`, PF-5.4): a parallel spread of twice the pleat's depth along a line from one
  point to another, folding toward the side that stays, with its fold line (solid, an arrow to the
  placement line) and placement line (dashed) added in the piece, so they move with it; a command
  only for now (it needs a depth). **Inverted pleats** (`design.pleat` with `kind: "inverted"`,
  PF-5.5): four times the depth, both edges folded to the middle, two fold lines and arrows and
  one placement line. **Gathering marks** (`design.gather`, PF-5.6; Pattern › Gather with two
  selected points): a wavy line 5 mm inside an edge and a tick across it at each end; with `into`,
  the length it gathers into and the ratio are reported and the mark is named after it. **Flat
  collars** (`design.collar`, PF-5.7; Pattern › Draft Collar with the bodice block's back and
  front selected), from The Creative Curator's published steps (cited in `pf_core::design`): the
  front neckline laid against the back's at the neck point, the shoulders overlapping (3 cm unless
  given; the page says 2 to 4), the neckline lowered if asked (1 cm for a block), the collar 6 cm
  wide unless given, its front rounded to centre front; the neck edge is as long as both
  necklines. Corners where an offset edge turns are now joined by an arc outside and trimmed
  inside (an edge offset twice had spiked out at the neck point: caught by drawing it, then by a
  test). **Patch pockets** (`design.pocket`, PF-5.8): the pocket piece, its finished width and
  depth plus a hem (2.5 cm unless given) folded inside at the top with its fold line (dashed),
  square, rounded or pointed at the bottom; with `on` and `at`, the finished outline marked on the
  garment (dashed, in the piece); a command only for now (it needs a size). **Walking seams**
  (`design.walk`, PF-5.9): two seams that sew together, each from the ends that meet and the
  shorter way round its outline; their lengths and the difference, and with `notches` or `every`
  matching notches at the same distances along both pieces, in one undo step. Walking the skirt
  block's side seams found the front's 0.23 mm shorter (its hip curve's handles left out the rise
  above the waist line); fixed, they now match. **Smoothing across a seam** (`design.smooth`,
  PF-5.10): where a hem, waist, neckline or armhole meets a seam on two pieces that sew together,
  both curves' handles are turned to one line (the second piece laid against the first along the
  seam), so the curve runs on without a corner once sewn; handle lengths are kept and the corner
  there was is reported. **Box pleats** (`design.pleat` with `kind: "box"`, PF-5.11): an inverted
  pleat turned over, the slash spread four times the depth, the fold lines a depth in from the
  spread's edges with arrows out to placement lines a depth past them; the box shows twice the
  depth wide and the fabric meets behind it. **Stand collars** (`design.standCollar`, PF-5.12;
  Pattern › Draft Stand Collar), from The Creative Curator's published method
  (https://www.thecreativecurator.com/draft-mandarin-collar-pattern/): half a collar, centre back
  on the fold, as long as the back and front necklines (measured on the bodice block, or on any
  path given its neck points); the neck edge straight to the shoulder, then curving up 15 mm by
  centre front (so it hugs the neck), on past centre front by the extension (half the button stand
  when the fronts meet, all of it when they overlap); the top edge parallel at the stand's height
  (35 mm), the front end square, the centre front line dashed. **In-seam pockets**
  (`design.inSeamPocket`, PF-5.13; Pattern › In-Seam Pocket), after the published methods of
  Seamwork (https://www.seamwork.com/articles/design-your-own-in-seam-pockets) and Doina Alexei
  (https://www.doinaalexei.com/beginnersewingtutorialblog/video-tutorial-how-to-make-a-sewing-pattern-for-a-skirt-on-seam-pocket):
  a pocket bag hanging inside the piece from its seam, its seam edge following the seam from 15 mm
  above the opening to 15 mm below it (the opening 140 mm long, 90 mm down by default), square in
  by the allowance, then curving out 130 mm and down 150 mm below the opening; on pattern pieces,
  notches at the opening's ends on the seam and on the seam it sews to, in the same undo step.
  **Welt pockets** (`design.weltPocket`, PF-5.14; Pattern › Welt Pocket), after The Shapes of
  Fabric's published tutorial
  (https://www.theshapesoffabric.com/2019/06/24/welt-pocket-tutorial/): single or double welts;
  the welt strip (the opening plus 15 mm at each end, twice the welt plus 7 mm each side, folded
  along its middle; cut two for double welts), the lower bag and the upper one deeper by the
  opening's height, both as long as the welt; with `on` and `at`, the opening box (dashed) and its
  cut lines (along the middle, out to the corners) on the garment. **Shirt collars**
  (`design.shirtCollar`, PF-5.15; Pattern › Draft Shirt Collar), after the published method in
  NIMI's Fashion Design Technology trade theory
  (https://nimi.co.in/cits_ebooks1/Fashion_Design_Technology/Fashion_Design_Technology_TT/files/basic-html/page164.html):
  the stand (a stand collar, 30 mm high, 10 mm rise, 15 mm past centre front) and above it the
  collar that turns down over it, its neck edge as long as the stand's top edge to centre front
  and curving 13 mm down at the front, 40 mm high at centre back, its point 51 mm long and 19 mm
  out. Tests: 19 in `pf-core`, 18 engine tests (`tests_design.rs`). Not yet (PF-5.x): box pleats
  wider than twice their depth (two slashes), notches at a pleat's ends, rounded or winged stand
  fronts, flap pockets; more truing (the princess seam curved through the bust); dart shortening
  from the bust point; design steps replayed when the measurements change; the sewing terms in the
  translations checked by sewists who speak each language.
- 2026-10-20, **PF-4.1**: the one-piece straight sleeve block (`block.sleeve`, Pattern › Draft
  Sleeve Block), Natalie Bray's method from a public tutorial: the crown depth from the top arm by
  the tutorial's table, 5 cm ease round the top arm, the crown through marks on the back and front
  lines and the tutorial's midpoint offsets, straight sides, the wrist curved down at the back and
  up at the front; it follows the measurements (`top_arm`, `sleeve_length`). The result reports
  the crown's length. By the table alone the crown came out about as long as the bodice's armhole
  (1.6 mm short with the test's made-up measurements), not the 3 to 4.5 cm longer a set-in sleeve
  needs, so an optional `sleeve_crown` measurement (a formula such as `armhole + 30`) sets the
  crown's length, and the crown depth is solved to match it. Blocks drafted without an origin now
  start 20 mm inside the artboard by their own extent, so the sleeve, which reaches left of its
  origin, isn't cut off. Tests: 1 more in `pf-core`, 1 more engine test (`tests_block.rs`). Not
  yet: the bodice's armhole measured into the set by a command (it is `design.length` by hand
  now), notches for the crown's balance points, a two-piece sleeve, an elbow dart.
- 2026-10-20, **PF-4.2**: the basic trouser block (`block.trousers`, Pattern › Draft Trouser
  Block), front and back side by side, from The Shapes of Fabric's metric steps: half the hip and
  2 cm ease wide, the crotch line 1.5 cm below the crotch depth, crotch extensions of an eighth of
  the half hip out from the centre front (3 cm more at the back), the centre front 0.5 cm in and 1
  cm down at the waist, the centre back 4 cm in and 2.5 cm above, waists of a quarter waist and
  1.5 cm (front) and 0.5 cm with a 2 × 10 cm dart (back), hems 1 cm under and over half the hem,
  the crease half way between fork and side seam, the knees from the line between hem and fork (1
  cm wider each side at the back). Measurements: `waist`, `hip`, `waist_to_hip` (shared with the
  skirt), `crotch_depth`, `waist_to_knee`, `waist_to_ankle`, `trouser_hem`. The test drafts the
  tutorial's own example and gets its numbers (forks 6 and 9 cm out, hems 22 and 24 cm, waists 20
  and 19 cm). The curves the tutorial leaves to its drawings are chosen in the code and marked;
  the block was looked at as an exported picture. Tests: 1 more in `pf-core`, 1 more engine test.
  Not yet: a front dart (the author's comments add about 1 cm at the front side seam for one;
  done: PF-4.7), walking the inseams and side seams (the tutorial's own check; done: PF-4.6), a
  waistband (done: PF-4.8).
- 2026-10-21, **PF-7.1**: DXF-AAMA export for cutting rooms (`document.export {format: aama}`,
  File › Export › Export As › DXF-AAMA (cutting room)), written from the published descriptions of
  ASTM D6673 (DH Patterns and Fit's overview,
  https://dorthehansen.com/wp-content/uploads/2014/10/ePattern-ASTM-Standard.pdf, and Patro's
  notes, https://fabricesalvaire.github.io/Patro/resources/file-format/dxf-astm.html; the standard
  itself is not bought). DXF R13 (or R12 for older readers): a block per piece, inserted at the
  origin, its cutting line on layer 1, seam line on 14, both copied to the validation layers 84
  and 87, turn points (2) and curve points (3) of both, notches as points on the cutting line
  carrying depth, width and angle (4), the grainline (7), the paths drawn inside the piece as
  internal lines (8), and the piece system text (Piece Name, Quantity as right and left pieces,
  Material); the style system text (Style Name, dates, Author, Sample Size, Units: METRIC, the
  standard's version, Curve Tolerance) on layer 1 at the origin. Millimetres, y up, the pieces
  where they are relative to each other. An unnamed piece goes by its seam line's name; every
  piece name is once in the file. The DXF writer (`vectorcraft-cad`) gained blocks besides model
  and paper space. Checked: the 10 cm square test (its seam line exactly 100 mm across in the
  file, the cutting line 120), and the skirt block's file read by an independent reader (ezdxf
  1.4.5's recover and strict readers: no errors, no fixes), drawn the right way up. Tests: 3 in
  `vectorcraft-cad`, 3 engine tests (`tests_aama.rs`). Not yet: grading (graded nests and the rule
  table, PF-6), drill holes (done: PF-7.6), mirror lines and the NM attribute, notch shapes besides slit and V
  (layers 80 to 83; done but the check notch: PF-7.7), opening AAMA files (done: PF-7.8), an options dialog (the style name and sample size are
  parameters), a test in a cutting-room program. Seen while checking: the cutting line runs its
  allowance round a dart's legs, as if the dart were cut out; a cutting line should bridge the
  dart (PF-1 follow-up).
- 2026-10-21, **PF-1.1**: the cutting line bridges darts. A dart (a sharp inward V of two straight
  legs, closer than 50°) is no longer cut round: the cutting line crosses it with the dart's cap,
  constructed as a pattern maker does by folding the dart closed (pressed toward the side before
  it), drawing the allowance and opening it again; the allowance lines meet the legs where the
  folded line crosses them, equally far from the point, and the cap peaks (or dips, where closing
  the dart bends the seam) over the dart's middle line where the folded-under dart reaches. Found
  while checking the DXF-AAMA export (PF-7.1), whose pieces had their allowance run round the
  darts' legs as if the darts were cut out. Tests: 3 in `pf-core` (`allowance.rs`: nothing of the
  cutting line inside a dart, the cap's points mirrored, the cap folding flat onto a straight
  waist and onto a sloping one); they fail with the dart test switched off. The skirt and bodice
  blocks made pieces were looked at: every waist, shoulder and bust dart bridged. Not yet: a dart
  pressed toward the other side (a choice per dart), curved dart legs, darts inside a curved edge.
- 2026-10-21, **PF-7.1** (moved onto CADCraft's DXF crate, as the owner asked): DXF-AAMA files are
  now written and read on `crates/pf-dxf`, CADCraft's standalone DXF tag reader and writer
  (`crates/dxf`, from the published DXF reference; same licence, copyright kept in `NOTICE`, the
  CADCraft name kept out of the crate's name and messages), by a new `crates/pf-aama` (layer 1):
  the same file as before (R13 or R12, a block per piece, the layers and texts of ASTM D6673), now
  built as DXF tags, and read back (the style and piece system text, cutting and sew lines,
  notches, the grainline and internal lines; lightweight polylines and pieces placed away from the
  origin too), which every file written is checked against. VectorCraft's DXF writer
  (`vectorcraft-cad`) is back as it was. Checked again with ezdxf 1.4.5: the skirt block's pieces
  in R13 and R12, no errors, no fixes, the same extents. Tests: 5 in `pf-aama` (layers, true
  scale, read back as written, an insert and a lightweight polyline read, bad input), the vendored
  crate's 14, the 3 engine tests.
- 2026-10-21, **PF-2.1**: SVG exports at true scale. VectorCraft wrote the SVG's size without a
  unit (`width="283.465"`), which every SVG reader takes as CSS pixels, 96 per inch: a 10 cm
  square came out 75 mm in browsers, Inkscape and cutting software. The size is now in points
  (`width="283.465pt"`, the viewBox unchanged), so it measures 10 cm everywhere. Opening such a
  file again reads a size in points that matches a `0 0 w h` viewBox (our own files) as before, a
  user unit a point, so nothing goes through usvg's pixels and back. Test:
  `crates/svg/tests/true_scale.rs` (the 10 cm test square measured by usvg at 96 per inch: 100 mm;
  75 mm with the fix undone), and the SVG tests that drew exported files with usvg now read them
  at 72 per inch (points as pixels), as they did.
- 2026-10-21, **PF-7.2**: SVG for cutting (`document.export {format: svgcut}`), for laser and
  blade cutters and plotters that take SVG. The page is sized in millimetres
  (`width="…mm"`, viewBox in mm, one user unit a millimetre) with a 10 mm margin round the
  pieces. Each piece's cutting line is a red hairline path (`id="cut-N"`, `data-name`) in a `cut`
  group, notches are blue strokes in `notches`, and with `marks` on (the default) a `draw` group
  carries the dashed seam line, grainline, internal lines and the piece's name and cut count; with
  `marks` off only the cuts and notches are written. Tests: `crates/engine/src/tests_svgcut.rs`
  (a 10 cm square with 10 mm allowance cuts at 120 mm and its seam line at 100 mm, read back from
  the file; without marks only cuts are written).
- 2026-10-21, **PF-7.3**: PDF for projector (`document.export {format: projector}`), for
  projecting a pattern onto fabric on the cutting table, made as projector sewists describe such
  files (https://www.craftstorming.com/2020/05/tips-for-using-pdf-sewing-patterns-on-a-projector):
  one page at 100% holding every piece, set side by side (`layout: row`, the default; blocks are
  drafted touching, so their allowances would overlap) or where they are drawn (`asDrawn`), with
  a margin round them (`margin`, 25 mm); cutting lines and notches 3 pt thick (`lineWidth`),
  dashed seam lines, grainlines, and names and cut counts in 10 mm type. PDF layers: a 10 cm grid
  (on), a 4 in grid (off), a 10 cm test square in a band above the pieces, and the pieces; with
  `invert`, white lines on a black page (a "Background" layer); with `mirror`, the page mirrored
  left to right for cutting with the fabric's wrong side up (the type stays readable). Tests:
  `crates/engine/src/tests_projector.rs` reads the PDF back: the test square measures 100 × 100
  mm, the cutting line of a 10 cm square with 10 mm allowance 120 mm, the layers and their
  visibility, inverted and mirrored pages, touching pieces set apart. Not yet: the projector
  view in the app itself (full-screen, calibrated to the cutting mat), pieces cut on the fold
  opened out, layers per size (PF-6).
- 2026-10-21, **PF-6.1**: grading by redrafting (**Pattern › Grade Nest**, `grade.nest`;
  `grade.clear`). A PatternFoundry draft follows its measurements (PF-3), so each size of the
  size chart is the same draft made again from that size's measurements (blocks, point formulas,
  pieces with their allowance, dart caps and notches); grading "allows the one pattern to be made,
  then sized up and down" (https://uphance.com/blog/pattern-grading-a-beginners-guide/). The
  sizes are drawn over each other as a nest on a locked layer "Graded Nest", a sublayer per size
  ("Size 12"), each size in its own colour and the size the document is drafted in heavier; the
  document's own measurements and pieces don't change. `line`: the cutting lines (default) or the
  seam lines; `align`: each piece's sizes lined up where they are drafted (default) or on a corner
  or the centre of their bounds, for every piece or per piece (the skirt: the back on its top
  left, centre back at the waist; the front on its top right). The result lists each size's
  pieces with their width, height and line length in mm, the grade check; a size that draws the
  same as the document (its chart row lacks the measurements the draft uses) is a warning. A
  PDF saved with layers carries a layer per size (PF-2's "layers per size"). The nest stores
  outlines only, never measurements. Tests: `crates/engine/src/tests_grade.rs` (a skirt in three
  sizes: each piece grows by about a quarter of the hip step; size 12 is the document's own
  cutting lines; measurements untouched; replaced, undone, cleared; PDF layers per size;
  alignment; bad requests). Not yet: grade rules per point (PF-6.2) for patterns not drafted
  from measurements, grade rules in DXF-AAMA files, printing one chosen size.
- 2026-10-21, **PF-6.2**: grade rules per point, for patterns that aren't drafted from
  measurements (`grade.setSizes`, `grade.setRule`, `grade.info`). As the cutting-room standard
  describes rule grading (ASTM D6673, from DH Patterns and Fit's overview,
  https://dorthehansen.com/wp-content/uploads/2014/10/ePattern-ASTM-Standard.pdf), chosen
  points of a piece carry a grade rule: how far the point moves in x and y for each size. The
  document keeps a size range (smallest first) and its base size; a rule is either the same move
  at every size step from the base (`step: [dx, dy]`, mm, x right and y down) or each named
  size's move from the base (`sizes: {"L": [dx, dy]}`). Points between ruled points move in
  proportion to how far along the seam line they are (`pf_core::grade::moves`), so a curve
  between two graded points keeps its shape; with one rule the whole piece moves. `grade.nest`
  grades into the size range when there is one: each size is drafted from the size chart's
  measurements where the chart has it, then the rules move the points, so the two kinds of
  grading add up. The rules live on the piece (saved with it); the document is never graded
  itself. Tests: 4 in `pf-core` (sizes, regular and irregular rules, points in proportion,
  wrapping round the outline) and 4 more engine tests (`tests_grade.rs`: a square graded 90, 100,
  110 mm; irregular rules; rules on top of redrafting; bad sizes and rules). Not yet: a grading
  panel (rules from the canvas), grade rules written into DXF-AAMA files (the RUL table),
  printing one chosen size, rules copied between pieces.
- 2026-10-21, **PF-6.3**: one size of a graded pattern written for cutting. DXF-AAMA, SVG for
  cutting and the projector PDF take `size` (`document.export {format: aama, size: "14"}`): the
  pieces are written as the graded nest draws that size (drafted from the size chart's
  measurements where it has the size, moved by the grade rules where the size range has it),
  and the document stays in its own size with its own measurements; DXF-AAMA's sample size is
  the size written unless `sampleSize` says otherwise. Tests: 2 more in `tests_grade.rs` (a
  ruled square's cutting SVG at 130, 140 and 150 mm pages, the AAMA sample size, the projector
  page; the skirt's chart sizes exported redrafted, measurements untouched). Not yet, and why:
  graded DXF-AAMA (the base size with a grade rule table, the `.RUL` file) needs the rule
  table's exact syntax, which no public source found so far gives (the standard itself is
  sold by ASTM; DH Patterns and Fit's overview was unreachable): the owner's call whether to
  buy ASTM D6673 or wait.
- 2026-10-21, **PF-6.4**: the **Grading** panel (Pattern and Window menus): the size range typed
  as "S, M, L" with its base size (Set Size Range; without a base chosen, the middle size), the
  selected piece's seam line points each with its step rule (x and y, mm a size; × removes it;
  a rule naming each size shows as "per size"), and Grade Nest / Clear Graded Nest (a size the
  pattern can't make is said in the status bar). Everything goes through `grade.*`. Tests:
  `crates/ui-egui/src/panels/tests_grading.rs` (drawn headless: the hint without a piece, a row
  per point, the nest buttons, the typed size range sent). Not yet: picking a point on the
  canvas, per-size rules edited in the panel, rules copied between pieces.
- 2026-10-21, **PF-6.5**: grade rules copied between pieces (`grade.copyRules {from, to?,
  mirror?}`; the Grading panel's "Copy Rules To", with Mirrored). Each ruled point's rule goes to
  the target's point nearest the same place in its bounds (a share of the width and height), so a
  front takes a back's rules though its points differ (`pf_core::grade::copy_rules`); mirrored
  (x), places flip left to right and x moves turn the other way, for the opposite half drawn the
  other way round. The target's own rules are replaced. Tests: 1 in `pf-core` (nearest points,
  mirroring, both rule kinds), 1 engine test (a front graded from a back grows to the left
  mirrored, to the right not; refusals) and 1 panel test. Not yet: points picked on the canvas,
  per-size rules edited in the panel.
- 2026-10-21, **PF-8.1**: **Ask About Patterns** (Help and Pattern menus, `ui.askAboutPatterns`)
  and **Foundry Account** (Help menu, `ui.account`), adapted from StitchFoundry's AI client, as
  the owner approved (`crates/sf-core/src/ai` and `crates/ui-egui/src/embroidery/{ask,account,
  prompt}.rs` in github.com/Stitchwright/StitchFoundry). A new `crates/pf-ai` (standalone, no
  network code) builds the Messages API requests and reads the replies: the conversation, Claude's
  two tools (`find_commands`, `run_command`), the Foundry cloud service's requests (sign in, the
  account, checkout) and AI requests sent through it paid in credits (only the body keys the
  service accepts; model `claude-opus-5-5`, at most 16,000 tokens). The app sends them through the
  host: `ureq` on a thread on the desktop (`apps/vectorcraft/src/ai_service.rs`, with the API key
  and session in private files next to the settings), the browser's fetch on the web. Claude can
  look at the pattern and change it through PatternFoundry's commands: reads run at once, changes
  wait for the user's Allow (each an undo step), files, exports, preferences, plug-ins and windows
  are out of reach. Measurements are personal data: the pattern summary sent with a question
  names the pieces, blocks, sizes and which measurements are set, never their values, and
  measure.info (the only command that returns them) waits for the user's Allow like a change.
  Tests: 9 in `pf-ai`, 7 in `crates/ui-egui/src/ai/tests.rs` (with a fake service: the summary
  without values, reads at once and the skirt drafted after Allow, a declined read, what Claude may
  run, no key or question, the web's fetch, a request paid with credits going to the service and
  its out-of-credits refusal). Not yet: a real request checked against the service, drafting a
  whole garment from a description (the "wrap dress" of PF-8's done-when: needs a wrap bodice in
  the design tools), fit advice from muslin photos.
- 2026-10-21, **PF-5.16**: **Make Wrap Front** (`design.wrapFront`, Pattern menu), for PF-8's
  wrap dress, as Ploen Patterns drafts a wrap blouse from the bodice block
  (https://ploenpatterns.com/blogs/news/how-to-draft-a-wrap-blouse-step-by-step-pattern-making-tutorial):
  the bust dart moved into the armhole first ("so there is room for the wrap"), the half front
  opened out across centre front ("because the front is asymmetric, don't place it on a
  center-front fold"; the block reflected across centre front, as Sewing with Numbers drafts a
  wrap skirt, https://sewingwithnumbers.substack.com/p/how-to-draft-a-wrap-skirt), and cut along
  the wrap line from the shoulder 4.5 cm from the neck point to the far side's waist; `curve`
  bows the wrap edge into the piece (the source's gaping fix). The fold is found (the longest
  straight edge on the outline's left or right side) or named; `from`/`to` move the line. A
  piece becomes cut as a mirrored pair. A dart the wrap line cuts away is reported (move the bust
  dart out of the shoulder first), and a dart of the far side whose point falls past the line is
  left out. Geometry: `pf_core::design::{opened_out, bow}`. Tests: 1 in `pf-core`, 2 engine tests
  (`tests_design.rs`: the bodice block's wrap front, the warning, the line 45 mm along the
  shoulder, nothing past it, cut 2 mirrored, one undo; a bowed edge and refusals). Not yet: the
  ties and the waistband, the back's matching changes (the source's lowered armhole, extended
  shoulder).
- 2026-10-22, **PF-5.17**: the wrap dress's other pieces. **Make Wrap Front** with `edge:
  "straight"` makes a wrap skirt's front: the skirt block's front opened out across centre front
  and cut by an edge parallel to it, `overlap` past it, from the waist to the hem (Sewing with
  Numbers marks where "top wrap ends" on the waistline,
  https://sewingwithnumbers.substack.com/p/how-to-draft-a-wrap-skirt; Rookie's overlap panel
  extends past centre front, https://www.rookiemag.com/2015/06/diy-wrap-skirt/); without
  `overlap`, the whole front (to match a wrap bodice that ends at the far side's waist). An edge
  that runs down a dart's opening is warned about. **Make Band** (`design.band`) draws a straight
  band to make a piece: a waistband (Ploen Patterns: about 8 cm wide, 1.5 to 2 times the waist),
  ties (Rookie: 3.5 × 25 in), cuffs, with a dashed fold line along its middle. Tests: 1 more in
  `tests_design.rs`. So a wrap dress is: the bodice back; the front's bust dart rotated into the
  armhole, then Make Wrap Front; the skirt back; the skirt front with Make Wrap Front (straight);
  a waistband and ties with Make Band.
- 2026-10-22, **PF-8.2**: **Check My Muslin** (Help and Pattern menus, `ui.checkMuslin`), PF-8's
  fit advice, adapted from StitchFoundry's Check My Sew-out as the owner approved
  (`crates/sf-core/src/ai/sewout.rs`, `crates/ui-egui/src/embroidery/sewout.rs`). Up to six photos
  of a muslin tried on (turned upright, made no larger than Claude looks at, sent as JPEG), the
  pattern's summary when it is open (pieces, blocks, sizes and which measurements are set, never
  their values) and a few answers (the muslin's fabric, the final fabric, how it feels, notes) go
  to Claude with fitting instructions (`pf_ai::muslin`): read the drag lines and folds, say where,
  give each problem's cause and the pattern alteration with how much, talk about the garment
  never the body's looks, never guess measurements from the photos. The answer, and the
  alterations it offers (each after the user's Allow; reading the measurements too), are in the
  Ask About Patterns window in its muslin mode, and questions after it go with the same
  instructions. The photos show the user: they go with the check only, are let go of once sent or
  when the window closes, and nothing logs them. Tests: 2 in `pf-ai` (the first message, the
  request) and 2 in `crates/ui-egui/src/ai/tests.rs` (a photo made smaller, a file that isn't one
  refused, the check's message with no measured value, the alteration made after Allow, the
  follow-up's instructions; a check without a photo). Not yet: a real request checked against
  the service, the pattern drawn as a picture to compare with the photos.
- 2026-10-22, **PF-9.1**: marker making, **Pattern › Make Marker…** (`marker.make`). A marker is
  "a diagram of the precise arrangement of pattern pieces" for a cut, its efficiency the share of
  the fabric the pieces use
  (https://www.onlineclothingstudy.com/2011/11/marker-planning-and-marker-making.html). Every copy
  of a fabric's pieces (the cut count; a piece cut in mirror-image pairs gives half its copies
  flipped) is laid on the fabric's width with its grainline along the length, parallel to the
  selvedge (https://www.threadsmagazine.com/2018/02/22/fabric-grain-explained), either way up, or
  all one way for a nap or a one-way print (`oneWay`,
  https://www.threadsmagazine.com/2020/06/11/how-to-lay-out-patterns-on-napped-fabrics), `gap`
  apart (2 mm), optionally in one `size` of the range. The laying (`pf_core::marker::lay`) is a
  bottom-left fill (Hopper & Turton, EJOR 128, 2001): largest piece first, each where its end
  comes furthest along, overlap tested on a grid (`precision`, 5 mm) where each piece covers every
  cell its outline grown by half the gap touches, so pieces never overlap. The marker is drawn on
  the layer "Marker <fabric>" (made again it replaces it): the fabric's outline, a caption (width,
  length, % used), and each copy's cutting line, seam line, grainline and notches as plain art
  with its name. Pieces too wide for the fabric are listed. The bodice, skirt and sleeve blocks,
  two of each, take 1.37 m of 150 cm fabric at 68% (precision 3 mm). Tests: 3 in `pf-core` (rectangles fill the
  width then the length, concave pieces never overlap and stay on the fabric, one-way and too
  wide), 2 in `tests_marker.rs` (copies, mirror images, true-scale fabric, the lining's own
  marker, replaced and undone; the blocks on two metres, untouched when the measurements change).
  Not yet: folded fabric (pieces cut on the fold laid against the fold), a small tilt off grain,
  plaid and stripe matching, pieces moved by hand with the length and efficiency kept up to date,
  a better search (trying orders, as nesting software does), the marker in DXF-AAMA.
- 2026-10-22, **PF-9.2**: pieces cut on the fold, and markers on folded fabric. A piece's fold
  (`piece.set {fold: edge | "auto" | null}`; the Pieces panel's **Cut on fold**, "auto" the
  longest straight edge) is a straight edge of its seam line: it takes no allowance and the label
  says "Cut 1 on fold". **Make Marker** lays such a piece opened out across the fold, whole, with
  one cutting line round it and the seam line and grainline on both halves
  (`pf_core::marker::opened_out`). With `folded` (the fabric folded lengthwise, selvedges
  together, the usual home layout: https://www.threadsmagazine.com/2009/06/30/the-fold-line) the
  marker is half the width with the fold dashed along its top edge: pieces on the fold lie against
  it, and every other piece cuts a mirror-image pair through both layers, so a piece cut 2 is laid
  once (an odd count is warned about: it cuts one more). Tests: 2 more in `pf-core` (a label on
  the fold; opening out, lying against the fold) and 1 in `tests_marker.rs` (no allowance on the
  fold, opened out single, against the fold when folded, pairs, the odd count, the fold taken
  off). Not yet: the fold in the projector PDF and in DXF-AAMA (its mirror line), a fold that is
  not straight.
- 2026-10-23, **PF-9.3**: markers checked after moving pieces by hand. A marker's copies are
  plain groups, so they can be moved with the selection tool (or `object.move`); **Pattern ›
  Check Marker** (`ui.checkMarker`, `marker.check {layer?}`) then draws the fabric's outline (and
  the fold, on folded fabric) to the last piece again, brings the caption's length and % used up
  to date in one undo step, and tells in a message which copies overlap (their cutting lines
  cross, or one lies inside another: `pf_core::marker::overlaps`) and which lie off the fabric
  (past a selvedge or before its start). The marker it checks is the one the selection is on, or
  the document's only one. Tests: 1 in `pf-core` (crossing, nested, touching, apart, a square in
  an L's notch), 1 in `tests_marker.rs` (as laid nothing wrong; a copy moved 200 mm along: the
  fabric and caption 200 mm longer; moved onto the other copy: an overlap; past the selvedge: off
  the fabric; not a marker's layer refused), 1 for the message. Not yet: plaid and stripe
  matching, the check made as pieces are dragged, turning a copy by hand off its grain warned
  about, the marker in DXF-AAMA.
- 2026-10-23, **PF-9.4**: plaid and check matching. **Make Marker…** takes `plaid` (the repeat, mm
  or a length with its unit, one value for a square check or `[along, across]`). Each copy is
  then moved only by whole repeats from where its piece is drawn (after turning its grain along
  the fabric), so pieces drawn level with each other (the bodice front and back at the same waist
  and underarm, a skirt's panels side by side) meet the same lines of the plaid on the fabric, the
  way plaids are matched at the seam lines (Seamwork, "10 tips for matching plaids and stripes",
  https://seamwork.com/articles/10-tips-for-matching-plaids-and-stripes: "drawing a straight line
  connecting these points across the front, back and sleeves is a quick way to make sure your
  plaids meet"; Apparel Resources on markers made "exactly on the repeat",
  https://apparelresources.com/technology-news/manufacturing-tech/taking-control-plaid-matching-spreading-cutting).
  No copy is turned on a plaid. A piece cut on the fold keeps against the fold and is matched
  along the fabric only. The caption names the plaid. `pf_core::marker::lay_matched` (the grid's
  cells grown by one so a copy off the grid by up to half a cell never overlaps). Tests: 1 in
  `pf-core` (every offset whole repeats both ways, none turned, none overlapping, on the fabric; a
  repeat of 0 refused), 1 in `tests_marker.rs` (two pieces drawn 400 mm apart lie whole 40 mm
  repeats apart; the caption; bad repeats refused). Not yet: match points picked on each piece
  (a sleeve matched to the bodice at its notches), stripes that run one way only, a mirror-image
  plaid's phase across a mirrored copy.
- 2026-10-22, **PF-7.4**: the fold in the exports. **DXF-AAMA** writes a piece cut on the fold as
  the half with its fold as the mirror line, a LINE on layer 6 ("Mirror line", "Symmetry of
  fold" in Patro's layer table,
  https://fabricesalvaire.github.io/Patro/resources/file-format/dxf-astm.html), and reads it back.
  The **projector PDF** shows such a piece opened out, whole, for fabric laid out single (the
  cutting line round the whole, the seam line, notches and inner lines on both halves, the fold
  drawn inside), or with `openFolds: false` the half with its fold dashed, to lay on the fold of
  folded fabric; its label says "on fold". The **SVG for cutting** keeps the half where it is
  drawn with its fold dashed in `draw` (opening it out could cut into a neighbouring piece) and
  cuts it whole with `openFolds: true`. Tests: 1 more each in `pf-aama` (the mirror line),
  `tests_aama.rs`, `tests_projector.rs` (opened out 220 mm, the half 110 mm, the 10 cm square)
  and `tests_svgcut.rs`. Not yet: drill holes, the other mirror-line uses in AAMA (symmetric
  pieces drawn as halves without being cut on the fold).
- 2026-10-22, **PF-4.4**: the armhole checked against the sleeve head. **Pattern › Check Sleeve
  Ease** (`ui.checkSleeveEase`; the query `design.sleeveEase`) measures the sleeve's crown from
  underarm to underarm and the back and front armholes from the shoulder points to the side seam,
  and tells the ease the crown takes in against the range for the sleeve's style: the University
  of Kentucky extension's figures, measured notch to notch, 1½ to 2 in for a classic set-in sleeve,
  ¾ to 1 in for a shorter casual cap, ½ to ¾ in for a shirt sleeve, less for knits (FCS2-807,
  https://publications.ca.uky.edu/sites/publications.ca.uky.edu/files/fcs2807.pdf), or `min` and
  `max` given. Too little: the sleeve may pull across the cap; too much: the cap may pucker. It
  gives the `sleeve_crown` that redrafts the sleeve block with the ease in the middle of the range.
  The drafted blocks are found by themselves (`pf_core::block::armhole`: where each part's
  armhole runs, while the part has its drafted anchors); a block changed since is named by hand
  (`{id, from, to}`, as Seam Length). It showed that the sleeve drafted from the top arm table
  alone has no ease against the bodice block with the same made-up measurements (crown 430.5 mm,
  armhole 432 mm): setting `sleeve_crown` as it says fixes that. Tests: 1 in `pf-core` (the
  anchors against the drafts), 1 in `tests_block.rs` (found and named seams agree, the
  `sleeve_crown` it gives brings the ease into range, styles, two sleeves), 1 in the UI (the
  message). Not yet: notches placed for the ease (front and back balance marks), two-piece
  sleeves.
- 2026-10-22, **PF-1.2**: edges and notches from the Pieces panel. Under the selected piece, a
  section (folded at first, its title the edge and notch counts) lists each edge of the seam line
  with its seam allowance (the piece's own until one is typed for the edge; a button back to the
  piece's; "On fold" for the fold, which takes none) and an Add Notch button (a notch in the
  edge's middle), then each notch, where it is (edge, % along) and a button to remove it; all
  through `piece.set {edges}`, `piece.addNotch` and `piece.removeNotch`. Tests: 1 in
  `panels/tests_pieces.rs` (the panel's first test). Not yet: notches and allowances picked on the
  canvas, a notch moved along its edge, the notch kind chosen in the panel.
- 2026-10-22, **PF-7.5**: the projector view in the app. **Pattern › Projector View**
  (`ui.projector`) draws the pattern pieces full screen at true size, white on black, for a
  projector over the cutting table: the cutting lines thick, the seam lines dashed, notches,
  grainlines, lines inside, names and how to cut, a 10 cm test square beside them; pieces cut on
  the fold opened out (or the half with its fold, for folded fabric). True size needs the
  projector calibrated once: the view shows a 10 cm square to measure on the mat and size with the
  arrows (Shift: faster), Enter keeps the scale as Preferences › Devices › Projector Scale (screen
  points per mm on the mat); until then it opens on the square. The arrows move the pattern 1 cm
  (Shift: 1 mm), M mirrors it (to cut with the wrong side up), C calibrates again, Escape leaves;
  the view takes these keys before the app's shortcuts, so they don't move the selection. The
  pieces come from the new query `piece.outlines` (each piece's lines in mm, y down; built as the
  DXF-AAMA export builds them). Tests: 1 in `tests_projector.rs` (the outlines true to scale,
  opened out and halved), 1 in the UI (calibrating, keeping the scale, moving and mirroring, full
  screen on and off). Not yet tried on a real projector; not yet: a grid over the whole mat to
  check against, a scale per projector, the view on the web.
- 2026-10-22, **PF-3.4**: personal measurement sets kept on this computer. **My Measurements** in
  the Measurements panel (`measure.saveSet`, `measure.sets`, `measure.loadSet`,
  `measure.deleteSet`) keeps the document's set (values and formulas) as a `.pfmeasure` file in a
  Measurements folder next to the preferences (the desktop app and `patternfoundry-cli` attach it;
  the web has none), to load into any other pattern (undoable; blocks and point formulas follow).
  Measurements are personal data: the sets stay in that folder, are saved never shared, are never
  logged (a file that can't be read is logged by its file name only), and are read back through
  the same checks `measure.set` makes, so a damaged or edited file can't bring in a value no set
  can hold. Tests: 1 in `tests_store.rs` (saved, listed without values, loaded into another
  session's pattern and undone, damaged files skipped, deleted), 1 in the panel. Not yet: a set
  chosen when drafting a block, sets synced between the user's computers (only if they ask).
- 2026-10-22, **PF-5.18**: dart shortening. **Pattern › Shorten Dart…** (`design.shortenDart`)
  moves a dart's point along its centre line toward its opening, 25 mm by default (a bust dart is
  ended about 1 in short of the bust point), the legs straight from the same ends so the seam it
  opens on stays as it is, as Made by Rae shows it ("a new point on the dart's center line one inch
  away from the original point", the legs redrawn with a straight edge:
  https://www.made-by-rae.com/blog/2015/07/bust-dart-adjustment-how-to-shorten-or-lengthen-a-dart);
  less than 0 lengthens it. The point is `apex` or the one selected point; a point whose legs open
  a right angle or wider, curved legs, and a shortening as long as the dart are refused. Tests: 1
  in `pf-core` (along the centre line, the legs' ends kept, lengthened back, refusals), 1 in
  `tests_design.rs` (the bodice block's shoulder bust dart ended 25 mm short of the bust point,
  the waist dart's point at the same bust point kept, one undo step). Not yet: curved dart legs, the
  new point dragged on the canvas.
- 2026-10-24, **PF-10 plan** (the 3D fit preview, in parts, all in Rust and wasm-safe, from public
  sources only): **PF-10.1** an avatar from the measurements (this entry); **PF-10.2** sewing
  lines: which edges of which pieces are sewn together (the walked seams of PF-5's truing, kept
  on the pieces), shown on the pattern; **PF-10.3** cloth simulation: each piece triangulated at
  its true size, placed round the avatar, its seams pulled together and draped by gravity
  (position-based dynamics: stretch and bend constraints, collision with the avatar's rings),
  a skirt first; **PF-10.4** the drape view: the draped garment drawn beside the 2D pattern with
  the existing 3D shading (`vectorcraft-three-d`), turned with the mouse, tight places shown in
  colour; **PF-10.5** glTF export of the avatar and the draped garment for game and 3D artists
  (done, below).
- 2026-10-24, **PF-10.1**: an avatar from the measurements. **Pattern › Draw Avatar**
  (`avatar.draw`) draws the body the document's measurements make, front and side at true scale,
  on the layer "Avatar" below the pattern (drawn again it replaces it); `avatar.exportObj`
  writes its mesh as a Wavefront OBJ file (metres, y up) for 3D software. The body
  (`pf_core::avatar`) is lofted through elliptic cross-sections at the neck, bust, waist, hips and
  crotch and each leg's thigh, knee, calf and ankle, each exactly as long round as its girth
  (Ramanujan's perimeter) and as deep as the U.S. Army's ANSUR II survey of 1,986 women finds the
  body there (public data,
  https://ph.health.mil/topics/workplacehealth/ergo/Pages/Anthropometric-Database.aspx; its
  means: the chest 0.918 as deep as broad, the waist 0.709, the hips 0.658; the leg's girths as
  shares of the hips; the heights of the landmarks, scaled by the nape to waist). It needs bust,
  waist, hip, nape_to_waist and waist_to_hip, and uses neck, crotch_depth, waist_to_knee and
  waist_to_ankle where the set has them. The survey's waist is at the navel, below a
  dressmaker's waist, so without a crotch depth the crotch is put the survey's 52 mm below the
  fullest part of the hips rather than measured from the waist. Measurements stay personal: the
  avatar is drawn and exported only when asked, and never logged. Tests: 3 in `pf-core` (each
  ring's girth to a hundred-thousandth and its depth; the heights in order, the survey's woman's
  waist at 980; given lengths used; a closed mesh facing out; the OBJ), 2 in `tests_avatar.rs`
  (the drawing's height and breadths at true scale, side by side, replaced, undone; the OBJ as
  text and as a file). Not yet: arms and a head, the bust's shape (an ellipse is flatter than a
  bust), a menu item for the OBJ export.
- 2026-10-24, **PF-10.2**: sewing lines, which seams are sewn together, for the 3D fit preview.
  `piece.sew {a: {id, from, to}, b: {id, from, to}, label?}` records that two runs of seam line
  are sewn together (each from one anchor forward to another, from the ends that meet) and gives
  their lengths and the difference; `piece.seams` lists every pair by label; `piece.unsew {label}`
  forgets one. Each piece keeps its own runs (`Piece.sewn`: the label and the run's anchors), so a
  run follows its seam line when anchors are added or removed (PF-1.4), and the two halves of a
  pair find each other by label (A, B… then A2…). The Pieces panel lists the selected piece's
  sewn seams ("A: sewn to Back, 20 mm shorter") with a button to unsew. Tests: 1 in
  `tests_piece.rs` (a 100 mm edge sewn to a 120 mm one: -20 mm; the next label; an anchor added
  inside a run; the same run twice, an anchor past the end and a label in use refused; unsewn),
  1 in `tests_pieces.rs` (the panel's list and its button). Not yet: picking the two runs on the
  canvas, the labels drawn on the pieces, a block's seams sewn when it is drafted, grade rules
  (which also name anchors) kept on their points when anchors are added.
- 2026-10-24, **PF-10.3** (first part): the cloth simulation's core, `pf_core::cloth`. A piece's
  outline becomes a panel: a particle at every grid point `step` apart inside it, linked to its
  neighbours (stretch), its cells' diagonals (shear) and its neighbours but one (bending), each
  link as long as on the pattern. Panels are placed in 3D by any function of the pattern's
  points, sewn particle to particle, and draped by position-based dynamics (Müller, Heidelberger,
  Hennix and Ratcliff, "Position Based Dynamics", VRIPHYS 2006,
  https://matthias-research.github.io/pages/publications/posBasedDyn.pdf): each step moves the
  particles by their velocity (a little damped) and gravity, then corrects them a few times over:
  the links to their lengths (light woven cloth by default: stretch 1, shear 0.6, bend 0.05),
  sewn particles together, and every particle out of the body. Held particles keep the others
  within their distance on the pattern (long-range attachments: Kim, Chentanez and Müller, SCA
  2012, https://matthias-research.github.io/pages/publications/sca2012cloth.pdf), without which
  cloth hanging from two corners stretched 76% at ten iterations a step. The avatar is a
  collider: a point inside the torso or a leg, as the rings are at its height, goes out to the
  surface 2 mm clear. Tests: 4 in `pf-core` (a square and an L made panels; cloth held at two
  corners sags 250 mm and stretches under 6%; two panels sewn into a tube hang round a post,
  outside it, their seams closed to 3 mm; the avatar pushing points out of its waist and a leg).
  Next: the document's pieces draped on the avatar (a skirt first), then the drape view.
- 2026-10-24, **PF-10.3** (second part): pieces draped on the avatar, a skirt first.
  `drape.make {ids?, precision?: 15 mm, steps?: 240, path?}` wraps the pieces sewn together
  (`piece.sew`) round the avatar at the waist, in the order they are sewn and the first at the
  front, each row of cloth round a circle as long round as the pieces are wide together at that
  height (the outline's own width there, so the sewn sides start together and nothing starts
  stretched); pairs each seam's two runs by height on the pattern (pieces drawn upright), and lets
  the cloth fall four seconds as light woven cloth, four substeps a sixtieth of a second with 40
  rounds of corrections each. The collider now pushes cloth out along the body's surface normal
  (it leans up where the body widens downward, and holds a skirt up) and cloth that touches the
  body keeps a fifth of its sliding (friction). It reports where the cloth hangs (its top and hem
  against the avatar's waist), the worst stretch and the widest seam gap, and writes the draped
  cloth as an OBJ file. The debugging is in `docs/debug-logs/2026-10-24-skirt-falls-off.md`.
  Tests: 2 in `tests_drape.rs` (an A-line skirt 720 round at the waist and 1013 at the hips on a
  body of 680 and 960 settles within 60 mm below the waist and hangs its 600 mm; the OBJ; no
  measurements, no sewn pieces and a bad precision refused), the collider's test (onto the surface,
  pushed up on the slope from the waist to the hips). Not yet: particles on the outline itself
  (a seam pulls particles a grid step inside a slanted side together: a 12 mm gap, one link 41%
  stretched there), a drape's speed (about an eighth of a second a step at precision 15 in a test build: half a minute for the default four seconds),
  bodices and sleeves (placed round the torso and arms), a menu item (with the drape view).
- 2026-10-24, **PF-10.3** (third part): the cloth reaches its seam lines. A panel's particles at
  the grid's edge are moved onto the outline (the nearest point of it, where that is within a
  step, and a quarter step from any other moved there, as at a corner), the links taking their
  lengths from where the particles are; the particle nearest a point is found by where the
  particles are. A seam now sews the pieces' edges, not points a step inside them: the A-line
  skirt's widest seam gap went from 12 mm to 2 mm and its worst stretch from 41% to 7%. Tests: 1
  more in `pf-core` (a trapezoid's edge particles on its outline, none outside it, the particle
  nearest a point of the slanted side on it), the skirt's test bounds tightened (seams within
  5 mm, stretch under 15%; it hangs 580 to 680 mm, a few per cent long under its weight).
- 2026-10-30, **PF-10.19**: trousers sit at the waist. Draped, the trouser block hung 63 mm
  below the avatar's waist, its hems on the floor (PF-10.10 and PF-10.12 put it down to the
  crotch seam); the investigation (`docs/debug-logs/2026-10-30-trousers-sit-low.md`) ruled out
  sliding, soft cloth, the waistband, the floor and gravity (without gravity they sat as low) and
  found the thighs: each reached 12 mm past the middle, so just below the torso they overlapped,
  and the crotch seam, sewn shut there, was pushed down out of them to where they part, 65 mm
  below the body's crotch, taking the trousers with it. Now the thighs sit side by side across the
  hips, each half the hips' breadth less a 5 mm gap, as round as the survey's thigh (so deeper
  than broad). And the avatar's crotch has the survey's length: below the hips the front draws in
  under the belly and the back under the buttocks to the crotch, until the way from the waist in
  front down through the crotch and up to the waist behind is ANSUR II's share of the hips (the
  crotch length from the navel over the buttock girth, 0.609 for women, 0.611 for men; children
  the women's), longer on each side by how much further the body's waist is above its crotch
  than the survey's navel above its crotch, 55% of it behind (women; men 57%) as near as the shape
  allows; before, the hips' full depth was carried down to the crotch (755 mm on the test body,
  the survey's 700). Both are needed: the thighs apart with the old crotch, the trousers reached
  the waist but strained to 187%, 94% stretch, a seam 44 mm open. On the test bodies the women's
  trouser block's top is now 1.5 mm above the waist (was 66 below), the women's block on a man at
  his waist (was about 150 below), the men's block 40 mm below (its waist is 95 cm for an 86 cm
  waist, under a 4 cm waistband; it was on the hips). Tests: 1 in `pf-core` (the crotch as long as
  the survey's share for a woman, a man and a deep crotch, drawn in from the hips, the back the
  longer way; the thighs side by side, 5 mm apart, as round as the survey's), the trouser tests'
  bounds tightened (the women's within 30 mm of the waist, the man's within 40, the men's block
  within 80).
- 2026-10-29, **PF-10.18**: the draped garment goes to 3D software in its fabric. Export Drape
  as glTF… (`drape.make format: "glb"`) writes the cloth in the document's fabric photos where
  there are any (`colour: "fit"` keeps the fit colours): a part for each photo ("Drape: Main"),
  its picture the drape view's swatch (evened and made seamless if the photo is) as a PNG inside the
  file, each side scaled to the nearest power of two (up to 2048) so viewers on WebGL 1 can repeat
  it, and each point's place on it (TEXCOORD_0) its place on the fabric along the grain over the
  photo's size, so the photo repeats at true size through a repeating sampler; pieces whose photo
  can't be read in a grey part of their own. Written from the glTF 2.0 specification
  (<https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html>: §3.7.2.1 TEXCOORD_0, §3.8
  textures, images and samplers, §3.9.2 the base colour texture), the file checked with the
  Khronos glTF Validator (0 errors, 0 warnings) and seen in three.js. Tests: 1 in `pf-core` (a
  printed part's TEXCOORD_0, its PNG in a buffer view without a target, a repeating sampler, the
  base colour texture; none on a plain part; a texture whose places aren't one to a point
  refused), 1 in the engine (the striped skirt's file: the part "Drape: Main", its 80 × 30 photo
  as a 64 × 32 PNG, its places 8.5 photos across the two 1700 mm of pattern; `colour: "fit"`;
  another colour refused).
- 2026-10-28, **PF-10.17**: a fabric photo taken at home can be evened out and made to repeat.
  Use as Fabric… now offers *even* (even out the light: each pixel's brightness divided by the
  photo's brightness averaged over a quarter of the photo round it, times the photo's mean, at
  most 4× either way, so a window's slow fall-off across the photo goes and the print stays:
  flat-field correction, <https://en.wikipedia.org/wiki/Flat-field_correction>) and *seamless*
  (cross-fade the photo into itself moved half its width and height, fully the moved one at the
  edges, so each edge meets the next copy through the photo's own middle: the offset-and-blend
  repeat; <https://en.wikipedia.org/wiki/Texture_synthesis> describes the tiling it is for), both
  on in the menu's dialog and off by default for the command (`fabric.set even?, seamless?`),
  kept when not given again, saved with the photo. Applied to the drape view's swatch (after it
  is averaged down), never to the placed image itself. Tests: 3 in `pf-core` (a check print lit
  twice as bright on one side evened to under a third of the change, its squares still showing;
  a black-to-white ramp's edge jump of over 200 brought to no more than a step inside it, across
  and down; a broken swatch left alone), 1 in the engine (the options off, set, kept, refused when
  not true or false, saved, and the drape drawn in the photo).
- 2026-10-28, **PF-10.16**: see the draped garment in your own fabric. Photograph the fabric
  flat, the selvedge (the lengthwise grain) upright, and place the photo; select it and choose
  Pattern › Use as Fabric… (`fabric.set`) with how wide the photo is on the fabric (by default as
  wide as the photo is placed, so a photo scaled to a ruler in it needs nothing typed), the
  fabric's name if pieces name theirs (Piece Options' fabric: Main, Lining, Contrast…), and the
  quarter turns that bring the selvedge upright if the photo was taken sideways. Drape on Avatar then
  draws a third view, the front again with each piece in the photo of its fabric (else the first
  photo), the photo repeating across the piece along its grainline, at true size (1 mm a pixel),
  the photo averaged down to that so a fine weave doesn't sparkle; a mirror-image pair shows the
  print the right way round on both sides. Stop Using as Fabric (`fabric.clear`) unmarks it;
  `fabric.list` lists them. The workflow follows the public descriptions of fabric textures in 3D
  garment software: a texture entered at its real size
  (<https://support.clo3d.com/hc/en-us/articles/115013661707-Can-I-enter-the-texture-in-real-size>)
  from a photo or scan of the fabric laid flat, its grain aligned with the photo
  (<https://support.clo3d.com/hc/en-us/articles/115012228067>); making the photo repeat
  seamlessly and evening its lighting are left to a photo editor for now. Tests: 1 in `pf-core`
  (a striped swatch shows 10 mm stripes at true size, unprinted triangles keep their colours), 1 in
  the engine (the photo's width as placed and as given, bad widths and turns refused, a
  non-image refused, the draped skirt's front in 10 mm stripes 8–12 px wide down its middle,
  cleared and undone).
- 2026-10-28, **PF-10.15**: the avatar has a head and hands, and looks round. The head stands above
  the base of the neck as high as the figure's survey's head stands above its cervicale for the
  body's rise (ANSUR II: the women's 0.560 of the cervicale's height above the waist, the men's
  0.518; the children's survey 0.60 to 0.69 by age), as round as its share of the chest (0.593,
  0.543; a child's 0.97 at 2–4 down to 0.63 at 16–18), 1.28 to 1.31 times as long front to back as
  broad; lofted through the throat, the jaw, the temples, its roundest and the crown, a little
  forward of the neck (where those rings sit and how round each is: chosen). Each hand goes on
  along the arm's line from the wrist, as long as its share of the arm (0.333, 0.326, a child's
  0.29 to 0.32), flat, through the palm, the fingers and their tips. The drape view lights the
  avatar and the cloth as curved surfaces (each point's normal the average of its triangles',
  blended across each triangle) instead of flat facets. The cloth collides with the body from the
  base of the neck down: the head and the neck above its base are drawn and exported (glTF, OBJ)
  but not in the cloth's way (a garment starts round the body as if on already; pushed out of the
  head a bodice's neckline tore, stretch 15% → 136%). Tests: 2 in `pf-core` (the head and hands by
  the survey, a child's head the larger share; smooth shading without a step at an edge), the
  avatar's drawn and exported heights to the top of the head.
- 2026-10-27, **PF-4.15**: collars for the men's shirt. Draft Collar, Draft Stand Collar and Draft
  Shirt Collar (`design.collar`, `design.standCollar`, `design.shirtCollar`) know the men's shirt
  block's necklines and shoulder points, as they know the bodice block's: select its back and front
  and draft. On the test man (neck 400) the shirt's neckline is 396 mm round, the collar's neck
  edge as long. Children's blocks are not drafted yet: the public children's drafts found (a
  self-drafted "easy" block in inches with a fixed 3-inch front neck, a sheet missing its armhole's
  points, an "arc"-measurement draft needing nine measurements PatternFoundry doesn't take and
  leaving its dart's point to its drawing) are not good enough to fit; a child figure drapes the
  women's and men's blocks drafted from its measurements meanwhile. Tests: 1 in `tests_design.rs`
  (a shirt collar from the men's shirt block selected).
- 2026-10-27, **PF-4.14**: a men's shirt sleeve. **Pattern › Draft Men's Shirt Sleeve Block**
  (`block.mensShirtSleeve`, in all 23 languages) drafts the sleeve on the men's shirt block's
  armhole, as the same author publishes it
  (<https://opensourcestitches.wordpress.com/2011/02/18/pattern-drafting-101-the-mens-shirt-sleeve-block/>):
  the crown a third of the armhole high above the armhole line; its top, middle and underarm points
  each pivoted from the last by the armhole's own lengths (the straight shoulder-to-armhole
  distances and the curves' lengths) and 7.5 mm to 1.5 cm more; the sleeve straight down from the
  crown's top, 5 cm in each side at the hem, the hem dipping 1 cm at the back's placket (a 10 cm
  slit, marked). Measurements: the shirt block's and sleeve_length; option: the cuff's depth (6 cm,
  chosen: the page gives no length), off the sleeve's length. Chosen where the page is silent: the
  crown's curves (within its distances of the lines), its "x" (never placed: the crown's front curve
  taken over to b in one), the length. On the test man (chest 1000, armhole depth 240) the crown
  is 2.4 cm longer than the front armhole and 2.7 cm than the back's (the source's ease, generous for
  a shirt); set in, the shirt drapes with a sleeve down each arm, the armpit's tightest place 42%
  (a women's bodice and sleeve: about 250%), seams within 5 mm. Tests: 1 in `pf-core` (the crown's
  height and its ease over the armhole, the hem, refusals), 1 in `tests_drape.rs` (the shirt with
  sleeves on the male figure, at the app's own precision: at 25 mm the coarse cloth can't follow the
  armpit). Not yet: the collar and cuffs (BurdaStyle's collar post is not public text here).
- 2026-10-27, **PF-4.13**: a men's shirt block. **Pattern › Draft Men's Shirt Block**
  (`block.mensShirt`, in all 23 languages) drafts the men's shirt block from the metric steps
  published by Open Source Stitches
  (<https://opensourcestitches.wordpress.com/2011/02/17/pattern-drafting-101-the-mens-shirt-block/>,
  after Gedwood's BurdaStyle tutorial, "partly based on" Aldrich's *Metric Pattern Cutting for
  Menswear*; the construction rules implemented, no text or drawing copied): a rectangle half the
  chest and 8 cm wide (16 cm of ease round the chest), the armhole line the armhole depth and
  2.5 cm down, the back's width half the half back and 2.5 cm, the side seam between it and a
  third of the chest and 1.5 cm; the back neck a fifth of the neck less 5 mm across and 2 cm up,
  the front neck a fifth deep; the shoulders from the page's points, the front's 5 mm longer;
  straight sides, no darts. Measurements: bust (round the chest), neck, back_width (the half back
  half of it), armscye_depth, nape_to_waist; option: its length down the centre back (78 cm,
  chosen: the page leaves it to the maker). Chosen where the page is silent: the curves (the
  armhole within the page's 3 cm of the back's corner and 1.75 cm of the front's). The drape now
  hangs a top longer than the body is from the neck to the waist from the base of the neck (it
  put every top's hem at the waist): on the male figure the shirt hangs from the shoulders to
  about 32 cm below the waist, seams within 4 mm, stretch 7%; the women's bodice is placed as
  before. Tests: 1 in `pf-core` (by the page's numbers, the armhole near its corners, a shirt too
  short refused), 1 in `tests_drape.rs` (drafted, cut on the folds, sewn and draped). Not yet: the
  men's shirt sleeve (the page's next post), a collar and cuffs.
- 2026-10-26, **PF-4.12**: a men's trouser block. **Pattern › Draft Men's Trouser Block**
  (`block.mensTrousers`, in all 23 languages) drafts the traditional British men's trouser block
  from the metric steps published by Textile Blog
  (<https://www.textileblog.com/mens-pants-pattern-drafting-from-measurement/>; the construction
  rules implemented, no text or drawing copied): from a crease line, the crotch line the body rise
  and 1 cm less the waistband down, the seat line a quarter of the rise above it; the front's fork
  a twelfth of the seat and 1.5 cm and then a sixteenth and 5 mm out, a quarter seat and 2 cm
  across the seat line, a quarter waist and 2.5 cm at the top; the back on the same lines, a
  quarter seat and 3 cm and a quarter waist and 4.5 cm with a 2.5 × 12 cm dart, its centre back
  1 cm up, its fork further out and 5 mm lower, its hem and knee 2 cm wider each side. Measurements:
  waist, hip (round the seat), crotch_depth (the body rise), waist_to_ankle (the inside leg is it
  less the crotch depth), trouser_hem (the bottom width half of it). Options: the waistband's depth
  (4 cm; the block's top is its lower edge), an easy fit along the back crotch (the source's
  3.5 cm, none by default), the back dart. Chosen where the page is silent: the knee points it
  names but never places, on its knee line 1 cm wider each side than the hem (a straight leg's
  "knee and hem nearly equal", Sewing for a Living); the curves. The back is laid out beside the
  front turned over (its fork on the right), as the women's block's is: drawn on the front's lines
  its copy as drawn was the other leg's, and the drape sewed one leg's front to the other's back
  (a seam spiralling round the leg). On the male figure (seat 1020, waist 860) the side seams and
  inside legs come out as long front and back (within 2.4 mm), and the trousers drape down both
  legs, seams within 4 mm, stretch 11%, on the hips (their top 95 cm round, made for a separate
  waistband). Tests: 1 in `pf-core` (the source's example by its numbers; turned over; refusals),
  1 in `tests_drape.rs` (drafted, sewn, the runs as long, draped on the male figure). Not yet:
  men's bodice and sleeve blocks, children's blocks.
- 2026-10-26, **PF-10.14**: feet and a floor. Each of the avatar's legs ends in a foot: under the
  ankle a ring as long as the figure's foot and as broad (ANSUR II: the women's foot 246.3 long
  for an ankle bone 62.7 up, 3.93 times, 0.376 as broad; the men's 3.72; the children's survey's
  3.87 to 4.11 by age, 0.39 to 0.41 as broad), a quarter of it behind the ankle, at a fifth of the
  ankle bone's height, the leg's tube closed under it; and the body pushes the cloth up out of the
  floor. The drape view, the side silhouette and the glTF and OBJ meshes show the feet. A hem long
  enough rests on the feet and the floor: the man's trousers (PF-10.12's test body) now sit 54 mm
  higher (147 below the waist), their hems folded over the feet; the women's trouser block, which
  ends at the ankle bone, hardly moves (1037). Tests: 2 in `pf-core` (each leg's foot as long and
  broad as the survey's, forward of the ankle; the floor pushes a point up onto it), the avatar's
  drawn height and side (down to and forward to the feet), the man's trousers' seams within 20 mm
  (their folds over the feet).
- 2026-10-26, **PF-10.13**: a child figure. The measurement set's figure can be **Child**
  (`measure.set {figure: "child"}`, the Measurements panel, in all 23 languages). Its
  proportions come from the U.S. Consumer Product Safety Commission's survey of 3,900 children
  from 2 to 18 (Snyder et al., *Anthropometry of Infants, Children, and Youths to Age 18 for
  Product Safety Design*, University of Michigan, 1977; public domain, its individual records
  published by NIST at <https://math.nist.gov/~SRessler/anthrokids/>): the means of eight age
  bands (2–4 … 16–18), computed from the records, the band chosen by the chest's girth (512 mm at
  2–4 to 879 at 16–18) and between two bands in proportion, so there is no age to enter. What the
  survey doesn't measure is taken from the landmarks it does by ANSUR II's women's proportions
  between them (the cervicale from the suprasternale, the chest from the armpit, the shoulder
  point between the armpit and the cervicale, the ankle bone from the stature); the depths from
  each girth and its breadth (a child's chest is about as deep as broad, the waist 0.86 at 2 to
  0.71 at 18). No bust. The blocks are still the women's. Tests: 1 in `pf-core` (each band's own
  means make an avatar, its landmarks in order; between bands in proportion), the engine's
  figure test (a six-year-old's measurements make a child under 1.1 m). Not yet: children's
  blocks, a head.
- 2026-10-26, **PF-10.12**: the cloth grips the body. Where the body pushes the cloth out, its
  move across the body's surface is undone while it is less than 0.5 times how far it was pushed
  (static friction), and cut by 0.4 times that when it slides (Macklin, Müller, Chentanez and Kim,
  "Unified Particle Physics for Real-Time Applications", SIGGRAPH 2014, section 6.1; 0.5 is dry
  skin's "typically around 0.5" in Derler and Gerhardt's review of skin friction, Tribology
  Letters 45, 2012). Before, the cloth only slid slower where it touched and never stuck: trousers
  for a man (a waist 790, hips 1000) crept down off his hips to the floor. Only skirts and
  trousers grip (they hang from the waist); bodices and sleeves hang from the shoulders, and
  gripping pinched them in the armpit (the tightest place 253% → 794%); nothing grips while a
  garment is being sewn up round the body, so its pieces slide into place. On the test bodies:
  the trouser block now sits 66 mm below the waist (was 90); the man's trousers stay on, about
  200 mm low, on his hips (the waistband 22% stretched); a skirt with 40 mm of ease sits about
  at the waist, its loose rim up to 11 mm above it. Tests: 1 in `tests_drape.rs` (the man's
  trousers stay on), the skirt's bound loosened from 10 to 15 mm above the waist.
- 2026-10-26, **PF-10.11**: a male figure. A measurement set has a figure, female (the default)
  or male (`measure.set {figure}`, **Figure** in the Measurements panel, kept in the document and
  in My Measurements; in all 23 languages). It chooses whose proportions the avatar fills in what
  the measurements don't give: ANSUR II's 1,986 women's (as before) or its 4,082 men's, their
  means computed from the same public data the same way (the women's reproduce the constants
  already there): the chest 0.877 as deep as broad (women 0.918), the waist 0.728, the hips
  0.711; the shoulders 0.393 of the chest across and 0.503 as deep; the thigh 0.613 of the hips,
  the knee 0.401, the calf 0.385, the ankle 0.225; the neck 0.410 of the chest, the upper arm
  0.338, the forearm 0.866 and the wrist 0.491 of it; all the landmark heights (the waist at the
  navel 1056.5 above the floor, the cervicale 1517.3, the crotch 845.9…), the arm 593.1 long, the
  elbow 335.2 down it. A male figure has no bust standing forward and no underbust ring: its
  chest (the `bust` measurement, taken round the chest) is one elliptic ring. The blocks are
  still the women's (Aldrich's bodice, a women's trouser block: a man's waist close to his hips is
  refused by the trouser block's "the waist is wider than the hips with ease"). Tests: 1 in
  `pf-core` (no underbust, the chest's depth, the shoulders from the men's share, the mesh), 1 in
  `tests_avatar.rs` (the figure chosen on the measurements makes the side view shallower, kept,
  undone, a wrong figure refused). Not yet: men's blocks.
- 2026-10-26, **PF-10.10**: trousers on the avatar. **Pattern › Drape on Avatar** drapes the
  trouser block (or pieces named trousers, pants or shorts): a front and a back, each cut as a
  mirror-image pair, sewn at the side seams and inseams. The centre front and centre back are a
  piece's run sewn to the same run of its mirror copy: `piece.sew` now takes the same run as `a`
  and `b` on a piece cut 2 mirrored (a centre seam), and refuses it on any other. Each leg's front
  and back start round that leg below the crotch (the inseam inside), the four round the hips
  above, blended from the hip line down to the crotch; all from the hems up, so the back's rise
  stands above the waist. The cloth now has at least forty rounds of corrections a substep at any
  precision: at 25 mm it had 26, and the trousers (as heavy as two skirts) slid down the legs.
  On the test body (hip 960, crotch depth 260) they hang down both legs, seams within 10 mm,
  stretch 33%, but sit about 85 mm below the waist with the crotch reading tight: the block's
  crotch seam is 657 mm (front 296, back 361) and the avatar's, waist to waist between the legs,
  about 755 (ANSUR II's women's mean crotch length from the navel, 621 mm, fits the avatar's).
  A trouser crotch is cut shorter than the body's as woven cloth gives on the bias there; the
  simulated cloth doesn't give enough. Tests: 2 in `tests_drape.rs` (the trousers down both legs,
  their back darts sewn, cloth round each knee and none between; a centre seam refused unless
  the piece is a mirror-image pair). Not yet: cloth that gives on the bias, the hem resting on a
  floor or the feet.
- 2026-10-25, **PF-10.9**: the armpit. The avatar's arms now swing out from a shoulder joint
  (the head of the upper arm, a fifth of the deltoid's radius in from the shoulder point and one
  radius below it), not from the armpit: the top of the arm is the deltoid (1.12 times the upper
  arm's girth, chosen), rounded over with a cap ring, so the sleeve's crown rests on it; before,
  the arm's top was buried in the chest and the arm had no top for the cloth to rest on. A
  tilted arm's level cross-sections are longer by 1/cos of its angle, so the upper arm is round
  as measured square to the arm. And a bodice with sleeves is put on as a dresser would: the
  bodice first (half the time, the sleeves held and not yet sewn in), then the sleeves sewn in
  with the rest held (a sixth of the time, without gravity, the seams pulling harder and harder),
  then everything let go. On the test body a bodice with sleeves went from 201% worst stretch and
  557% at its tightest (the armpit) to 139% and 261%; the bodice alone from 23% / 31% to 14% /
  16%. Limits: the crown's seam still opens up to about 45 mm over the deltoid, and the bodice
  hangs about 20 mm higher than before (the deltoid holds it up). Tried and left out: the joint
  further in or the arm at 20° (the sleeves dragged the bodice down the arms), sewing the
  sleeves in all at once with the new arms (596% at the armpit), moving each crown stitch
  straight onto its armhole before sewing (one sleeve was pulled through its arm). Tests: the
  avatar's arm test (the cap and deltoid rings, the upper arm round as measured square to the
  arm), the sleeve drape's tightest place under 400% (338% at the test's coarser cloth).
- 2026-10-25, **PF-10.8**: the avatar's bust. Its cross-sections can stand forward of the body's
  middle, and the bust does: under it a ring as round as the new `underbust` measurement (else a
  B cup's, the bust 150 mm more round: EN 13402-3 sizes cups by the bust less the underbust, B
  14 to 16 cm), 75 mm below the bust point (chosen: the survey doesn't measure it), 0.8 as deep as
  broad (chosen, between the chest's and the waist's); the back flat from there up to the bust,
  the bust's extra girth to the front. The drape view, glTF and OBJ meshes and the side
  silhouette show it. A bodice block with its darts sewn now has a bust to shape: its stretch on
  the test body fell from 34% to 23%, its tightest place from 33% to 31%. Tried and left out: the
  body's parts joined with rounded creases (a smooth minimum of their depths, Inigo Quilez's
  polynomial one): it eased the armpit (a sleeve's tightest place 638% → 145%) but let the
  armhole slide down the angled arm, the bodice off the shoulders; with the arms less angled
  the sleeves slid off too. Tests: 1 in `pf-core` (`avatar`: the bust's back in line with the
  ribcage's, its front forward, the underbust measured or a B cup's), the side view's depth in
  `tests_avatar.rs`. The armpit: PF-10.9.
- 2026-10-24, **PF-10.7**: the drape in the background. **Pattern › Drape on Avatar** no longer
  holds the app while the cloth falls (some seconds; more for a bodice with sleeves): the drape is
  set up from the document at once, falls on the background worker that Background Save and
  Export use, and the export progress dialog shows it ("Draping on Avatar…", "Step n of 240", a
  bar, Cancel and Continue in Background; the status bar says so too); editing goes on meanwhile.
  When it is done the drape view is drawn in the document it was asked for, brought to the front
  if another one is (one undo step, as before); cancelled, nothing is drawn and the status bar
  says the drape was cancelled. `drape.make {background: true}` asks for this (the menu item does);
  without it (an agent's call, the command line) the drape runs at once and returns its measures,
  as before; on the web, which has no worker, it runs at once too. The engine's drape is split for
  it into `drape::setup` (reads the document), `Setup::run` (the fall, reporting each step and
  stopping when asked) and `drape::finish` (the measures, files and drawing); the progress dialog
  counts a job's steps in its own words (`background::run_with_steps`). Test: 1 in
  `tests_background.rs` (the call comes back at once, the job's label, steps and progress; drawn
  in its own document with another in front; cancelled, nothing drawn; without `background`, at
  once).
- 2026-10-24, **PF-10.6**: bodices and sleeves on the avatar. `drape.make` (Pattern › Drape on
  Avatar) now puts each piece where it belongs: a bodice (a bodice block's part, or a piece named
  so) from the waist up over the shoulders, a sleeve (a sleeve block's, or named so) round the arm
  of the armhole it is sewn into, anything else (a skirt) from the waist down. Pieces are one side
  of the body, as blocks are drawn: one cut on the fold is opened out whole, one cut as a
  mirror-image pair (cut 2, mirror) is both sides, every seam sewn on each side where its pieces
  are; darts (two legs about as long meeting at a sharp point, the mouth open) are found and sewn
  shut, and seams are sewn with their darts closed (the result counts the darts). Seams are sewn
  particle to particle along the two runs, in order, each partner once (sewing the particles
  nearest points along the runs had sewn one to two a step apart, and a bodice's side seams, which
  slant differently front and back, stayed 15 mm open). The first third of the time the garment is
  sewn up without gravity, its seams pulling harder and harder, then it falls into place. The
  avatar has shoulders (the shoulder line as broad as two shoulder lengths and the neck, 60.6 mm
  below the nape, as deep as ANSUR II's shoulders) and arms (the upper arm as round as top_arm, the
  elbow and wrist the survey's shares of it, as long as sleeve_length, the armpit at
  armscye_depth), growing out of the shoulders and hanging out at an A pose of 30°, as fitting
  avatars stand; its height from the waist to the nape is the nape to waist (measured down the
  back) × 0.977 (ANSUR II: 415.5 straight up for 425.4 down the back). The proportions are means
  computed from ANSUR II's public data (the female file, 1,986 women). The investigation is in
  `docs/debug-logs/2026-10-24-bodice-and-sleeve-drape.md`. Tests: 1 in `pf-core` (`avatar`:
  shoulders and arms from the measurements, out of the chest at the armpit, clear of the hips at
  the wrist), 2 in `tests_drape.rs` (the bodice block on the fold hangs from the base of the neck
  to about the waist, its 8 darts sewn, seams within 12 mm; with its sleeves it stays on the
  shoulders and the sleeves hang down both arms). Known limits: the avatar has no bust (its chest
  is an ellipse), so a bodice with its bust darts sewn sits about 15 mm above the waist; with
  sleeves set in, the armpits and the top of the bodice read too tight (red), the cloth pinched
  where the avatar's arm and chest meet. Not yet: trousers (they need a leg each), a better
  avatar at the bust and the armpit.
- 2026-10-24, **PF-1.6**: opening a file that is already open shows its tab. File › Open, Open
  Recent, a file dropped on the window and `file.open` used to open a second copy of a file
  already open, and the two tabs then each saved over the other's changes (found in the
  regression run of 2026-10-24). Now the open document is brought to the front, its unsaved work
  kept, and the status bar says "<name> is already open" (in all 23 languages); the same file
  named another way (a link, `..`) counts, and so does a file still being read in the background.
  The result is `{index, title, alreadyOpen: true}`. Tests: 1 in `tests_save.rs` (no second copy,
  the unsaved work kept, the other spelling, opens again once closed); the tests that saved a file
  and opened it again to see how it reopens now close it first.
- 2026-10-24, **PF-10.5**: glTF export. **Pattern › Export Avatar as glTF…**
  (`avatar.exportGlb`) and **Pattern › Export Drape as glTF…** (`drape.make` with
  `format: "glb"`, or a `.glb` path) write a binary glTF 2.0 file for 3D software and game engines
  (Blender, Unity, Unreal, three.js, Windows 3D Viewer, macOS Quick Look): the avatar in skin
  colour and, for the drape, the draped cloth coloured as the drape view colours it (each
  particle the tightness of its triangles, averaged). `pf_core::gltf` writes it from the Khronos
  Group's published specification (https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html):
  the GLB header and its JSON and BIN chunks four-byte aligned; metres, y up, the front toward +z
  (our mm, z up, become `(x, z, y) / 1000`, a mirror, so each triangle is turned back to stay
  counter-clockwise from outside); POSITION with its min and max, unit NORMALs (area-weighted),
  COLOR_0 as normalized bytes four to a vertex, in linear light as glTF's colours are; a material
  that is not metallic and rough; the cloth seen from both sides. No one's name goes in the file
  (the nodes are "Avatar" and "Drape"). Without a path the file comes back as `dataBase64`;
  `format: "obj"` still writes the cloth as OBJ. The save panel asks before the drape runs (it
  takes some seconds). Checked with the Khronos glTF Validator (open source, 2.0.0-dev.3.10):
  the drape's and the avatar's files have no errors, warnings or notes, and three.js loads the
  drape upright, facing front, in its colours. Tests: 2 in `pf-core` (`gltf`: the layout, metres
  and y up, a solid counter-clockwise from outside with its normals out, the colours' accessor;
  bad parts refused; sRGB to linear), 1 in `tests_avatar.rs` (its height, its volume positive and
  some tens of litres, written to a path the same as returned), 1 in `tests_drape.rs` (avatar and
  drape nodes, the cloth's particles and height in metres, data without a path, OBJ, a format it
  can't write refused), 1 in `tests_saveext.rs` (both menu items write a picked `.glb`). Not yet:
  the cloth's seams welded into one mesh (each piece's edge particles are their own), a glTF
  animation of the fall.
- 2026-10-24, **PF-10.4**: the drape view. **Pattern › Drape on Avatar** (`drape.make`, now
  `draw: true` by default) drapes the sewn pieces and draws the avatar and the draped cloth as two
  pictures on the layer "Drape" (draped again it replaces it): seen from the front and from the
  left side, side by side, at true scale (2 mm a pixel). `pf_core::view3d` draws them: each
  triangle lit by a light over the viewer's shoulder, the nearer in front (a depth buffer), the
  cloth coloured by how tight it is (`Cloth::tightness`: a triangle's area over its area on the
  pattern; blue at its cut size or looser, green half way, red at 10% larger or more, the way fit
  software shows strain). The result also says the tightest a triangle is (%). Also: the grid
  cells a slanted side cuts across get the triangle of their three corners inside (the cloth was
  missing a sawtooth along such sides, the body showing through at the seam), the stretch and
  tightness leave out the short links and slivers at the edge, the default precision is 20 mm
  (finer to 15), the corrections a step grow as (20 / precision)². The debugging is at the end of
  `docs/debug-logs/2026-10-24-skirt-falls-off.md`. Tests: 2 in `pf-core` (`view3d`: a square at
  its true size, the nearer of two in front; blue, green, red), 1 in `tests_drape.rs` (both views
  at 2 mm a pixel, side by side, the same height; replaced, undone, not drawn when asked). Not
  yet: turning the view round with the mouse, the drape running in the background (it holds the
  app for some seconds), bodices and sleeves.
- 2026-10-24, **PF-1.5**: the pattern commands' messages translated, and the Pattern menu's
  name. The 186 messages the pattern commands (blocks, design tools, pieces, markers, grading,
  measurements) and their drafting give when they can't do what was asked ("select a drafted
  block part (Pattern › Draft … Block)", "the dart is not that long", …) have `@msg` rows in all
  23 languages, in each catalog's sewing terms, the menu paths in them as the menus read; the
  values the user types (parameter names, "square", "5/8 in", `measure.importChart`) stay as they
  are. The top-level Pattern menu was translated with the same row as Object › Pattern (the fill
  pattern: German "Muster", French "Motif"); menu titles now look for a `menu` row first
  (`i18n::t_menu`, the in-window and the macOS menu bars), and each language has one for Pattern
  (Schnitt, Patron, Patrón, Cartamodello, Molde, Выкройка, 型紙, 纸样…), so the menu reads as the
  sewing pattern. Terms the catalogs didn't have yet (grainline, box pleat, a collar's rise and
  extension, a wing tip) were chosen by the translators and are flagged for the native-speaker
  check. Test: 1 in `i18n/tests.rs` (a German message with its menu path; the menu row against
  the fill pattern's; every language has both). Not yet: messages built with values
  (`format!`), which need templates.
- 2026-10-23, **PF-1.4**: a piece's edge data follows its seam line. Notches, per-edge
  allowances, corner styles and the fold are kept by edge number, so when the seam line gained or
  lost an anchor (an anchor added with the pen, or a design tool such as a pleat or a spread on a
  piece) they ended up on whichever edge then had their number. Now, inside the edit, the piece
  is compared with the document before it: when its seam line's anchor count changed, each notch
  is put back where it was (the nearest point on the new seam line), each new edge takes the
  allowance of the old edge its middle lies on (both halves of a split edge keep it), each corner
  style stays at the anchor in the same place, and the fold moves to the one edge along it (or
  comes off when the fold edge itself was split). Notches the edit adds itself (a pleat's) are
  already on the new seam line and are kept as they are. Test: 1 in `tests_piece.rs` (an anchor
  in the middle of a square's top edge: the notches, both halves' allowance, the bottom's
  allowance, the fold and the squared corner where they were; undo).
- 2026-10-22, **PF-1.3**: notches and the fold picked on the canvas. Right-click a pattern piece
  on or near its seam line: **Add Notch Here** puts a notch on the nearest edge, where the click
  was along it; **Cut on Fold Here** makes the nearest straight edge the fold. The commands take
  the point (`piece.addNotch {point}`, `piece.set {foldAt}`): the nearest edge of the seam line,
  within its allowance and 5 mm more, else a refusal (a click in the middle of the piece adds
  nothing). The right-click's document point is kept while the menu is open. Tests: 1 in
  `tests_piece.rs`, 1 in `tests_contextmenu.rs` (the items as the menu runs them, the refusal, no
  items without a piece). Not yet: a Notch tool that adds one per click, a notch dragged along its
  edge, allowances picked per edge on the canvas, grade points picked on the canvas.
- 2026-10-22, **PF-6.6**: an edge's allowance and a grade rule picked on the canvas. Right-click a
  pattern piece on or near its seam line: **Seam Allowance Here…** sets the allowance of the edge
  nearest the click (`piece.set {edgeAllowance, at}`), and, in a document with a size range,
  **Grade This Point…** gives the seam line's point nearest the click (within 10 mm) a grade rule,
  how far it moves each size, across and down in mm (`grade.setRule {point, across, down}`). Both
  open the command dialog for the value; the point right-clicked goes along unseen (`__fixed` in
  a menu item's params, the dialog's fixed params). Tests: 1 in `tests_grade.rs`, 1 in
  `tests_contextmenu.rs` (each item through its dialog to the piece). Not yet: dragging a graded
  point to set its rule, per-size rules in the dialog.
- 2026-10-23, **PF-6.7**: per-size grade rules edited in the Grading panel. Each point's rule is a
  step (the same move a size) with a **Per size** button: then a row for each size but the base,
  its own move from the base across and down in mm (filled from the step to start with), and a
  change sends them all (`grade.setRule {sizes}`), for grades that are not even from size to size.
  **One step** goes back to a step, the move of the size above the base. Tests: 1 more in
  `panels/tests_grading.rs`.
- 2026-10-23, **PF-7.6**: drill holes. A piece keeps its drill holes (`Piece.drills`: a point and
  the punch's diameter in mm, 0 for a mark only), which move with it and are drawn as a 3 mm cross
  (and the punch's circle) in a generated **Drill holes** part. Right-click inside a piece: **Add
  Drill Hole Here** (`piece.addDrill {point, diameter?}`, refused outside the cutting line), and
  over one, **Remove Drill Hole Here** (`piece.removeDrill {point | index}`, within 5 mm); the
  Pieces panel lists them, each with a button to remove it. DXF-AAMA writes each as a POINT on
  layer 13 with the diameter as its z, as DH Patterns and Fit's overview of ASTM D6673 describes
  ("Drill holes are placed on layer 13 … defined by a DXF point element … The Z-coordinate of the
  point can be edited to define the diameter of the punch hole":
  https://dorthehansen.com/wp-content/uploads/2014/10/ePattern-ASTM-Standard.pdf), and reads them
  back. The projector PDF and the projector view draw them; the SVG for cutting puts them in a
  `drill` group (a punch as its circle, a mark only as a cross); a piece opened out across its
  fold has them on both halves; marker copies carry them. Tests: 1 in `pf-core` marks, piece
  checks, the `pf-aama` round trip, 1 each in `tests_piece.rs`, `tests_aama.rs` (the point 60 mm
  in and 120 mm up, z 4), `tests_svgcut.rs`, `tests_contextmenu.rs` and `panels/tests_pieces.rs`.
  Not yet: the NM (not mirrored) attribute and drill-type numbers, a diameter set from the panel,
  drill holes graded with the piece.
- 2026-10-23, **PF-7.7**: T, castle and U notches. A notch can be a slit, a double slit, a V, or
  now a **T notch** (a slit with a 3 mm bar across its end), a **castle notch** (a 3 mm cut with a
  square end) or a **U notch** (the same with a round end), as DH Patterns and Fit's overview of
  ASTM D6673 describes them ("T-notch: Split with zero width at the boundary and symmetrical "T"
  width at the depth … Castle-notch: Rectangular shaped notch … U-notch: Shaped as a castle notch
  but with a semicircle reaching the depth":
  https://dorthehansen.com/wp-content/uploads/2014/10/ePattern-ASTM-Standard.pdf).
  `piece.addNotch {kind}` takes them, `piece.setNotch {index, kind}` changes one, and the Pieces
  panel picks each notch's shape from a list. DXF-AAMA writes them on layers 80, 81 and 83 (a T
  notch's point at its depth, as the overview says) and reads them back; the projector PDF, the
  projector view and the SVG for cutting draw each shape, from one drawing
  (`pf_aama::Notch::lines`) in place of three copies. Tests: `pf-core` marks (each shape),
  `pf-aama` (the layers, the T point at the depth, the round trip, each shape's lines), 1 in
  `tests_aama.rs`, the Pieces panel test. Not yet: the check notch (layer 82; the overview's
  sign for its opening is unclear), notch width and depth set per notch.
- 2026-10-23, **PF-7.8**: DXF-AAMA files open as pattern pieces. A `.dxf` with piece system text
  ("Piece Name:") that reads as ASTM D6673 (`pf_aama::read`) opens, or is placed, as its pieces
  on a "Pattern pieces" layer, in millimetres; any other DXF opens as a drawing as before. Each
  piece's sew line is its seam line (its cutting line when it has none); each edge's seam
  allowance is how far the cutting line is from the edge's middle, the piece's allowance the
  middle value and edges that differ keeping their own (a dart's legs are left out, as the cutting
  line bridges darts); the mirror line is the fold when it lies along a seam edge; notches go to the
  nearest seam edge with their shapes; the grainline, drill holes, internal lines, quantity (right
  and left pieces as the cut count and mirrored pairs) and material come along. Tests: 3 in
  `tests_aama.rs` (a square with a fold, a 40 mm hem, a castle notch and a drill hole written and
  opened again as the same piece; the skirt block's curves back with one allowance each and its
  cutting lines measuring as before to 0.02 mm; a plain DXF still a drawing). Not yet: a test with
  a file from a cutting-room program, the grade rules (.RUL, waiting on the standard), the
  annotation text (layer 15).
- 2026-10-23, **PF-7.9**: options for the pattern exports. **File › Export As…** with DXF-AAMA,
  SVG for cutting or PDF for projector now asks for the format's options before writing the file
  (it used to write it with the defaults): the rows come from the format's options in the engine's
  format table (the save options dialog, its action `file.exportAs`), with lists for the size (the
  document's size range, or As drawn), the projector's layout (Side by side or As drawn) and the
  DXF-AAMA version (R13 or R12); the style name, sample size, margin, line width, open folds,
  invert, mirror and marks as fields and checkboxes. Tests: 1 in `dialogs/tests_export.rs` (Export
  As DXF-AAMA → its options → an R12 file with the style named; the projector PDF the same way).
  Not yet: the options remembered from one export to the next.
- 2026-10-23, **PF-4.5**: block options. Each block's ease and darts are options it keeps with its
  parts (`BlockRef.options`, by name in mm; one left out is the block's default, the value the
  source gives): the bodice's ease across the half bust (5 cm), waist ease (3 cm), bust dart (by
  default worked out from the bust) and back shoulder dart (1 cm); the skirt's ease at the half
  hip (1 cm) and its back and front dart lengths (14 and 10 cm); the sleeve's ease round the top
  arm (5 cm); the trousers' ease at the half hip (2 cm) and back dart width and length (2 and
  10 cm). **Pattern › Block Options…** (`ui.blockOptions`) shows the selected block's options with
  their defaults and a button back to each; OK runs `block.setOptions {options: {name: mm | null}}`,
  which draws every part of the block again (drafted together, where it was drafted), pieces made
  from it following; `block.options` lists them, and the `block.*` drafts take `options` from the
  start. The parts still follow the measurements. Tests: 1 in `pf-core` (each option changes the
  draft; unknown and out-of-range options refused), 1 in `tests_block.rs` (7 cm ease puts the
  bodice's centre front 2 cm out on both parts, with the bust still followed; refusals, back to
  the default, undo, a piece selected), 1 in `dialogs/block_options.rs`. Not yet: options graded
  per size, more options (the armhole's depth, the front neck drop).
- 2026-10-23, **PF-4.6**: the trouser seams walked. **Pattern › Check Trouser Seams**
  (`ui.checkTrouserSeams`; the query `design.trouserSeams`) walks the drafted trouser block's front
  against its back as The Shapes of Fabric's tutorial checks it ("first walk the inseams from the
  kneeline to the hemline", then from the kneeline to the crotch point, and the side seams from the
  kneeline to the hemline and to the waistline:
  https://www.theshapesoffabric.com/2020/08/16/learn-how-to-draft-the-basic-pants-pattern/), and
  tells each run's lengths and how much longer the back's is, with the tutorial's advice (where a
  line doesn't run on smoothly, correct both lines to the point half way between; the back inseam
  is more curved, so often longer, and lowering the back crotch curve slightly is fine). The runs
  are anchor ranges of the block as drafted (`pf_core::block::TROUSER_SEAMS`). Tests: 1 in
  `pf-core` (every run starts or ends at the knee), 1 in `tests_block.rs` (the tutorial's example:
  four runs, knee to hem about 44 cm, the back inseam longer above the knee), 1 in
  `dialogs/trouser_seams.rs`. Not yet: notches matched along the walked seams (design.walk does it
  for seams named by hand), the seams of a block changed since it was drafted.
- 2026-10-23, **PF-4.7**: a front dart for the trouser block. The trouser block's `front_dart`
  option (none by default; Pattern › Block Options…) adds a waist dart in the middle of the front
  waist, square to it, `front_dart_length` long (8 cm unless set: the source leaves it open), and
  adds its width at the front side seam so the waist is as it was with the dart closed, as the
  tutorial's author answers a reader ("If you want to add a dart to the front, too, just add that
  extra width to the side seam of the front piece", "could be even just 1cm wide":
  https://www.theshapesoffabric.com/2020/08/16/learn-how-to-draft-the-basic-pants-pattern/).
  Check Trouser Seams walks a front with a dart too. Tests: 1 in `pf-core` (the side seam's waist
  end 1 cm further out, the legs 1 cm apart, the point 8 cm down, the back unchanged), 1 more step
  in `tests_block.rs` (the seams below the knee as they were, the front side seam to the waist
  shorter, standing straighter). Not
  yet: the dart's place chosen (on the crease line, as some methods put it).
- 2026-10-23, **PF-4.8**: the trouser waistband. **Pattern › Draft Trouser Waistband**
  (`design.trouserWaistband {height?: mm (30)}`) makes The Shapes of Fabric's straight waistband
  for the drafted trouser block ("You can make a straight waistband for your pants": its "width is
  twice the waistband height (max. 3 + 3cm)", its length half the waist, the side seam marked with
  a notch:
  https://www.theshapesoffabric.com/2020/08/16/learn-how-to-draft-the-basic-pants-pattern/): a
  pattern piece cut twice in mirror-image pairs, twice the height wide with a dashed fold line
  along its middle, as long as the block's front and back waists with their darts closed (so it
  fits the trousers as drafted, front dart or not), the centre front at its left end and a notch
  at the side seam on both long edges; one undo step. Tests: 1 in `tests_block.rs` (half the 74 cm
  waist give or take the curves, 6 cm wide, cut 2 in pairs, the notch the front waist along, the
  seam line 2 × (length + 6 cm) round, undo, the same length with a front dart). Not yet: a
  shaped (contour) waistband, button and buttonhole overlap.
- 2026-10-23, **PF-5.19**: flap pockets. `design.weltPocket {flap: mm, flapShape?: square | rounded
  | pointed}` (Pattern › Flap Pocket…, a rounded 5 cm flap to start from) adds the flap to the welt
  pocket's pieces, as The Shapes of Fabric's welt pocket tutorial makes one ("The length of the
  flap should be the same as your finished welt pocket, or maybe 2mm smaller", "Draw a shape you
  like and then add seam allowance all around", "cut 2 pieces per pocket and interface one of
  them", the welt's allowance at least along the top:
  https://www.theshapesoffabric.com/2019/06/24/welt-pocket-tutorial/): 2 mm narrower than the
  opening, the welt's side allowance all round (a rounded corner's radius grows with it; a
  point's sides are moved out, mitred), its stitching line dashed inside, beside the upper bag.
  The rounding (a third of the depth) and the point (a third of the depth below the corners) are
  chosen here. Tests: 1 in `pf-core` (each shape: the stitching line 118 × 50 mm, the cutting
  line 7 mm out all round), 1 in `tests_design.rs`. Not yet: a flap on a patch pocket, the
  flap's buttonhole.
- 2026-10-23, **PF-4.9**: the sleeve set into the bodice's armhole (the owner's decision). Where
  the document has a bodice block, the sleeve block's crown is that bodice's armhole (back and
  front, shoulder point to side seam, drafted with the bodice's options) plus the sleeve's new
  `crown_ease` option, by default 44.45 mm, the middle of a classic sleeve's 1½ to 2 in
  (University of Kentucky, FCS2-807); the crown depth is solved to make it that long, as for a
  `sleeve_crown` measurement, which still wins. Without a bodice it is drafted from the top arm
  table as before. It follows the bodice: a sleeve drafted first is set in once the bodice is
  drafted, and a bigger bust (or other bodice options) redraws it. Check Sleeve Ease now finds
  the blocks within the range; before, the table-drafted crown came out about 1.6 mm shorter than
  the armhole. `pf_core::block::sleeve_for`, `bodice_armhole`, `run_length`. Tests: 1 in `pf-core`
  (the crown the armhole and 44.45 mm, the option, the measurement winning), 1 more and 1 more step
  in `tests_block.rs` (drafted in either order, following the bust, the option), the Check Sleeve
  Ease dialog test (within range, then tight with no crown ease). Not yet: more than one bodice
  in a document (the first one's armhole is used).
- 2026-10-23, **PF-5.20**: design steps made again when the measurements change (first part: dart
  rotation and dart shortening). A design step made on a drafted block part used to cut it loose
  from its block (it stopped following the measurements). Now Rotate Dart and Shorten Dart on a
  block part keep it with its block and record the step (`BlockRef.steps`: a
  `pf_core::design::DesignStep`, its anchors as the outline was before it, a point given on the
  outline as its edge and the place along it, lengths in mm); when the block is drawn again (the
  measurements or its options changed) the part is drafted and its steps made again in order, and
  a step that no longer works keeps the last outline. `block.options` lists a part's steps. Tests:
  1 in `tests_design.rs` (the bust dart moved half way along an armhole curve and shortened 20 mm
  at 88 cm, the bust changed to 96 cm: the same outline as drafting at 96 cm and making the same
  two steps, to a millionth of a point; a step the new measurements don't allow keeps the last
  outline), the princess bodice test (the rotated front follows a bigger bust, undo puts it
  back). Not yet: the other design tools (split, spread, pleats, gathers, match length, smoothing,
  wrap front), a list of the steps in the Pieces panel to change or remove one.
- 2026-10-23, **PF-5.21**: more design steps made again when the measurements change: splitting
  a block part along a line (each panel keeps its block, and the step names the two anchors and
  the panel kept), slash and spread (its two points given on the outline as edge and place along
  it, the amount and the way it spreads) and truing a seam to a length (`design.matchLength`; the
  step records the length in mm, so after a change the seam is trued to that length again). The
  panels of a princess bodice now follow the bust, and so does a flared skirt. Tests: 1 in
  `tests_design.rs` (a princess front split and its side seam trued to 250 mm at 88 cm, changed to
  96 cm: the same outline as drafting at 96 cm and making the same steps; a skirt slashed and
  spread 50 mm, the hip changed). Not yet: pleats, gathers and smoothing (they also make separate
  mark nodes), wrap fronts, the steps list in the Pieces panel.
- 2026-10-23, **PF-5.22**: the design steps of a block part listed in the Pieces panel. With a
  drafted block part selected (or a piece made from one), Pattern Pieces shows "Design steps: n",
  each step in words (dart moved, dart shortened 20 mm, slashed and spread 50 mm, seam trued to
  250 mm, split into panels) with a button to remove it. `block.removeStep {id?, index}` removes
  the step and the ones after it, because a later step's anchors are on the outline the earlier one
  made; the part is drafted again from the measurements and the steps before it made again, in one
  undo. A split stays (it made two panels: delete the panel not wanted), and so do the steps before
  it. Tests: 1 in `tests_design.rs` (the last step and then the first removed, each outline as
  drafting and making the steps left; undo; a split refused), 1 in `tests_pieces.rs` (the list and
  its button). Not yet: changing a step's amount in place.
- 2026-10-23, **PF-5.23**: curved dart legs. Pattern › Curve Dart Legs… (`design.curveDart {apex,
  depth: 3 mm}`) bows a dart's two straight legs by the depth at their middle: more than 0 bows
  them out (a convex dart, which takes in more fabric between its ends and its point), less than 0
  bows them in (a concave dart, which gives room there), as Threads, "Darts build shape into a
  garment" (https://www.threadsmagazine.com/project-guides/learn-to-sew/darts-build-shape-into-a-garment)
  describes them. The dart's ends and point stay, so the seams it opens on don't change; the depth
  is at most a quarter of the dart's opening so the legs can't cross. On a block part it is a
  design step (`curveDart`), made again when the measurements change and listed in the Pieces
  panel. `pf_core::design::curve_dart`. Tests: 1 in `pf-core` (each leg's middle 3 off, away from
  the centre line or toward it; the ends stay; too deep, twice and a corner refused), 1 in
  `tests_design.rs` (the bodice's dart curved, the bust changed: the same outline and handles as
  drafting at the new bust and curving it). Not yet: truing a princess seam over the bust.
- 2026-10-23, **PF-5.24**: rounded and winged stand-collar fronts. `design.standCollar` takes
  `frontShape: "square" | "rounded" | "winged"` and `wing` (35 mm); Pattern › Draft Rounded Stand
  Collar and Draft Wing Collar. Rounded: the top front corner rounded with a quarter circle, half
  the stand's height round (or 0.9 of the extension where that is shorter). Winged: the top edge
  run on past the front end to a wing tip that turns down when worn, the front end curving in to
  it by a quarter of the wing, after the wing collar in "Pattern Making Instructions"
  (https://www.scribd.com/document/36932530/Pattern-Making-Instructions: "After constructing the
  basic mandarin collar, extend 3.5cm from F. Join F-H with an inward curve"). The neck edge, its
  length, the shoulder notch and the centre front line don't change; a shirt collar's stand stays
  square. `pf_core::design::collar_front`, `CollarFront`. The radius and the curve's depth are our
  choice (the sources leave them to the eye). Tests: 1 in `pf-core` (with and without an extension:
  the rounded corner's two anchors as far from the old corner, the curve inside it; the wing tip
  35 past it, the front end curving in by 35/4; the neck edge the same length), the bodice stand
  collar test (both fronts on the bodice block's neckline, a front not known refused).
- 2026-10-23, **PF-5.25**: box pleats of any width, and notches at a pleat's lines. `design.pleat`
  takes `face` (mm) for a box pleat: the width of the box between its two folds (twice the depth,
  as before, when it isn't given). A box pleat is two knife pleats turned away from each other, so
  each fold is brought out twice the depth (the fabric it folds under) and the spread stays four
  times the depth; the folds sit half the face either side of the spread's middle. On a pattern
  piece, each fold and placement line is notched where it meets the seam line, at both ends (the
  waist it is sewn into and the hem it is pressed to), in the same undo step; `notches: false`
  leaves them off. `pf_core::design::pleat(…, face, …)`. Tests: 1 more in `pf-core` (a 100 face
  on a 20 deep pleat: the folds at 90 and 190, the placements at 50 and 230; the old box pleat the
  same), 1 in `tests_design.rs` (a 60 face in the skirt piece: 8 notches, the folds 60 apart, one
  undo, without notches, a face on a knife pleat refused). Not yet: pleats from the Pattern menu
  (they need two points picked on the canvas), notches already on a piece kept on their edges
  when a design tool adds anchors to its seam line.
