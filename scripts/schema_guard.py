#!/usr/bin/env python3
"""CI guard against Diesel schema drift (issue #1520).

Rule: `table!` declarations live ONLY in the canonical `botschema` crate.
While the migration children (#1515-#1518) are still landing there are
pre-existing declarations outside `botschema`; the guard therefore enforces a
checked-in BASELINE so the count can only go down, never up.

It fails (exit 1) when:
  * a `table!` appears in a file that is not in the baseline (a NEW offender), or
  * the total number of non-canonical declarations exceeds the baseline count.

When the baseline reaches 0 the guard becomes fully blocking for any new
`table!` anywhere outside `botschema/src`.

Usage:
    python3 scripts/schema_guard.py            # check (CI mode)
    python3 scripts/schema_guard.py --update   # recompute baseline from tree
"""
import os
import re
import sys
import json

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
BASELINE_PATH = os.path.join(ROOT, "issues", "schema", "table_baseline.json")

SCAN_DIRS = [
    "botserver/src",
    "botserver/crates",
    "botlib/src",
    "botmodels/src",
    "bottest/src",
]

TOKEN = r"[A-Za-z_][A-Za-z0-9_]*"
TABLE_RE = re.compile(r"(?:(?:" + TOKEN + r")::)*(?:pub\s+)?table!\s*\{")


def scan():
    """Return {relfile: count} of non-canonical table! declarations."""
    per_file = {}
    for d in SCAN_DIRS:
        base = os.path.join(ROOT, d)
        if not os.path.isdir(base):
            continue
        for dirpath, _dirs, files in os.walk(base):
            for fn in files:
                if not fn.endswith(".rs"):
                    continue
                path = os.path.join(dirpath, fn)
                rel = os.path.relpath(path, ROOT)
                try:
                    with open(path, "r", encoding="utf-8", errors="replace") as f:
                        text = f.read()
                except OSError:
                    continue
                n = sum(1 for m in TABLE_RE.finditer(text))
                if n:
                    per_file[rel] = n
    return per_file


def canonical_duplicates():
    """Return {table: [files]} for tables declared more than once in botschema."""
    import glob as _glob

    pattern = re.compile(r"table!\s*\{\s*(\w+)\s*\(")
    seen = {}
    base = os.path.join(ROOT, "botschema", "src")
    for path in _glob.glob(os.path.join(base, "**", "*.rs"), recursive=True):
        try:
            text = open(path, encoding="utf-8", errors="replace").read()
        except OSError:
            continue
        text = re.sub(r"^\s*//.*$", "", text, flags=re.M)
        rel = os.path.relpath(path, ROOT)
        for table in pattern.findall(text):
            seen.setdefault(table, []).append(rel)
    return {t: f for t, f in seen.items() if len(f) > 1}


def load_baseline():
    if not os.path.exists(BASELINE_PATH):
        return None
    with open(BASELINE_PATH) as f:
        return json.load(f)


def main():
    per_file = scan()
    total = sum(per_file.values())
    files = sorted(per_file.keys())

    if "--update" in sys.argv:
        os.makedirs(os.path.dirname(BASELINE_PATH), exist_ok=True)
        with open(BASELINE_PATH, "w") as f:
            json.dump(
                {"total": total, "files": files}, f, indent=2, sort_keys=True
            )
        print(f"baseline updated: total={total} files={len(files)}")
        return 0

    duplicates = canonical_duplicates()
    if duplicates:
        print(
            "schema_guard: FAIL — tables declared more than once inside botschema:"
        )
        for table, files in sorted(duplicates.items()):
            print(f"  {table}: {', '.join(files)}")
        return 1

    baseline = load_baseline()
    if baseline is None:
        print(
            "schema_guard: no baseline found; run `--update` first "
            f"(current total={total})"
        )
        return 1

    base_total = baseline["total"]
    base_files = set(baseline["files"])

    if total != base_total or set(files) != base_files:
        # A baseline that no longer matches the tree is stale: a legitimate
        # reduction must refresh it in the same commit, and an increase must be
        # reverted instead of absorbed by old slack.
        if total <= base_total and set(files) <= base_files:
            print(
                f"schema_guard: FAIL — baseline is stale (tree has {total} "
                f"declarations, baseline says {base_total}); "
                "run `python3 scripts/schema_guard.py --update`"
            )
            return 1

    if total > base_total:
        new_files = sorted(set(files) - base_files)
        print(
            f"schema_guard: FAIL — table! declarations outside botschema rose "
            f"from {base_total} to {total}"
        )
        if new_files:
            print("New offending files:")
            for nf in new_files:
                print(f"  {nf}: {per_file[nf]}")
        print(
            "Move these `table!` blocks into the botschema crate "
            "(see AGENTS.md \"Diesel schema ownership\")."
        )
        return 1

    new_files = sorted(set(files) - base_files)
    if new_files:
        print(
            f"schema_guard: FAIL — new files declare table! outside botschema "
            f"(total {total} <= baseline {base_total} but new offenders):"
        )
        for nf in new_files:
            print(f"  {nf}: {per_file[nf]}")
        return 1

    print(
        f"schema_guard: OK — {total} non-canonical table! declarations "
        f"(baseline {base_total}); no new offenders."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
