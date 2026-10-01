# Handoffs — GitHub Releases say our latest version is 2.3.0

**A slug directory, not `NNN-slug/`.** No RFC governs it; it is a defect in how the repository
presents itself, found by the owner. Named per `rfcs/README.md`'s rule, which asks a non-`NNN-slug/`
directory to say why it is one.

## The defect

`gh api repos/forskscope/sheets-diff-rs/releases/latest` returns **`2.3.0`**, dated 2026-08-16.
crates.io serves **3.3.0**. **Sixteen tags have no release at all**, including every 2.x after 2.3.0
and the whole of 3.x — so the repository's own sidebar tells a visitor the project last shipped in
August, and that a major version they would then not know about does not exist.

The releases were hand-written and stopped. **This is the quarter's recurring defect pointed at our
users instead of at us**: a marker asserting more than it measured. It is not an absence — absence
would be honest. It is a false `Latest`.

**Why not simply stop.** The Releases tab cannot be hidden, so "deliberately no releases" leaves
`Latest: 2.3.0` standing forever. The only honest form of stopping is deleting all seventeen, which
destroys the 1.x history for no gain.

## Queue

| | Unit | Nature |
|---|---|---|
| 01 | [Generate releases from the CHANGELOG, and backfill](./01-generate-and-backfill.md) | One workflow job, one extractor, sixteen backfills |
