# Handoff 01 — Generate releases from the CHANGELOG, and backfill the sixteen

**Written:** 2026-10-01. **Approved:** owner, 2026-10-01.
**You do not publish anything.** The workflow you write runs on a tag push, which is a joint action
at the cut; the backfill is sixteen publications under the project's name and is the owner's to
trigger (`.git-exclude/rules/004-outward-facing-communication.md`).

## Purpose

Make the GitHub release list true, and make it stay true without anyone remembering to do it.

## Background

**Verified 2026-10-01.** Seventeen releases exist, the newest is `2.3.0`, and
`repos/.../releases/latest` returns `2.3.0`. These tags have no release:

```
2.0.0 2.0.1 2.1.0 2.2.0 2.2.1 2.2.2 2.2.3
2.4.0 2.4.1 2.5.0 2.5.1 2.6.0 3.0.0 3.1.0 3.2.0 3.3.0
```

No workflow creates releases — `grep -rln "gh release" .github/workflows/` is empty. They were made
by hand and the habit lapsed.

**The release body is not a second record.** `CHANGELOG.md` has 18 cleanly delimited `## [x.y.z] -
YYYY-MM-DD` sections; the release body is **extracted** from the one matching the tag. One source of
truth. **Hand-written notes are what stopped last time, and a hand-maintained copy of the CHANGELOG
is the drift this project has spent a week correcting in six other places.**

## Change scope

- `.github/workflows/ci.yaml` — one new job.
- A script for the extraction and the backfill. **There is no `scripts/` or `xtask` directory** — I
  looked. The nearest convention is `examples/` (`gen-fixtures.rs`, `gen-fuzz-corpus.rs`), which
  holds maintainer tools that are compiled Rust and ship in the package. A release extractor is
  neither of those things, so **pick a home and argue for it** — including "a step inline in the
  workflow, with no script at all", which may well be the right answer for something this small.
- `CHANGELOG.md` — `### Documentation`.

## Non-change scope

- **Do not create, edit or delete any release.** Not even 3.3.0, not even to test. Use
  `--dry-run`-equivalent paths and a scratch repository if you need a live check, and say so.
- **Do not edit the seventeen existing releases**, including 2.3.0's hand-written title.
- Do not change any tag.
- Nothing under `src/` or `tests/`.

## Required implementation

**1. The extractor.** Given a version, emit the CHANGELOG section between `## [<version>]` and the
next `## [`, excluding both headings. **If there is no section for that version, exit non-zero with
a message naming the version.** A tag we never wrote notes for is a release we should not publish,
and silence there is how this started.

**2. The job.** Gated on a tag push, the way `package` already is at `ci.yaml:289`
(`startsWith(github.ref, 'refs/tags/')`). It must:

   - **depend on the gates passing.** A release published for a commit whose tests failed is worse
     than no release. Use `needs:` — list what actually matters and say why you chose that set.
   - **run after `package`**, since `cargo publish --dry-run` is the check that the thing being
     announced can actually be published.
   - be **idempotent**: re-running a tag's workflow must not fail and must not duplicate. Decide
     between "skip if it exists" and "update in place", and argue for one.
   - set the release name to **the plain version**, matching every existing release except 2.3.0's
     hand-written title. Do not invent descriptors.

**3. Permissions — and this one needs saying in the workflow, not just doing.** The top of
`ci.yaml` declares `permissions: contents: read`, with a comment citing RFC-034 §5.4 and asserting
*"No job in this workflow needs more."* **Creating a release needs `contents: write`, so that
comment becomes false the moment you add this job.** Grant it at **job level only**, never at the
top, and **correct that comment** to say which single job holds write and why. Leaving a now-false
assertion in place is the exact defect this unit exists to fix.

**4. The backfill.** A script that creates the sixteen missing releases from the same extractor, so
backfilled and future releases are identical in shape. Two requirements:

   - **Ascending version order**, and pass `--latest` **only** for `3.3.0`. GitHub's `latest`
     is a property, not a derivation — get it wrong and the list is still lying, just differently.
   - **Verify afterwards** with `gh api repos/.../releases/latest --jq .tag_name` and nothing else.
     `gh release view --json isLatest` **does not exist** — I tried it; the field list it printed in
     the error does not include it. Do not report success from a command that cannot answer.

   **Write the script and the verification. Do not run it against the real repository.**

## Required tests

- The extractor, against **every** version in `CHANGELOG.md`: each must yield a non-empty body, and
  a made-up version must exit non-zero. 18 sections; check all of them, not a sample.
- The extracted body for one release quoted in the evidence, so I can read what a user would see.
- A dry run of the backfill showing the sixteen commands it *would* issue, in order, with
  `--latest` on exactly one.

## Acceptance criteria

1. The extractor works for all 18 sections and fails loudly for an unknown version.
2. The job is gated on tags, depends on the gates, runs after `package`, is idempotent, and holds
   `contents: write` at job level only.
3. `ci.yaml`'s top-level permissions comment corrected.
4. The backfill script exists, orders ascending, flags `--latest` only for 3.3.0, and verifies via
   the API.
5. **Nothing published.** `gh release list` identical before and after your work — include it in the
   evidence both ways.
6. Gates as always, plus rule 003.

## Prohibited shortcuts

- Do not hand-write a release body. The CHANGELOG is the source.
- Do not grant `contents: write` at the top of the workflow.
- Do not leave the permissions comment saying no job needs more than read.
- Do not test by creating a real release "and then deleting it".
- Do not report `isLatest` from a tool that has no such field.

## Known risks

**1. A tag whose CHANGELOG section does not exist — I checked, and there is none.** All sixteen
backfill targets have a `## [x.y.z]` section. I am leaving the risk here because the *extractor*
must still fail loudly on an unknown version (§1) for every future tag, and because you should
re-verify rather than take my grep: I have had three counts wrong in handoffs this week.

**2. `needs:` is a judgement call.** Too narrow and a release can ship on a red build; too wide and
a flaky unrelated job blocks a release. State your choice and the failure mode you preferred.

**3. The `package` job is itself gated on tags**, so `needs: [package]` is only meaningful on a tag
run. Confirm the interaction rather than assuming it; a `needs:` on a skipped job can make a job
skip silently, which would be a release that never happens and never complains — the same shape as
the defect being fixed.

## Required evidence

Under `.git-exclude/review-request/releases-01-generate-and-backfill/evidence/`:

1. The extractor, and its output for all 18 versions (non-empty each) plus the unknown-version
   failure.
2. One full extracted body, as a user would read it.
3. The backfill dry run: sixteen commands, in order, `--latest` on one.
4. Any tag with no CHANGELOG section (Known risk 1).
5. `ci.yaml`'s diff, including the corrected permissions comment.
6. `gh release list` before and after your work — unchanged.
7. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/releases-01-generate-and-backfill/README.md`, with:

- Where you put the script and what convention you followed.
- Your `needs:` set and the failure mode you chose.
- Your idempotency choice, argued.
- Whether every tag has a CHANGELOG section.
- Whether anything else in `.github/workflows/` asserts something that is no longer true. The
  permissions comment was one; I would rather find out now than next quarter.
