# Handoff 01 — Reconstruct RFC-033

**Governing RFC.** [RFC 000](../../done/000-rfc-lifecycle-policy.md)
**Roadmap.** M3, track B
**Sequence.** After track A. Precedes status verification (unit 02).

## Purpose

Make twenty normative citations true.

## Background

`src/` cites **RFC-033** at 20 sites across `model.rs`, `options.rs`,
`error.rs`, `diff.rs`, `normalize.rs`, `meta.rs` and `lib.rs`. `model.rs` opens
with *"All types here are normatively defined in RFC-033"* and calls it the
canonical lexicon. `lib.rs` attributes the public entry points to "RFC-033 §12".

**No copy exists** — not in `rfcs/`, not in git history, not in the v2 planning
package, which stops at 032. It was written, cited, and lost before this
repository ever held it.

A dangling normative reference is a defect whichever way it is resolved. There
are two legitimate resolutions and **you must choose and justify one**:

- **Reconstruct it** from the code that cites it, recovering the numbering the
  citations already use.
- **De-cite** — remove the references and let the code stand on its own, or
  redirect them to the RFCs that actually survive (007 for typed values, 003 for
  the result model, 005 for diagnostics, and so on).

Reconstruction preserves twenty working references and gives the public model a
stated authority. De-citing is cheaper and honest, but concedes that the model
has no normative source beyond the code.

## Change scope

Either `rfcs/done/033-<slug>.md` plus `rfcs/README.md`, **or** the citation
sites in `src/` plus a note recording the decision. Not both.

## Non-change scope

Do **not** change behaviour, the public API, or any type. If reconstruction
tempts you to "fix" something so the document reads better, stop — the document
describes what exists.

## Required implementation

### If reconstructing

Recover it **from the citations**, not from imagination. Each of the 20 sites
tells you what RFC-033 was expected to define; several name sections (`§1`
mapping table, `§2`–`§3` cell values, `§5` cell change model, `§6` sheet change,
`§8` diagnostics, `§9` errors, `§10` limits, `§11` options, `§12` entry points
and top-level result). That is most of a table of contents, recovered from
evidence.

Where a citation implies a decision the code no longer reflects, **record the
divergence rather than describing the code as if it were the design.** Those
divergences are findings.

Number and place it per RFC 000: `033-<slug>.md` in `done/`, since what it
describes has shipped. Its Status must say what it is — a reconstruction, dated,
recovered from citations — not imply it is the original.

### If de-citing

Replace each reference with one that resolves, or delete it. Record the decision
somewhere durable so nobody reconstructs it later believing the citations were
merely broken links.

## Required tests

None; documentation. If de-citing touches `src/`, the full matrix must stay
green and no golden may move.

## Acceptance criteria

1. Zero dangling RFC-033 references remain — verify with a grep, showing the
   count go to zero or resolve to a real file.
2. The choice between reconstruction and de-citing is stated with its reasoning.
3. If reconstructed: every section the citations imply exists; every divergence
   between citation and code is recorded as a finding.
4. `rfcs/README.md` reflects the outcome.
5. Nothing under `src/` changes behaviour.

## Prohibited shortcuts

- Do not invent design rationale. If you cannot tell what a decision was, say
  the reconstruction is incomplete at that point.
- Do not write it as though it were the lost original. It is a reconstruction
  and must say so.
- Do not silently drop a citation that is inconvenient to satisfy.

## Known risks

Reconstruction may reveal that code and cited design disagree. That is a
valuable outcome, not an obstacle — report each instance.

## Required evidence

- The grep before and after
- The chosen resolution and its reasoning
- Any citation/code divergence found

## Review request format

Per development policy §9.2, plus the resolution choice and its reasoning.
