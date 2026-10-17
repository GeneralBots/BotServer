#!/usr/bin/env python3
"""Inventory of Diesel table! declarations across the workspace (issue #1514).

Walks the Rust sources (excluding the canonical botschema crate), parses every
`table! { name (pk) { col -> Type, ... } }` block, groups them by table name and
classifies each duplicate group as:
  - identical      : byte-for-byte same column list (+types) and same pk
  - additive       : one declaration is a strict superset of another (extra cols)
  - conflicting    : columns present in one but not the other, or same col with
                     different types
Produces JSON (raw) + a markdown summary.
"""
import os
import re
import json
import sys
from collections import defaultdict

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

# Directories to scan (canonical crate excluded).
SCAN_DIRS = [
    "botserver/src",
    "botserver/crates",
    "botlib/src",
    "botmodels/src",
    "bottest/src",
]

# The canonical schema crate -- excluded from "duplicate" counts but still parsed
# so we can note which tables the canonical source already declares.
CANONICAL_DIRS = ["botschema/src"]

TOKEN = r"[A-Za-z_][A-Za-z0-9_]*"

# Match:  [path::]table! { name (pk) { ... } }
# We capture the balanced body by scanning braces.
TABLE_RE = re.compile(
    r"(?:(?:" + TOKEN + r")::)*(?:pub\s+)?table!\s*\{"
)


def find_blocks(text):
    """Yield the inner text of every balanced `table! { ... }` block."""
    for m in TABLE_RE.finditer(text):
        start = m.end()  # position just after '{'
        depth = 1
        i = start
        while i < len(text) and depth > 0:
            c = text[i]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            i += 1
        inner = text[start : i - 1]
        yield inner, m.start()


def parse_table_block(inner):
    """Parse `name (pk) { col -> Type, ... }` -> dict or None."""
    # Comments may sit between `table! {` and the table name; without skipping
    # them the header regex below fails and the whole declaration is dropped
    # from the report (silent undercount).
    inner = re.sub(r"^(?:\s*//[^\n]*|\s*/\*.*?\*/)+", "", inner, flags=re.S).strip()
    hdr = re.match(
        r"(" + TOKEN + r")\s*\(([^)]*)\)\s*\{", inner
    )
    if not hdr:
        return None
    name = hdr.group(1)
    pk = hdr.group(2).strip()
    body_start = hdr.end()
    # find matching close brace of the columns block
    depth = 1
    i = body_start
    while i < len(inner) and depth > 0:
        c = inner[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                break
        i += 1
    body = inner[body_start:i]
    # Drop diesel attribute lines (e.g. `#[sql_name = "type"]`) that precede a
    # column; they would otherwise hide the column from the token match below.
    body = re.sub(r"#\[[^\]]*\]", " ", body)
    cols = []
    for line in body.split(","):
        line = line.strip()
        if not line:
            continue
        line = re.sub(r"^\(\s*", "", line)  # some decls wrap like (col -> Type)
        cm = re.match(r"(" + TOKEN + r")\s*->\s*(.+)$", line)
        if cm:
            col = cm.group(1).strip()
            ty = re.sub(r"\s+", "", cm.group(2))
            # strip trailing comments
            ty = ty.split("//")[0].strip()
            cols.append((col, ty))
    return {"table": name, "pk": pk, "cols": cols}


def scan(dirs):
    decls = defaultdict(list)  # table -> list of occurrences
    for d in dirs:
        base = os.path.join(ROOT, d)
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
                for inner, pos in find_blocks(text):
                    parsed = parse_table_block(inner)
                    if not parsed:
                        continue
                    line = text.count("\n", 0, pos) + 1
                    parsed["file"] = rel
                    parsed["line"] = line
                    decls[parsed["table"]].append(parsed)
    return decls


def _colmap(o):
    return {c: t for c, t in o["cols"]}


def classify(occurrences, canonical=None):
    """Classify a group of declarations of the same table.

    `canonical` is the botschema declaration (or None). When present it
    participates in the comparison: a table whose consumer copies agree with
    each other but *differ from botschema* is NOT identical — botschema itself
    drifted (older/smaller shape or different column types).
    """
    if len(occurrences) == 1 and canonical is None:
        return "unique"
    maps = [_colmap(o) for o in occurrences]
    pks = {o["pk"] for o in occurrences}
    if canonical is not None:
        maps = maps + [_colmap(canonical)]
        pks = pks | {canonical["pk"]}

    # type conflict: same column with different types anywhere in the set
    allkeys = set()
    for m in maps:
        allkeys |= set(m.keys())
    for col in allkeys:
        types = {m[col] for m in maps if col in m}
        if len(types) > 1:
            return "conflicting"

    # identical: every map equal (and same pk)
    base = maps[0]
    if all(m == base for m in maps) and len(pks) == 1:
        return "identical"

    # additive: one map is a superset of all others (extra columns only)
    superset_exists = any(set(m.keys()) == allkeys for m in maps)
    if superset_exists:
        return "additive"

    # columns differ but no single superset -> conflicting
    return "conflicting"


def main():
    decls = scan(SCAN_DIRS)
    canon = scan(CANONICAL_DIRS)
    # first canonical declaration per table (botschema should declare each once)
    canon_first = {t: v[0] for t, v in canon.items() if v}

    report = {
        "distinct_tables": len(decls),
        "total_declarations": sum(len(v) for v in decls.values()),
        "duplicated_tables": 0,
        "identical": [],
        "additive": [],
        "conflicting": [],
        "unique": [],
        "canonical": {t: len(v) for t, v in canon.items()},
        "not_in_canonical": [],
        "details": {},
    }

    for table, occs in sorted(decls.items()):
        cls = classify(occs, canon_first.get(table))
        report["details"][table] = {
            "classification": cls,
            "count": len(occs),
            "canonical": (
                {"cols": {c: t for c, t in canon_first[table]["cols"]}}
                if table in canon_first
                else None
            ),
            "occurrences": [
                {
                    "file": o["file"],
                    "line": o["line"],
                    "pk": o["pk"],
                    "cols": {c: t for c, t in o["cols"]},
                }
                for o in occs
            ],
        }
        if cls == "unique":
            report["unique"].append(table)
        elif cls in ("identical", "additive", "conflicting"):
            report["duplicated_tables"] += 1
            report[cls].append(table)
        if table not in canon:
            report["not_in_canonical"].append(table)

    out_json = "/tmp/schema_drift_inventory.json"
    with open(out_json, "w") as f:
        json.dump(report, f, indent=2, sort_keys=True)

    print(f"distinct_tables        = {report['distinct_tables']}")
    print(f"total_declarations     = {report['total_declarations']}")
    print(f"duplicated_tables      = {report['duplicated_tables']}")
    print(f"  identical            = {len(report['identical'])}")
    print(f"  additive             = {len(report['additive'])}")
    print(f"  conflicting          = {len(report['conflicting'])}")
    print(f"unique (1 decl only)   = {len(report['unique'])}")
    print(f"canonical(botschema)   = {len(report['canonical'])} tables")
    print(f"not_in_canonical       = {len(report['not_in_canonical'])}")
    print()
    print("Top duplicated tables:")
    dups = sorted(
        ((t, len(o)) for t, o in decls.items() if len(o) > 1),
        key=lambda x: -x[1],
    )
    for t, n in dups[:20]:
        cls = report["details"][t]["classification"]
        print(f"  {t:32} x{n:<3} [{cls}]")
    print(f"\nRaw JSON -> {out_json}")

    # write a compact markdown artifact too
    write_markdown(report)


def write_markdown(report):
    L = []
    L.append("# Schema drift inventory (#1514)")
    L.append("")
    L.append("Generated by `scripts/schema_drift_inventory.py` (checked-in artifact; "
             "regenerate with `python3 scripts/schema_drift_inventory.py`).")
    L.append("")
    L.append("The canonical source is the `botschema` crate. Everything below is a "
             "`table!` declared *outside* `botschema`, grouped by table and classified:")
    L.append("")
    L.append("- **identical** — byte-for-byte same columns/types + same PK. Safe to replace with `pub use botschema::…` (#1515).")
    L.append("- **additive** — one declaration is a strict superset (extra columns only). Adopt the superset (#1516).")
    L.append("- **conflicting** — columns or column types disagree. Needs a reconciliation decision + possibly a migration (#1517).")
    L.append("- **unique** — declared once, outside `botschema`; should migrate into the crate but is not a duplicate.")
    L.append("")
    L.append("| Metric | Value |")
    L.append("|---|---:|")
    L.append(f"| Distinct tables (outside botschema) | {report['distinct_tables']} |")
    L.append(f"| Total `table!` declarations | {report['total_declarations']} |")
    L.append(f"| Tables declared more than once | {report['duplicated_tables']} |")
    L.append(f"| — byte-identical duplicates | {len(report['identical'])} |")
    L.append(f"| — additive drift (superset) | {len(report['additive'])} |")
    L.append(f"| — conflicting drift | {len(report['conflicting'])} |")
    L.append(f"| Tables already in `botschema` | {len(report['canonical'])} |")
    L.append(f"| Distinct tables NOT yet in `botschema` | {len(report['not_in_canonical'])} |")
    L.append("")

    def section(title, tables, note=""):
        L.append(f"## {title} ({len(tables)})")
        L.append("")
        if note:
            L.append(note)
            L.append("")
        for t in tables:
            d = report["details"][t]
            inb = "in botschema" if t in report["canonical"] else "**not** in botschema"
            L.append(f"### `{t}` ×{d['count']} ({inb})")
            L.append("")
            # union of columns
            cols = {}
            for o in d["occurrences"]:
                for c, ty in o["cols"].items():
                    cols.setdefault(c, set()).add(ty)
            L.append("| column | type(s) |")
            L.append("|---|---|")
            for c in sorted(cols):
                tys = ", ".join(sorted(cols[c]))
                flag = " ⚠" if len(cols[c]) > 1 else ""
                L.append(f"| `{c}` | `{tys}`{flag} |")
            L.append("")
            L.append("| file | line | pk | #cols |")
            L.append("|---|---|---|---:|")
            for o in d["occurrences"]:
                L.append(f"| `{o['file']}` | {o['line']} | `{o['pk']}` | {len(o['cols'])} |")
            L.append("")

    section("Conflicting tables", report["conflicting"],
            "Columns or types disagree across declarations; #1517 must pick the canonical shape "
            "(⚠ marks a column typed differently in different crates).")
    section("Additive tables", report["additive"],
            "One declaration is a superset; adopt the widest column set (#1516).")
    section("Identical tables", report["identical"],
            "Safe mechanical replacement with `pub use botschema::…` (#1515).")

    out = os.path.join(ROOT, "issues", "schema", "schema_drift.md")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w") as f:
        f.write("\n".join(L))
    print(f"Markdown artifact -> {out}")


if __name__ == "__main__":
    main()
