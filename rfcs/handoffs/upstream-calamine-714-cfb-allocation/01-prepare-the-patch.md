# Handoff — Prepare a patch for `calamine#714` (do not submit it)

**Not a `sheets-diff` change. Work on someone else's codebase, prepared here and submitted by the
owner** (`.git-exclude/rules/004-outward-facing-communication.md`).
**Written:** 2026-09-29. **Amended 2026-09-30 — read §0 first. Now next: proceed.**
**Does not gate any release of ours.** **Agreed:** owner, 2026-09-29.

## 0. Amended 2026-09-30 — two allocation sites, not one, and that changes the patch

**Since this was written:** f133, f134 and f135 shipped or merged, 3.2.0 is released,
GHSA-w5x2-6474-pqp4 is published, and a RustSec advisory for our crate is open as
[rustsec/advisory-db#3297](https://github.com/rustsec/advisory-db/pull/3297) — whose body states
publicly that we are *not* filing against `calamine` because no fixed version exists to point at.
This patch is what would change that, so assume the maintainer may read it in that context. It
changes nothing about the work; be aware of it in the PR's tone.

**The substantive amendment. Three independent reproductions exist, and they do not all hit the same
`Vec::with_capacity`.** Arithmetic checked 2026-09-30:

| Reproduction | requested bytes | ÷ 4 | site |
|---|---:|---:|---|
| **#714's own** `fat-count.xlsx` | 17,179,869,180 | `0xFFFFFFFF` (`u32::MAX`) | **`fat_len`** — `fats` at `:122` |
| **ForskScope's** hand-built header | 9,261,023,232 | `0x8A000000` | **`fat_len`** — `fats` at `:122` |
| **ours** (`tests/fixtures/f132/oom-artifact-515b.bin`) | 9,261,285,372 | `0x8A00FFFF` | **`difat_len`** — `difat` at `:260` |

**A patch that fixes only the DIFAT site leaves both of the other two reproductions working.** The
DIFAT one is the tempting target, because it has the obvious second bug attached (the count is read
from `buf[62..76]`, a fourteen-byte slice, where MS-CFB puts it at `72..76`) — and that obviousness
is exactly the trap. The FAT site at `:122` has no wrong-offset bug and is just as unbounded.

This was inferred from reading the code when this handoff was written; it is now **measured, by two
parties independently**, one of whom was not looking for it. Fix both. The tests in §3 must cover
both, which means two artifacts or two crafted headers, not one.

## Purpose

`calamine` allocates from two untrusted header fields before reading anything, which is the defect
f132 worked around on our side. It is open upstream as
[#714](https://github.com/tafia/calamine/issues/714) with **no PR and no reply since 2026-08-31**.
The maintainer has said plainly that the project is not resourced for the volume of vulnerability
reports it receives and that *"there are enough public issues that need to be fixed first."* A
tested patch is the one contribution that answers that rather than adding to it.

## Why this is worth doing even though we are already safe

Our pre-screen makes the CFB parser unreachable through our API, so **we gain nothing from this
patch.** Everyone else using `calamine` on untrusted input does. Do it for that reason or not at all,
and keep the expectations honest: **[#696](https://github.com/tafia/calamine/pull/696), the fix for
the other defect we hit, has been open since 2026-07-27 — two months, unmerged.** Assume this one
sits too. **Nothing of ours may depend on it.**

## The defect, as I read it — verify before trusting

`src/cfb.rs` on `main` (fetched 2026-09-29; 518 lines):

```rust
let fat_len   = read_usize(&buf[44..48]);   // :253
let difat_len = read_usize(&buf[62..76]);   // :258  <- a 14-byte slice
let mut difat = Vec::with_capacity(difat_len);   // :260
difat.extend(to_u32(&buf[76..512]));             // :261
...
let mut fats = Vec::with_capacity(h.fat_len);    // :122
for id in difat.into_iter().filter(|id| *id < DIFSECT) {
    fats.extend(to_u32(sectors.get(id, reader)?));   // :124
}
```

**Two separate bugs in those lines.**

1. **The DIFAT count is read from the wrong bytes.** Per MS-CFB the header is: `44..48` number of FAT
   sectors, `48..52` first directory sector, `60..64` first mini-FAT sector, `64..68` number of
   mini-FAT sectors, `68..72` first DIFAT sector, **`72..76` number of DIFAT sectors**. The code reads
   `62..76` — fourteen bytes straddling three unrelated fields. #714's title says this; confirm it
   against the spec yourself before asserting it in a PR.
2. **Both counts reach `Vec::with_capacity` unchecked against the file's actual size**, which is the
   9.26 GB allocation.

**The finding that makes the patch small — check it first, because the whole shape depends on it.**
Grep every use of both fields on `main`:

```
121: debug!("load fat (len {})", h.fat_len);
122: let mut fats = Vec::with_capacity(h.fat_len);
192: fat_len: usize,          (struct field)
253: let fat_len = ...        (read)
258: let difat_len = ...      (read)
260: Vec::with_capacity(difat_len)
268: fat_len,                 (struct construction)
```

**Neither count is used for anything but a capacity hint and a log line.** Both vectors are filled by
`extend` from data read elsewhere — `difat` from `buf[76..512]`, `fats` from actual sector reads — so
the counts do not determine length, contents, or control flow. **If that holds, removing the
amplification cannot change behaviour at all**, which is what makes this patch mergeable by a
maintainer with no time: it is not a judgement call about limits, it is deleting a hint that was
never load-bearing.

## Required work

**1. Confirm the reading above on a fresh clone**, at the commit you patch. My grep was of a single
file fetched through the API; a caller elsewhere in the crate would change the answer. **If either
count turns out to be load-bearing, stop and tell me** — the patch becomes a design question about
bounds rather than a deletion, and that is mine to scope.

**2. The minimal patch.** Assuming step 1 holds:
   - `Vec::with_capacity(difat_len)` → `Vec::new()`, and likewise for `fats`. Do not clamp to an
     invented constant; do not add a `Limits`-style knob to someone else's crate.
   - Fix `62..76` → `72..76` so the debug log and the field are correct.
   - **Prefer deleting `difat_len` entirely** if nothing reads it once the capacity hint is gone —
     an unused wrong value is worse than no value. Judge it against the maintainer's taste, not ours;
     if removing a struct field looks invasive, keep `fat_len` and say why in the PR.

**3. A test, because this is the whole point.** A unit test in their layout that feeds a small
crafted CFB header with a large sector count and asserts an ordinary `Err` (or a successful parse
that allocates nothing large) rather than an abort. Use **#714's own attached `fat-count.xlsx`** as
the case if their test conventions allow a fixture; otherwise construct the header in code. Match
their existing test style — read `tests/` first and follow it rather than importing ours.

**4. Verify against both artifacts, and know which site each one exercises** (§0). Ours
(`tests/fixtures/f132/oom-artifact-515b.bin`, 512 bytes, 9,261,285,372 bytes requested — the
**DIFAT** site) and #714's (`fat-count.xlsx`, 17,179,869,180 — the **FAT** site). **Both must return
`Err`.** If only one does, the patch is half done and the other reproduction still works.

Run them **under a memory cap in a child process**. An uncapped run on a machine with lazy overcommit
survives the allocation and returns an ordinary error, which looks exactly like a pass — f132's review
hit this, and ForskScope then hit it independently and told us it is the reason a reader of our
advisory could conclude they were unaffected. **A green test on an uncapped big machine proves
nothing here**, before or after the patch, so capture the cap in the evidence.

**5. Their gates, not ours.** `cargo test` across their feature sets, `cargo fmt`, `cargo clippy`.
Do not impose our lint settings, our commit-message style, or our documentation conventions on their
repository. Read `CONTRIBUTING.md` if present and follow it.

## Non-change scope

- **Do not submit anything.** No PR, no issue, no comment, no fork pushed to a public remote under
  the owner's account. Rule 004. The deliverable is a patch file and a PR description, reviewed by
  me, sent by the owner.
- **Do not fix the other defects while you are in there.** `#694`/`#696` (column overflow), `#693`
  (`Range::from_sparse`), `#684` (shared-strings capacity) are separate, some already have PRs, and a
  patch that touches four things is a patch nobody can review. One defect, one patch.
- **Do not change our crate.** `git diff --stat` on `sheets-diff` must be empty apart from this
  handoff's own review artefacts.
- Do not bump our `calamine` dependency, even if the patch is accepted. That is a later, separate
  decision.

## Required evidence

Under `.git-exclude/review-request/upstream-calamine-714-01/evidence/`:

1. The grep that establishes both counts are capacity-only, at the commit you patched.
2. The patch itself (`git format-patch` against their `main`), and the commit it applies to.
3. Both artifacts through their public API, before and after, memory-capped in a child process.
4. Their full test suite, before and after, showing no change.
5. The proposed PR title and description, as a separate file — **written for their maintainer, not
   for us**: what the bug is, why the fix cannot change behaviour, what the test covers. Short.
   No reference to `sheets-diff`'s internals, no mention of our fuzzing infrastructure, no
   suggestion that they owe anyone a fix.

## Review request format

`.git-exclude/review-request/upstream-calamine-714-01/README.md`, with:

- Whether step 1 held, with the evidence.
- Whether you removed `difat_len` or kept it, and why.
- Anything in their code that made you want to change more than one thing, **listed and not done**.
- Your read on whether the PR description is in their register. I will review it for tone as much as
  for content; a patch to an overloaded maintainer's repository is a favour being offered, not a
  defect report being escalated, and it should read like one.
