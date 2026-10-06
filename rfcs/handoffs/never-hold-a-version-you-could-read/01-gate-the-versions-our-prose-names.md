# Handoff 01 — Gate the versions our prose names

**Unit:** never-hold-a-version-you-could-read 01. **Added 2026-10-06.** **Scoped by:** the architect.
**Origin:** ForskScope's letter of 2026-10-06 §3. The rule and the instrument are theirs.
**Semver:** none. CI and prose only.
**Release:** not scheduled. It is small and it prevents a defect class we have now hit twice.

## Why this exists

The backfill script held `latest_version = "3.3.0"` with a comment calling it *"today's actual
latest"*. True when written. By the time it ran, 3.4.0 had shipped, and the script would have
demoted 3.4.0 from `latest` — caught by reading a dry run, not by anything in the script.

The consumer had the same failure three times in one place and named the rule:

> *"a rule like **never hold a version you could read** is easier to apply than *remember to update
> this*."*

Their instrument: their threat model names our version in two present-tense sentences, and a build
gate asserts those sentences against the lockfile, so adopting a new release cannot quietly make the
document false — the gate fails before the bump lands.

**We have six of these**, all true today, which is exactly the state that precedes the defect:

| Site | Claim | Authority |
|---|---|---|
| `docs/src/non-goals.md:59,74,84` | "calamine 0.36" ×3 | `Cargo.toml`'s requirement, `calamine = "0.36"` |
| `docs/src/maintainers/performance.md:535` | "calamine 0.36.1" | `Cargo.lock`'s resolved `0.36.1` |
| `docs/src/maintainers/performance.md:539` | "calamine v0.36.1" | same |
| `docs/src/migration/v2-to-v3.md:571` | "minimum supported Rust version is still 1.88" | `Cargo.toml`'s `rust-version = "1.88.0"` |
| `.github/CONTRIBUTING.md:24` | `1.88.0` twice in the `msrv` gate command | same |
| `rfcs/accepted/037-v3-scope.md:385` | "MSRV stays 1.88" | same — **or** a scope decision, see below |

Note the two different authorities already in play: `non-goals.md` names the *requirement* and
`performance.md` names the *resolved* version. Both are legitimate and they answer to different
files.

## The line this unit must not cross

The consumer also declined to mechanise the neighbouring problem, and their reasoning is the scope
boundary here:

> *"We are **not** adding a check for it. A gate asserting that release notes describe the code is
> not mechanisable, and claiming one would be the fault we spend this register catching."*

**A version string is mechanisable because both sides are machine-readable. Prose describing
behaviour is not.** This unit gates the first and must not claim anything about the second. If you
find yourself writing a check that parses English, stop — that is the overclaim this whole register
exists to catch.

## Change scope

- `.github/scripts/` — a new check script.
- `.github/workflows/ci.yaml` — a job or a step that runs it.
- `docs/src/`, `README.md`, `.github/CONTRIBUTING.md` — rewording only where §1 of *Required
  implementation* finds a claim the gate cannot express.
- `.github/CONTRIBUTING.md`'s gate table — a row for the new check, like the others.

## Non-change scope

- **No `src/` changes. No behaviour. No API.**
- **Do not touch `rfcs/done/`, `rfcs/archive/`, `CHANGELOG.md` or `.git-exclude/`.** Those name old
  versions deliberately and correctly; a gate over them would be wrong, not merely noisy.
- Do not gate prose that describes behaviour.
- Do not bump any dependency.

## Required implementation

**1. Enumerate and classify first. This is the unit's substance.** For every version-looking token
in the gated paths, decide which of three it is:

- **a live claim** — asserts something about the current build, so the gate must check it;
- **a historical statement** — "v1 did X", "fixed in 3.1.0", "2.5.1 shipped with" — correct as
  written, must not be checked, and the migration guides are full of these;
- **ambiguous** — reads as present tense but is really a record, which `rfcs/accepted/037`'s "MSRV
  stays 1.88" may be.

**Report the three lists.** The gate can only be as good as this classification, and getting it
wrong in the noisy direction means the gate gets disabled the first time it cries wolf.

For the ambiguous ones, **propose the resolution rather than picking one silently** (rule 005,
`.git-exclude/proposal/version-gate-01/README.md`). My inclination: an RFC's scope statement is a
record of a decision and belongs outside the gate, but it should then *say* which version it
decided, not read as a live fact.

**2. Write the check.** It must:

- read the authority from the file, never from a constant — `rust-version` from `Cargo.toml`, a
  dependency requirement from `Cargo.toml`, a resolved version from `Cargo.lock`;
- distinguish the requirement from the resolved version, because §1's table shows we assert both;
- name, on failure, the file, the line, the claim and the authority it disagrees with. Someone will
  hit this during an unrelated dependency bump and needs to know in one read what to change;
- **be unable to pass by finding nothing.** Assert a minimum number of claims checked, or assert the
  known sites by name. A grep that silently matches zero lines after someone reformats a document is
  this unit's own failure mode, and it is the exact shape of the defect it exists to prevent.

**3. Prove it fails.** Three demonstrations, each reverted `cmp`-verified:

- change `rust-version` in `Cargo.toml` → the MSRV claims fail;
- change a prose version by one patch digit → that site fails;
- delete or reformat a gated line so the pattern no longer matches → **the count assertion fails**.
  The third is the one that matters.

**4. Add it to CI and to `CONTRIBUTING.md`'s gate table**, in the form the other rows use, with the
local reproduction command.

## Required tests

The script is the test. Beyond the three demonstrations above:

1. It passes on `HEAD` as it stands — all six sites are currently true, verified 2026-10-06.
2. It runs in CI on an ordinary push, not only on a tag. The defect it prevents arrives with a
   dependency bump, which is an ordinary commit.

## Acceptance criteria

1. The three classification lists, reported.
2. Ambiguous cases proposed via rule 005 and resolved before the gate lands.
3. The check reads every authority from a file; **zero version constants in the script.** If the
   script that enforces this rule holds a version, the unit has failed in the most direct way
   available.
4. Failure messages naming file, line, claim and authority.
5. A count assertion, demonstrated to fail when a gated line stops matching.
6. CI runs it on push; `CONTRIBUTING.md` documents it.
7. Gates green, rule 003, one scratch dir, deleted. Nothing committed.

## Prohibited shortcuts

- **No version literal in the check script.** Not even "for clarity".
- Do not gate `rfcs/done/`, `rfcs/archive/`, `CHANGELOG.md` or `.git-exclude/`.
- Do not write a check that parses prose for meaning.
- Do not reword a historical statement to make the gate simpler. The gate accommodates the prose, not
  the reverse — except where §1 finds a genuinely ambiguous sentence, which is a prose defect.
- Do not add an allowlist of exempt lines. If a line needs exempting, either the classification is
  wrong or the sentence is. `deny.toml`'s own comment on allowlisting to fit applies.

## Known risks

**1. The migration guides are mostly historical versions.** `docs/src/migration/` exists to describe
what older versions did. A naive gate over it will be wrong far more often than right, and the one
live claim in it (`v2-to-v3.md:571`'s MSRV sentence) sits among many correct historical ones. Expect
the classification to be the hard part.

**2. Two authorities, easily confused.** `calamine = "0.36"` is a requirement; `0.36.1` is what the
lock resolved. A gate that checks prose against the wrong one will either fail on a correct
statement or pass a stale one. `non-goals.md` and `performance.md` demonstrate both cases, so the
distinction is not hypothetical.

**3. The count assertion is the whole gate's integrity.** Without it, this becomes a check that
reports success for doing nothing — which is the category this quarter was spent deleting, installed
as the fix for it.

**4. `rfcs/accepted/037` may be out of scope and still wrong.** If "MSRV stays 1.88" is a decision
record, it should name the version it decided rather than read as a current fact. That is a prose
fix, not a gate.

## Required evidence

Under `.git-exclude/review-request/version-gate-01/evidence/`:

1. The three classification lists.
2. The proposal and my reply for the ambiguous cases.
3. The script, and a grep proving it holds no version literal.
4. The three failure demonstrations, with `cmp`-verified restores.
5. The gate passing on `HEAD`, with the number of claims it checked.
6. Gate sweep, rule 003, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/version-gate-01/README.md`, with:

- The three lists, and which sites you decided were live.
- How you told a requirement from a resolved version.
- What the count assertion asserts, and why that number.
- Anything in the prose you had to reword, and why the gate could not accommodate it as written.
