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

# Which release GitHub currently calls `latest`, asked rather than assumed. This was once the
# literal string "3.3.0", written when 3.3.0 was the newest version in existence. By the time the
# backfill actually ran, 3.4.0 had shipped and had its own release from the tag-gated workflow --
# so the hardcoded value would have passed `--latest` for 3.3.0 and silently demoted 3.4.0. A
# constant naming "today's latest" stops being true the next time anything ships.
#
# `latest` on GitHub is a property, not a derivation, so it has to be set deliberately for exactly
# one release and left alone for the rest. The rule below: pass `--latest` only if the newest
# release this script is creating is also newer than whatever is latest now; otherwise every
# backfilled release gets an explicit `--latest=false` and the existing latest is untouched.
latest_now="$(gh api "repos/$repo/releases/latest" --jq .tag_name 2>/dev/null || echo "")"

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

# The highest version this run would create, by version sort rather than array position.
newest_in_set="$(printf '%s\n' "${versions[@]}" | sort -V | tail -1)"

for v in "${versions[@]}"; do
  body="$(mktemp)"
  "$script_dir/extract-changelog-section.sh" "$v" > "$body"

  # Explicit `--latest=false` on every backfilled release, rather than relying on gh's "automatic
  # based on date and version" default -- these are all created within moments of each other, so
  # creation order is no proxy for version order. `--latest` is passed only when this script is
  # creating something newer than the release GitHub currently calls latest, which it is not when
  # a newer version has already shipped and published its own release.
  latest_flag="--latest=false"
  if [ -n "$latest_now" ] && [ "$v" = "$newest_in_set" ]; then
    # Highest of the two by version sort; if that is ours, we may claim latest.
    highest="$(printf '%s\n%s\n' "$v" "$latest_now" | sort -V | tail -1)"
    if [ "$highest" = "$v" ] && [ "$v" != "$latest_now" ]; then
      latest_flag="--latest"
    fi
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
