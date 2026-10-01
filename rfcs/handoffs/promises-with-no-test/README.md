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
| 01 | [Pin the cached-value promise, and survey what else is unpinned](./01-pin-the-promise.md) | One guard, plus a reported list |
