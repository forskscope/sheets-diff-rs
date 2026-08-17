# sheets-diff

Structured diff engine for Microsoft Excel `.xlsx` workbooks.

See the [README](../../README.md) for a quick start, feature table, and design
notes.

## Contents

- **[API guide](api-guide.md)** — path, reader, and bytes input; the options
  builder and resource limits; text and JSON output; error handling.
- **[Comparison semantics](semantics.md)** — five worked, run-for-real
  scenarios: typed value change, formula change, sheet rename, inserted
  row (and why `AlignmentMode` changes the answer), warning handling.
- **[Non-goals and limitations](non-goals.md)** — what this engine
  deliberately does not attempt, what is limited and why (upstream,
  deferred, or unreachable by construction), and the RFCs that shipped in
  part.
- **[Migration from v1](migration/v1-to-v2.md)** — how to update existing code
  that used v1's `Diff::new` / string cell model.
- **[Threat model](maintainers/threat-model.md)** — what this crate defends
  against, what it does not, and how each claim is checked.
