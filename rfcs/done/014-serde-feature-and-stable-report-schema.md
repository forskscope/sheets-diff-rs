# RFC-014: Serde Feature and Stable Report Schema

**Status.** Implemented as scoped (2.0.0–2.4.x) — verified 2026-08-16; the `Deserialize` deferral **closed as declined** 2026-08-17. Only `Serialize` ships on public model types. `Deserialize` is not planned: the only known consumer was asked directly and declined it, having no read path for anything they serialise and noting that a read path would require a stable format rather than a derive. Revisit only on a concrete request.
**Target:** v2.0.0 recommended optional feature  
**Created:** 2026-06-11  
**Category:** Serialization  

## 1. Summary

Provide optional serialization for structured diff reports while defining a schema stability policy.

## 2. Motivation

Application developers and test tools benefit from JSON snapshots, cached diff reports, and process boundaries. Serialization is useful, but it can accidentally freeze internals. This RFC makes serialization explicit and feature-gated.

## 3. Goals

- Support `serde` derives behind a feature flag.
- Define stable report schema expectations.
- Avoid serializing private/internal fields.
- Make JSON useful for tests and CLI integrations.
- Support forward-compatible consumers where practical.

## 4. Non-goals

- Do not require serde for minimal library use.
- Do not promise binary serialization stability.
- Do not expose internal calamine types in serialized output.

## 5. External design

Feature flag:

```toml
[features]
default = []
serde = ["dep:serde"]
json = ["serde", "dep:serde_json"]
```

Schema policy:

- v2.x may add fields only if they have defaults or are documented as optional.
- v2.x should not rename or remove serialized fields without a major version.
- enums intended for serialized output should have explicit names.
- `#[serde(other)]` or equivalent compatibility should be considered for non-critical enums.

## 6. Internal design

Public model derives:

```rust
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WorkbookDiff { ... }
```

If some fields are not stable enough, provide separate report DTOs:

```rust
pub struct WorkbookDiffReport { ... }
impl From<&WorkbookDiff> for WorkbookDiffReport { ... }
```

The DTO approach is more work but provides better schema control.

## 7. Data lifecycle

1. Comparison returns `WorkbookDiff`.
2. If serde is enabled, caller serializes directly or converts to report DTO.
3. CLI JSON output uses the same path.
4. Golden JSON fixtures validate schema stability.

## 8. Error, diagnostic, and edge-case behavior

Serialization should never include absolute paths unless the caller provided them as display names.

Deserialization should not be required for v2.0 unless there is a clear use case. Serialization-only may be enough for reports.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Crate builds with default features and no serde.
- Crate builds with `serde` feature.
- JSON report examples are stable in fixtures.
- Serialized output does not include hidden source paths.
- Schema policy is documented.

## 10. Migration and compatibility

v1 had no stable structured report schema. Migration docs should present JSON as new v2 functionality, not a compatibility promise with v1 text output.

## 11. Open questions

- Should the crate serialize the core model directly or define separate report DTOs?
- Is deserialization necessary in v2.0?
