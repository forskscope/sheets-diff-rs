# RFC-001: v2 Release Scope and Compatibility Line

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
**Target:** v2.0.0 planning  
**Created:** 2026-06-11  
**Category:** Release management  

## 1. Summary

Define the boundary between the v1.2 stabilization line, the v2.0 breaking release, and later v2.x quality improvements.

## 2. Motivation

The crate has two kinds of work: immediate correctness fixes and structural API improvements. Mixing them risks either delaying urgent bug fixes or rushing a breaking public model. This RFC establishes v2 as the structured API redesign after v1.2, while keeping v2.0 small enough to ship.

## 3. Goals

- Treat v1.2 as the compatibility-preserving bug-fix release.
- Use v2.0 for the breaking public data model and API redesign.
- Define which features are mandatory for v2.0 and which are deferred to v2.x.
- Protect maintainability by rejecting Excel-complete scope expansion.
- Make downstream GUI and CLI integration requirements explicit release gates.

## 4. Non-goals

- Do not include workbook editing or merge writing in v2.0.
- Do not require row/column alignment if it endangers v2.0 quality.
- Do not preserve every v1 public struct shape.
- Do not make CLI output the canonical data model.

## 5. External design

The v2 release line is defined as follows:

```text
v1.2.x  = compatible stabilization
v2.0.0  = new structured library API
v2.1+   = optional quality and advanced matching modes
```

Mandatory v2.0 themes:

- public module cleanup;
- non-panicking fallible API;
- typed cell values;
- path/reader/bytes input;
- structured diagnostics;
- deterministic coordinates, ranges, and ordering;
- conservative sheet rename detection;
- CLI as public-API consumer;
- migration guide.

Optional v2.x themes:

- row/column alignment;
- more advanced sheet similarity algorithms;
- additional output formats;
- format support beyond `.xlsx`.

## 6. Internal design

Implementation should create a v2 branch that can break public structs without disturbing v1.2 maintenance. The public API should be reviewed as if it will remain stable for the v2 family.

Recommended branch policy:

```text
main or 1.x branch       receives v1.2 stabilization
v2-main or next branch   receives v2 RFC implementation
```

Each RFC should land behind tests. Large feature PRs should be avoided; each change should map to a small RFC or to a subtask inside an RFC.

## 7. Data lifecycle

1. Complete or accept v1.2 stabilization.
2. Open the v2 branch.
3. Introduce v2 public modules and models.
4. Port old comparison behavior into the new model.
5. Add v2-specific quality features.
6. Freeze public API before release candidates.

## 8. Error, diagnostic, and edge-case behavior

Scope errors should be handled by rejecting the change in RFC review, not by silently widening v2.0. If a feature requires new workbook writing, formula evaluation, GUI components, or external services, it is out of scope for this release line.

## 9. Testing and acceptance criteria

Acceptance criteria:

- ROADMAP-v2.md identifies v2.0 required and v2.x optional features.
- Every mandatory v2.0 feature maps to at least one RFC.
- Every deferred topic is explicitly listed.
- Release gates are documented before implementation begins.

## 10. Migration and compatibility

Existing v1 consumers should be told to use v1.2 if they need non-breaking fixes and v2.0 if they can migrate to the new model. v2 should not pretend to be a drop-in replacement.

## 11. Open questions

- Should v2.0 release candidates be published under a prerelease tag such as `2.0.0-rc.1`?
- Should v1.2 receive long-term patch support after v2.0?
