# Handoff 03 — The records disagree with the code and with each other

**Milestone:** M9, unit 03. **Items:** C4, C6–C9, E, F-2.
**Written:** 2026-09-30, with every item **re-checked on the current tree** — several have moved
since the audit, one has got worse, and two are already fixed.

## Purpose

Correct the statements that are false. Nothing here changes behaviour; all of it changes what a
reader would conclude.

## Background — the current state of each item, verified 2026-09-30

**Do not work from the original audit. Work from this table**, and re-check each before changing it.

| Item | Audit said | Now |
|---|---|---|
| **C4** | `fuzz/README.md:45` says CI "does not run" the fuzz targets, but `fuzz-smoke` runs each at `-runs=20000` | **Worse.** The line is now `fuzz/README.md:84` and it is wrong in a second way: `fuzz_open_xlsx_bytes` runs at **`-runs=0`** and the other three at 20,000. Also, the audit cited `CONTRIBUTING.md:28`; that file is at **`.github/CONTRIBUTING.md`** |
| **C6** | RFC-035 and RFC-036 sit in `accepted/` with Status "units … are live", while `rfcs/README.md` calls 035 "delivered in 2.3.0" | Still true, and **RFC-037 is now there too** (v3 scope, delivered in 3.0.0) |
| **C7** | `performance.md`'s "~450 B/cell" was measured on the dense read that f123 replaced; nothing says whether it was re-measured | Still there, `performance.md:125`. **Not a claim to delete — a claim to date or re-measure** |
| **C8** | Five handoff directories are not `NNN-slug/` | **Eighteen now.** `f123`–`f135`, `m4`–`m10`, `release-*`, `alignment-followups`, `upstream-calamine-*`. The convention is the outlier, not the directories |
| **C9** | `README.md` says `calamine` 0.36 is "pinned"; `Cargo.toml` has `calamine = "0.36"`, a caret range | Still true: `README.md:93`, `Cargo.toml:49`. `Cargo.lock` pins 0.36.1 |
| **E** | `_new_remaining`; `let _ = confidence;`; a repeated doc line in `lib.rs`; a doubled position fetch in `diff.rs`; the issue template asking "How you started the server" | **`_new_remaining` is gone** (f133 removed it). `let _ = confidence;` remains. The issue-template line remains. **Re-check the `lib.rs` and `diff.rs` line numbers — they have shifted** |
| **F-2** | RFC-013 §5's exit-code table names 4 and 5, which the CLI never emits | Verify against `src/main.rs`'s actual exit codes before editing |

## Change scope

- `fuzz/README.md`, `README.md`, `docs/src/maintainers/performance.md`
- `rfcs/README.md` and the three RFCs in `accepted/`, **if** C6 resolves to moving them
- `rfcs/done/013-*.md` for F-2
- `src/matcher.rs`, `src/lib.rs`, `src/diff.rs` — **comments and dead bindings only**
- `.github/ISSUE_TEMPLATE/bug_report.md`
- `CHANGELOG.md` under `### Documentation`

## Non-change scope

- **No behaviour changes.** `git diff src/` must contain no executable change — a deleted `let _ =`
  and comment edits only. If something looks like it needs a real change, **report it**.
- **Do not rename the handoff directories** (C8). See *Required implementation* 4.
- Do not move an RFC out of `accepted/` on your own judgement — C6 needs a ruling, see §2.

## Required implementation

**1. C4: make `fuzz/README.md` true about what CI runs.** It must say that `fuzz-smoke` runs all
four targets, that `fuzz_open_xlsx_bytes` is at `-runs=0` and why, and that the other three mutate.
Note the file already explains the `-runs=0` decision further up — the stale line contradicts its own
document. Check `.github/CONTRIBUTING.md` says something true too.

**2. C6: report, do not resolve.** Whether an RFC moves from `accepted/` to `done/` is governed by
`.git-exclude/rules/000-rfc-lifecycle-policy.md`, and three RFCs are affected (035, 036, 037) with
different amounts of their scope delivered. **Read the rule, state for each RFC what it says should
happen, and propose.** I will rule. Do not move files.

**3. C7: date the figure or re-measure it.** Either is acceptable and they are different claims. If
you re-measure, say so and give the number. If you date it — "measured on the pre-f123 dense read,
not re-measured since" — that is honest and cheap, and it is what the rest of `performance.md`
already does for point-in-time figures. **Do not delete it silently**; a figure with a date is
useful and a missing figure teaches nothing.

**4. C8: propose, do not rename.** Eighteen directories deviate and the convention has been ignored
by every milestone since m4. That makes the *convention* the thing to change, not the directories —
renaming eighteen would break every cross-reference in every review file. **Propose the wording that
makes the current practice the rule** (RFC-numbered directories for RFC work, slug directories for
milestones and defect responses, which is what actually happens), and let me rule.

**5. C9: say which.** Either `Cargo.toml` pins exactly and the README is right, or the README should
say "0.36.x, resolved by the lockfile". **The second is almost certainly the honest one** — but check
whether anything depends on the caret range before proposing the first.

**6. E and F-2: fix what is trivially fixable, verify each line number first.** `let _ =
confidence;`, the repeated doc line, the doubled position fetch, the issue template. For F-2, read
`src/main.rs`'s exit codes and correct RFC-013 §5 to match — **or, if the RFC is right and the CLI
is wrong, stop and report**, because that is a behaviour question and not this unit's.

## Required tests

None new. The test is that the gates still pass and `git diff src/` shows no executable change.
**If you find yourself wanting a test, you are changing behaviour — stop.**

## Acceptance criteria

1. Every item in the table either corrected, or proposed-with-reasoning for the three that need a
   ruling (C6, C8, and C9 if you propose pinning).
2. Each line number re-verified before editing; the table above re-checked, not trusted.
3. `git diff src/` contains no executable change.
4. Gates as always, including doctests — several of these files carry compiled assertions.
5. `CHANGELOG.md` under `### Documentation`, one entry, not six.

## Prohibited shortcuts

- Do not work from the audit's line numbers. They are weeks old and I have already found three that
  moved.
- Do not resolve C6 or C8 by doing the tidy-looking thing. Both change conventions that other
  documents reference.
- Do not delete a figure to make a document consistent. Date it.

## Known risks

**1. Doctests.** `docs/src/*.md` files carry executable assertions; f135 had to fix two. If you edit
a number in a Markdown file, check whether it is inside an `assert_eq!`.

**2. C8 is a trap that looks like tidying.** Renaming eighteen directories would invalidate paths
quoted in dozens of review files under `.git-exclude/`, which are the project's own record of what
was decided. The cheap fix is the wrong one.

**3. Some of these may already be fixed by the time you start.** Two were between the audit and this
handoff. Re-check, and **report anything already correct** rather than editing it into a different
correctness.

## Required evidence

Under `.git-exclude/review-request/m9-03-the-records-disagree/evidence/`:

1. The table above, re-verified, with what you found for each.
2. For each correction: the before and after text.
3. For C6, C8, C9: your proposal and its reasoning.
4. `git diff src/`, showing no executable change.
5. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/m9-03-the-records-disagree/README.md`, with:

- Which items were already fixed, and which had moved.
- Your proposals for C6, C8 and C9, each with the case against.
- Anything false you found that is not in the table. This is the third records unit; the first two
  each found something the audit missed.
