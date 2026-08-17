# RFC-016: Security, Privacy, and No-Side-Effects Policy

**Status.** Implemented (2.0.0–2.4.x) — verified 2026-08-16. Both deferrals closed 2026-08-17: the `println!`/`eprintln!`/`dbg!` prohibition is a `clippy::disallowed_macros`/`clippy::disallowed_methods` gate scoped to the library target and unwaivable, enforced in CI (M5 Handoff 01, closed by Handoff 04); source-path privacy — a parent directory does not survive into a result or its rendered output, a non-UTF-8 file name yields `None` without panicking, error paths don't leak the directory, and byte/reader inputs carry no path at all — is tested (`tests/source_path_privacy.rs`, M5 Handoff 02).
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** Security/privacy  

## 1. Summary

Define security and privacy constraints for a local-first spreadsheet diff library.

## 2. Motivation

Spreadsheet files often contain confidential business data. A diff library embedded in GUI tools must not leak data, print unexpectedly, access networks, or expose absolute paths without caller intent.

## 3. Goals

- Guarantee no telemetry and no network access.
- Guarantee no stdout/stderr writes from library core.
- Avoid exposing absolute paths unless caller opts in.
- Return clear errors for unsupported/encrypted inputs.
- Respect resource bounds to reduce denial-of-service risk.

## 4. Non-goals

- Do not attempt to sanitize workbook contents for all possible output renderers.
- Do not decrypt or bypass protected files.
- Do not provide malware/macro analysis.

## 5. External design

Policy statements:

- The library performs local computation only.
- The library does not send workbook data anywhere.
- The library does not log workbook contents.
- The library does not print from core APIs.
- Source descriptions default to non-sensitive labels.
- Renderers may output workbook content because that is their explicit purpose.

## 6. Internal design

Internal code review checklist:

```text
[ ] No println!/eprintln!/dbg! in library code
[ ] No network dependencies
[ ] No background threads except explicit future feature
[ ] No absolute path in result unless provided as display name
[ ] Bounds checked in large loops
[ ] Unsupported encrypted/protected workbook returns structured error
```

Dependency policy should prefer small, well-maintained crates. `calamine` remains the workbook parser unless a future RFC changes it.

## 7. Data lifecycle

1. Caller provides source.
2. Source metadata is reduced to safe display metadata.
3. Workbook contents are read locally.
4. Comparison returns structured data.
5. Only caller-selected renderers expose contents as text/JSON.

## 8. Error, diagnostic, and edge-case behavior

Encrypted/password-protected files should return an unsupported/open error. The library should not try multiple strategies that resemble bypassing protection.

Resource-exhaustion risks are mitigated by bounds and cancellation.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Static grep/check prevents `println!`, `eprintln!`, and `dbg!` in library source.
- No network crates are added.
- Source path privacy tests pass.
- Bounds tests pass.
- Security/privacy policy appears in documentation.

## 10. Migration and compatibility

This tightens expectations for v1 users. CLI still writes output; the policy applies to library core and documented renderers.

## 11. Open questions

- Should CI include a custom lint script for forbidden macros?
- Should source display names default to file names only for path APIs, or to no name unless provided?
