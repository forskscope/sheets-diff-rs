#!/usr/bin/env python3
"""Gate the versions our prose names against the files that decide them.

Rule: never hold a version you could read (never-hold-a-version-you-could-read/01).
A version string is checkable because both sides are machine-readable. This script checks only that. It does
NOT check that prose describes behaviour correctly, which cannot be mechanised, and it must never grow a check
that parses English for meaning.

Every authority is read from a file at run time: Cargo.toml (rust-version, a dependency requirement) or
Cargo.lock (a resolved version). This script holds no version number.

Two authorities are kept distinct, because a claim must be checked against the right one:
  * a REQUIREMENT is what Cargo.toml asks for (the `calamine` entry in `[dependencies]`);
  * a RESOLVED version is what Cargo.lock fixed (the `version` of a `[[package]]` entry).

SITES below is the list of claims. Each site declares how many matches it must have, so the gate cannot pass by
finding nothing: if a document is reformatted so a pattern stops matching, that site's count fails.

RULE FOR THIS LIST: a site may be removed only as a reviewed decision recorded beside the change, never to get
back to green. If a reformat breaks the gate, update that site's pattern. Do not delete the site, and do not add
an allowlist of exempt lines.

Not gated, on purpose: rfcs/done, rfcs/archive, CHANGELOG.md, .git-exclude (they name old versions correctly),
rfcs/accepted (dated records), and historical statements in the migration guides.

A note on Cargo.toml and deny.toml sites: the COMMENT is the claim, and the dependency table plus the lockfile
are the authority. That can look circular. It is not.

Usage: python3 .github/scripts/check-version-claims.py   (run from anywhere inside the repository)
"""
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())


def read(path):
    return (ROOT / path).read_text(encoding="utf-8")


CARGO = tomllib.loads(read("Cargo.toml"))
LOCK = tomllib.loads(read("Cargo.lock"))["package"]


def fail_authority(msg):
    print(f"error: cannot read an authority: {msg}")
    sys.exit(2)


def requirement(table, name):
    for section in ("dependencies", "dev-dependencies"):
        dep = CARGO.get(section, {}).get(name)
        if dep is not None:
            return (dep if isinstance(dep, str) else dep["version"]).lstrip("=^~")
    fail_authority(f"Cargo.toml has no dependency `{name}`")


def package(name):
    found = [p for p in LOCK if p["name"] == name]
    if len(found) != 1:
        fail_authority(f"Cargo.lock has {len(found)} entries for `{name}`, expected exactly one")
    return found[0]


def edge(parent, child):
    """The version of `child` that `parent`'s lock entry depends on, read from the dependency edge."""
    for dep in package(parent).get("dependencies", []):
        parts = dep.split()
        if parts[0] == child:
            if len(parts) > 1:
                return parts[1]
            return package(child)["version"]  # a bare name means the lock holds only one `child`
    fail_authority(f"Cargo.lock: `{parent}` has no dependency edge to `{child}`")


# Each authority: (description for the failure message, function returning the version string).
AUTH = {
    "msrv": ("Cargo.toml package.rust-version", lambda: CARGO["package"]["rust-version"]),
    "calamine requirement": ("Cargo.toml `calamine` requirement", lambda: requirement(None, "calamine")),
    "calamine resolved": ("Cargo.lock `calamine`", lambda: package("calamine")["version"]),
    # ZIP: the lock holds two `zip` entries (one dev-only, one via calamine), so "the zip version" is
    # ambiguous. Both are resolved through the dependency edge of the crate that wants them, never by picking.
    "zip via calamine": ("Cargo.lock edge calamine -> zip", lambda: edge("calamine", "zip")),
    "zip dev-only via rust_xlsxwriter": (
        "Cargo.lock edge rust_xlsxwriter -> zip",
        lambda: edge("rust_xlsxwriter", "zip"),
    ),
    "quick-xml via calamine": ("Cargo.lock edge calamine -> quick-xml", lambda: edge("calamine", "quick-xml")),
    "benchmark pin": ("Cargo.toml `sheets-diff-v1_2` requirement", lambda: requirement(None, "sheets-diff-v1_2")),
}

V3 = r"(\d+\.\d+\.\d+)"
V2 = r"(\d+\.\d+)"

# (file, per-line pattern with ONE capture group, authority, components the claim must have, matches required)
SITES = [
    ("docs/src/non-goals.md", rf"calamine {V2}(?!\.\d)", "calamine requirement", 2, 3),
    ("docs/src/maintainers/performance.md", rf"calamine {V3}", "calamine resolved", 3, 1),
    ("docs/src/maintainers/performance.md", rf"calamine v{V3}", "calamine resolved", 3, 1),
    ("docs/src/migration/v2-to-v3.md", rf"minimum supported Rust version is still {V2}\b", "msrv", 2, 1),
    ("README.md", rf"`calamine` {V2}\.x", "calamine requirement", 2, 1),
    ("README.md", rf'caret range \(`"{V2}"`\)', "calamine requirement", 2, 1),
    (".github/CONTRIBUTING.md", rf"(?:install|run) {V3}\b", "msrv", 3, 3),
    ("docs/src/maintainers/threat-model.md", rf"`zip` {V3}", "zip via calamine", 3, 1),
    ("docs/src/maintainers/threat-model.md", rf"`calamine` {V2}$", "calamine requirement", 2, 1),
    ("docs/src/maintainers/threat-model.md", rf"`quick-xml` to {V3}", "quick-xml via calamine", 3, 1),
    ("Cargo.toml", rf"single `calamine {V3}` already here", "calamine resolved", 3, 1),
    ("Cargo.toml", rf"zip {V2}/\d+\.\d+", "zip dev-only via rust_xlsxwriter", 2, 1),
    ("Cargo.toml", rf"zip \d+\.\d+/{V2}", "zip via calamine", 2, 1),
    ("Cargo.toml", rf'\(version = "{V2}", features = \["deflate"\]', "zip via calamine", 2, 1),
    ("Cargo.toml", rf"already-resolved {V3} with", "zip via calamine", 3, 1),
    ("deny.toml", rf"`zip` {V3} \(dev-only", "zip dev-only via rust_xlsxwriter", 3, 1),
    ("deny.toml", rf"`zip` {V3} \(via `calamine`", "zip via calamine", 3, 1),
    ("deny.toml", rf"^# {V2}, `calamine` pins", "zip dev-only via rust_xlsxwriter", 2, 1),
    ("deny.toml", rf"`calamine` pins {V2},", "zip via calamine", 2, 1),
    ("deny.toml", rf"`= {V3}`", "benchmark pin", 3, 2),
    ("deny.toml", rf"Being {V3} is the entire purpose", "benchmark pin", 3, 1),
]


def main():
    problems = []
    checked = 0
    for file, pattern, auth_name, parts, required in SITES:
        desc, get = AUTH[auth_name]
        authority = get()
        found = []
        for number, line in enumerate(read(file).splitlines(), 1):
            for m in re.finditer(pattern, line):
                found.append((number, m.group(1)))
        if len(found) != required:
            problems.append(
                f"{file}: pattern /{pattern}/ matched {len(found)} time(s), expected {required}. "
                f"If the document was reformatted, update this site's pattern in SITES; do not delete the site."
            )
            continue
        for number, claim in found:
            checked += 1
            ok = len(claim.split(".")) == parts and authority.split(".")[:parts] == claim.split(".")
            if not ok:
                problems.append(
                    f"{file}:{number}: names `{claim}` ({parts} components) but {desc} says `{authority}`. "
                    f"Update the prose to match the authority."
                )
    if problems:
        print("version claims out of date:")
        for p in problems:
            print("  " + p)
        sys.exit(1)
    print(f"ok: {checked} version claims checked across {len(SITES)} declared sites")


main()
