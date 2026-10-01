# Handoffs — Promises with no test

**A slug directory, not `NNN-slug/`.** No single RFC governs this: it comes from a consumer's
observation about a commitment we made in correspondence and in a doc comment, and the work is a
guard rather than a feature or a defect response. Named per `rfcs/README.md`'s rule, which asks a
non-`NNN-slug/` directory to say why it is one.

## Why this exists

On 2026-10-01 ForskScope wrote, about a test they had built against a hazard we then promised not to
create:

> **The test we wrote against the old hazard stays**, and it changes character rather than losing
> its job. It pinned a defence against a documented behaviour you might one day implement; now that
> you have said you will not, it pins the promise instead. That is worth more, not less — **a
> commitment in a letter is not a compile error, and the next person to bump the dependency here
> will not have read your letter.**

They were describing their own repository. It applies to ours, and worse: **they have that test and
we do not.** Our commitment — that a formula cell's cached value is compared regardless of
`include_formula_cached_values` — lives in a letter we sent and a doc comment we wrote. Neither
fails a build.

This is the inverse of the eight markers M9 kept finding. Those were claims credited with more than
they measured. This is a claim with **nothing measuring it at all**.

## Queue

| | Unit | Nature |
|---|---|---|
| 01 ✅ | [Pin the cached-value promise, and survey what else is unpinned](./01-pin-the-promise.md) | One guard, plus a reported list |
| 02 ✅ | [Pin `hardened()`'s constants](./02-pin-the-hardened-constants.md) | From 01's survey. The documented boundary is arithmetic over one unpinned constant |

## Closed 2026-10-01 — and the answer was "no convention"

Two units. Eight promises surveyed across both:

| | |
|---|---|
| **Genuine gaps**, both now closed by a named test | 2 — `include_formula_cached_values`, the `hardened()`/`default()` constants |
| **Already pinned by consequence** | 4 — `SheetRef::index`, `address::MAX_ROW`/`MAX_COL`, `open::CFB_ENCRYPTED_MARKER_SCAN_LIMIT`, and `max_cells_read`'s both-sides mechanism |
| **Unpinnable by construction** | 1 — `min_severity`: advice about mechanism, byte-identical output either way, no public measurement of the thing it claims |
| **No live claim left** | 1 — `CANCEL_POLL_INTERVAL`, whose doc comment already withdraws its own derived figures |

**So no blanket rule.** "Every documented claim needs a test" would have made work for six items
that are already protected or have nothing to protect — and this project has had enough trouble with
checks that measure less than they claim without adding six that measure nothing.

**What the survey did reveal, and the reason this file still exists:** four of the eight are *pinned
by consequence*, not by name. `address::MAX_COL` is genuinely protected — changing 16,384 to 16,383
fails `address_downstream` — but the failure reads `"XFC1048576" != "XFD1048576"` in a test named for
something else. It tells a developer that something broke, not **which documentation they just
falsified.**

That is protection without explanation. It is not a defect and nothing is scheduled for it;
converting a by-consequence pin into a by-name one costs a comment, and is available to whoever is
next in those files. Unit 02's own test is the worked example: it names `hardened()`'s doc table and
the threat model's boundary paragraph as the things depending on its numbers.

**Origin, for the record:** ForskScope, 2026-10-01 — *"a commitment in a letter is not a compile
error, and the next person to bump the dependency here will not have read your letter."* They were
describing their repository. It was true of ours in two places.
