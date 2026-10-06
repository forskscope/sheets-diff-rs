# Never hold a version you could read

**Added 2026-10-06.** **One unit.** **Not scheduled** — small, and it closes a defect class rather
than fixing anything open.

The backfill script held `latest_version = "3.3.0"`, described in its own comment as *"today's actual
latest"*. By the time it ran, 3.4.0 had shipped, and it would have demoted 3.4.0 from GitHub's
`latest`. Nothing in the script could have objected; a dry run read by a human caught it.

ForskScope had the same failure three times in one check and named the rule — *never hold a version
you could read* — and the instrument: a build gate that asserts the version their prose names against
the version their lockfile resolves, so a bump cannot quietly make the document false.

**We have six prose sites naming a version, all true today**, which is precisely the state that
precedes the defect: a measurement when written, a claim thereafter.

## The boundary, which is theirs too

They declined to mechanise the neighbouring problem — a gate asserting that release notes describe
the code — on the grounds that it is not mechanisable and claiming one would be the fault their
register exists to catch. **A version string is mechanisable; prose describing behaviour is not.**
Unit 01 gates the first and is forbidden from claiming anything about the second.

## Standing rules

Rule 002 (one scratch target dir per sweep, deleted after), rule 003 (sweep both manifests),
rule 005 (propose the ambiguous classifications before gating them). Do not commit; review first.
