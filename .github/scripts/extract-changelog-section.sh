#!/usr/bin/env bash
# Emit CHANGELOG.md's section for one version: the text between its `## [<version>]` heading
# and the next `## [`, excluding both headings. Used by the `release` CI job (ci.yaml) and by
# backfill-releases.sh, so a GitHub Release's body always comes from the one place this project
# already keeps it current -- never hand-written. See
# rfcs/handoffs/github-releases-say-2.3.0/01-generate-and-backfill.md.
#
# A version with no section (e.g. a typo, or a tag CHANGELOG.md was never updated for) exits
# non-zero naming the version it could not find, rather than silently printing nothing: a release
# with no notes is exactly how this defect started.
#
# Usage: extract-changelog-section.sh <version> [changelog-path]
set -euo pipefail

version="${1:?usage: extract-changelog-section.sh <version> [changelog-path]}"
changelog="${2:-CHANGELOG.md}"

awk -v heading="## [$version]" -v ver="$version" '
  index($0, "## [") == 1 {
    if (printing) exit
    if (index($0, heading) == 1) { printing = 1; found = 1 }
    next
  }
  printing { print }
  END {
    if (!found) {
      print "no CHANGELOG section for version " ver > "/dev/stderr"
      exit 1
    }
  }
' "$changelog"
