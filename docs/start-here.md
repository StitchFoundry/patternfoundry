# Start here: notes for a new session

Written 2026-10-19 from the StitchFoundry session where PatternFoundry was decided. Read this,
then `docs/plan.md` and `docs/market.md`.

## The owner
**Bruce Wattendorf**, doing business as **SiteSentrix** (Rhode Island). He runs StitchFoundry
(embroidery digitizing) and is building a family of maker software: StitchFoundry,
LightFoundry (laser), MakerFoundry (marketplace) and PatternFoundry (sewing patterns). Works from
a Windows PC and the Claude app on his phone; plain, step-by-step instructions work best for
anything he clicks through.

## What exists to build on
- **VectorCraft** (github.com/storytold/vectorcraft): the Rust vector editor to merge in (PF-0).
- **CADCraft** (github.com/storytold/cadcraft), the ArtCraft team's Rust CAD app, under the same
  licence as PatternFoundry (MIT OR Apache-2.0), so its code may be reused with its copyright notice
  kept (in `NOTICE`) and none of the ArtCraft branding. Worth taking rather than rewriting (owner's
  pointer, 2026-10-19): `crates/constraints` (a parametric solver for geometric and dimensional
  constraints with expressions: PF-3 formulas and PF-5 drafting), `crates/dxf` (a standalone DXF
  tag reader/writer from the published DXF reference: the base for DXF-AAMA in PF-7), and its
  paper-space layouts, page setups and plotting to PDF (PF-2). Leave its `dwg` crate (it wraps the
  MPL-2.0 `acadrust`); DWG is not on our plan.
- **StitchFoundry** (github.com/Stitchwright/StitchFoundry): VectorCraft plus embroidery; the
  model for how a mode is added (its `docs/stitchfoundry-spec.md`, `crates/sf-core`, the
  `embroidery.*` commands, the AI features and the Foundry Account window), and its CI, release and
  translation set-up (23 complete languages).
- **StitchFoundry cloud** (cloud.stitchfoundry.app): the shared **Foundry Account** (one sign-in and
  one credit balance for every SiteSentrix app), credits, Stripe, AI (see Hosting, accounts and AI
  below).
- Websites on Cloudflare Pages from bwattendorf/sitesentrix-web; email through Cloudflare Email
  Routing and Resend; hosting on Railway Pro.

## Getting started (the new-user flow, #146)

A pattern starts from measurements, not a page size. On the Home screen choose **Start a pattern**
(or File › Start a Pattern…):

1. **What are you making?** A bodice, skirt, sleeve or trousers.
2. **Whose measurements?** Type your own (each has a short "how to measure" line; choose centimetres
   or inches), pick one of the round sample sizes, or use a set you kept in My Measurements.
3. **Create Pattern.** The pieces are drafted and the page is fitted to them; the paper size is only
   chosen later, at Print (home printers get tiled pages with the 10 cm test square).

Typed measurements stay on your computer. Tick "Keep these measurements" to save them to My
Measurements for next time. Prefer the old way? **Blank page** (or File › New…) still offers the page presets.

Code: `crates/ui-egui/src/dialogs/start_pattern.rs` (the dialog, `ui.startPattern`). Pattern › Draft … Block with no document or no measurements now opens a document or the Measurements panel and says what is missing (#138).

## Decided
- Name: **PatternFoundry**; sewing-pattern design software; built on VectorCraft like
  StitchFoundry.
- We build our own and never copy (see `CLAUDE.md`): commercial software is studied only through
  its public documentation, to match and beat its features.
- **Its own app** (owner's decision, 2026-10-30, replacing 2026-10-23's "a mode inside
  StitchFoundry"): PatternFoundry stays a separate app in this repository and is not combined
  into StitchFoundry. PF-11 (the move) is dropped.

## Hosting, accounts and AI (owner's decisions, 2026-10-19)
- **Hosting:** GitHub Pages for the web build and GitHub Releases for the desktop downloads, as
  StitchFoundry does. PatternFoundry has no server of its own and nothing on Railway.
- **Accounts and AI:** the shared **Foundry Account** (one sign-in and one balance of "Foundry
  credits" across all SiteSentrix apps), served by the StitchFoundry cloud service,
  https://cloud.stitchfoundry.app. PatternFoundry never holds an Anthropic key: AI requests go
  through the service, which adds the owner's key and charges the user's credits. The Foundry
  Account is the only way to sign in and pay for AI: the app has no "use my own API key" option
  (#140; a key saved by an older version is deleted on load).
- **Building the AI features (PF-8):** reuse StitchFoundry's client code
  (`crates/sf-core/src/ai/cloud.rs`, and the Foundry Account window in
  `crates/ui-egui/src/embroidery/account.rs`), and point builds at the service with `SF_CLOUD_URL`,
  as StitchFoundry's build workflows do.
- **The service:** `POST /v1/signup`, `/v1/signin`, `/v1/signout`, `/v1/account`,
  `/v1/checkout {pack}` and `/v1/messages`, with a bearer token `sfs_…`. `/v1/messages` accepts
  only `model`, `max_tokens`, `system`, `messages`, `tools`, `tool_choice`, `output_config`,
  `fallbacks` and `thinking`; no streaming; `max_tokens` at most 16,000; the model must be one the
  service allows (today `claude-opus-5-5`). For another model or limit, ask the StitchFoundry chat:
  the service lives in github.com/Stitchwright/StitchFoundry, `apps/stitchfoundry-cloud`, documented
  in its `docs/cloud.md`.
- Body measurements still stay on the user's computer: an AI request sends only what that request
  needs, and nothing is logged (CLAUDE.md).

## The owner decides (ask, don't assume)
Money, legal wording, anything public, domains, and the open decisions in `docs/plan.md`.

**Standing instruction (2026-10-23):** the owner has no pattern-making experience and wants the
work carried on through the list without stopping for questions: on pattern-making and technical
choices, do what you recommend and record why (in `docs/plan.md`, citing the public source).
Still ask before spending money, publishing anything, or touching accounts and services outside
this repository; note what such an item needs and work around it.

## Next task
PF-0 to PF-4 are done (with PF-4.1 and PF-4.2, the sleeve and trouser blocks; PF-4.4, Check Sleeve Ease;
PF-3.4, My Measurements kept on this computer; PF-1.1, the cutting
line bridging darts), PF-5's tools are in (dart rotation, splitting, slash and spread, seam length
and matching, walking seams, smoothing across seams, facings, knife, inverted and box pleats,
gathering marks, flat, stand and shirt collars, patch, in-seam and welt pockets) and PF-7 has
DXF-AAMA export (PF-7.1), SVG for cutting (PF-7.2) and the projector PDF (PF-7.3; see the
progress log in `docs/plan.md`). PF-6 (grading, PF-6.1 to 6.5) is done but for graded DXF-AAMA
(the rule table), which waits on the owner (see PF-6.3). PF-8 has Ask About Patterns and
Foundry Account (PF-8.1), the wrap dress's pieces (PF-5.16, 5.17) and Check My Muslin (PF-8.2).
PF-9 has markers (PF-9.1, Pattern › Make Marker…) with pieces cut on the fold and folded fabric
(PF-9.2); the exports know the fold too (PF-7.4: the DXF-AAMA mirror line, opened out on the
projector), and Pattern › Projector View shows the pattern at true size on a calibrated projector
(PF-7.5). Next: a real request through the
Foundry service, then drafting a garment from a description; selling patterns on MakerFoundry is still to do.
