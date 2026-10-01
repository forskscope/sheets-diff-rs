#!/usr/bin/env bash
# Backfill a GitHub Release for every tag that predates the `release` CI job, using the same
# extractor that job uses, so backfilled and future releases are identical in shape. See
# rfcs/handoffs/github-releases-say-2.3.0/01-generate-and-backfill.md.
#
# By default this only PRINTS the `gh` commands it would run, in ascending version order, one
# release per line -- nothing is created, edited, or published. Pass --execute to actually run
# them. This is the owner's to trigger (.git-exclude/rules/004-outward-facing-communication.md),
# never run automatically and never run by this script without that flag.
#
# `--verify-tag` on every call refuses to create a new tag from HEAD if one of these version
# strings were ever mistyped -- all sixteen tags below already exist, so this should never fire,
# but `gh release create` otherwise creates the tag itself when one is missing, which is a tag
# changed, not just a release.
#
# Usage: REPO=owner/name backfill-releases.sh [--execute]
set -euo pipefail

repo="${REPO:?set REPO=owner/name, e.g. forskscope/sheets-diff-rs}"
latest_version="3.3.0"

# Ascending version order. Every tag listed here already exists (`git tag --list`) and already
# has a CHANGELOG.md section (verified at review time) -- the sixteen this directory's README
# found with no release, per `gh api repos/.../releases/latest` returning 2.3.0 while crates.io
# serves 3.3.0.
versions=(
  2.0.0 2.0.1 2.1.0 2.2.0 2.2.1 2.2.2 2.2.3
  2.4.0 2.4.1 2.5.0 2.5.1 2.6.0 3.0.0 3.1.0 3.2.0 3.3.0
)

execute=0
if [ "${1:-}" = "--execute" ]; then
  execute=1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

for v in "${versions[@]}"; do
  body="$(mktemp)"
  "$script_dir/extract-changelog-section.sh" "$v" > "$body"

  # --latest only for 3.3.0 (today's actual latest); explicitly --latest=false for every other
  # backfilled release, rather than relying on gh's "automatic based on date and version" default
  # -- these sixteen are all created within moments of each other, so creation order is not a
  # proxy for version order here.
  if [ "$v" = "$latest_version" ]; then
    latest_flag="--latest"
  else
    latest_flag="--latest=false"
  fi

  cmd=(gh release create "$v" --repo "$repo" --verify-tag --title "$v" --notes-file "$body" "$latest_flag")
  edit_cmd=(gh release edit "$v" --repo "$repo" --verify-tag --title "$v" --notes-file "$body" "$latest_flag")

  printf '%q ' "${cmd[@]}"
  printf '|| '
  printf '%q ' "${edit_cmd[@]}"
  printf '\n'

  if [ "$execute" -eq 1 ]; then
    "${cmd[@]}" || "${edit_cmd[@]}"
  fi

  rm -f "$body"
done
