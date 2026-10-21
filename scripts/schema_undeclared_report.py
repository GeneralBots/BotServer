#!/usr/bin/env python3
"""Triage DB tables that have no Diesel `table!` declaration (#1525).

`schema_db_audit.py` reports the no-declaration set but does not classify it.
This tool answers the remaining question for every such table: is it accessed
through raw SQL somewhere in the Rust tree, or is it dormant (only created by a
migration and never referenced by code)?

Usage:
    python3 scripts/schema_undeclared_report.py --print-schema /tmp/print_schema.rs
    python3 scripts/schema_undeclared_report.py --print-schema /tmp/print_schema.rs \
        --markdown issues/schema/undeclared_tables.md
"""
from __future__ import annotations

import argparse
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCAN_DIRS = ["botserver/src", "botserver/crates", "botlib/src", "botui/src"]
TABLE_RE = re.compile(r"^\s{8}(\w+) \([^)]*\) \{", re.M)


def db_tables(print_schema: str) -> list[str]:
    with open(print_schema, encoding="utf-8", errors="replace") as f:
        return sorted(set(TABLE_RE.findall(f.read())))


def declared_tables() -> set[str]:
    declared: set[str] = set()
    table_re = re.compile(
        r"(?:(?:\w+)::)*(?:pub\s+)?table!\s*\{\s*(\w+)\s*\([^)]*\)", re.S
    )
    base = os.path.join(ROOT, "botschema", "src")
    for dirpath, _dirs, files in os.walk(base):
        for fn in files:
            if not fn.endswith(".rs"):
                continue
            path = os.path.join(dirpath, fn)
            try:
                text = open(path, encoding="utf-8", errors="replace").read()
            except OSError:
                continue
            text = re.sub(r"^\s*//.*$", "", text, flags=re.M)
            declared.update(table_re.findall(text))
    return declared


def string_literals(text: str) -> list[str]:
    """Return the contents of every Rust string literal (normal + raw)."""
    out: list[str] = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            j = text.find("\n", i)
            i = n if j < 0 else j + 1
            continue
        if c == "/" and i + 1 < n and text[i + 1] == "*":
            j = text.find("*/", i + 2)
            i = n if j < 0 else j + 2
            continue
        if c == "r":
            j = i + 1
            while j < n and text[j] == "#":
                j += 1
            if j < n and text[j] == '"':
                hashes = j - i - 1
                close = '"' + "#" * hashes
                k = text.find(close, j + 1)
                if k < 0:
                    break
                out.append(text[j + 1 : k])
                i = k + len(close)
                continue
        if c == '"':
            j = i + 1
            buf: list[str] = []
            while j < n:
                if text[j] == "\\" and j + 1 < n:
                    buf.append(text[j : j + 2])
                    j += 2
                    continue
                if text[j] == '"':
                    break
                buf.append(text[j])
                j += 1
            out.append("".join(buf))
            i = j + 1
            continue
        if c == "'":
            j = i + 1
            while j < n:
                if text[j] == "\\" and j + 1 < n:
                    j += 2
                    continue
                if text[j] == "'":
                    break
                j += 1
            i = j + 1
            continue
        i += 1
    return out


SQL_RE = re.compile(r"(?is)\b(select|insert\s+into|update|delete\s+from|truncate)\b")
DDL_RE = re.compile(r"(?is)\b(create\s+(?:table|index|unique\s+index)|alter\s+table)\b")


def source_references(tables: list[str]) -> dict[str, dict[str, list[str]]]:
    hits = {t: {"sql": [], "ddl": [], "ref": []} for t in tables}
    patterns = {
        t: re.compile(r"(?<![A-Za-z0-9_])" + re.escape(t) + r"(?![A-Za-z0-9_])")
        for t in tables
    }
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
                    text = open(path, encoding="utf-8", errors="replace").read()
                except OSError:
                    continue
                literals = string_literals(text)
                for table in tables:
                    if table not in text or not patterns[table].search(text):
                        continue
                    in_sql = in_ddl = False
                    for literal in literals:
                        if not patterns[table].search(literal):
                            continue
                        if DDL_RE.search(literal):
                            in_ddl = True
                        elif SQL_RE.search(literal):
                            in_sql = True
                    if in_sql:
                        hits[table]["sql"].append(rel)
                    elif in_ddl:
                        hits[table]["ddl"].append(rel)
                    else:
                        hits[table]["ref"].append(rel)
    return hits


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--print-schema", required=True)
    ap.add_argument("--markdown")
    args = ap.parse_args()

    declared = declared_tables()
    missing = [t for t in db_tables(args.print_schema) if t not in declared]
    hits = source_references(missing)
    raw_sql = [t for t in missing if hits[t]["sql"]]
    runtime_ddl = [t for t in missing if hits[t]["ddl"] and not hits[t]["sql"]]
    referenced = [
        t for t in missing if hits[t]["ref"] and not hits[t]["sql"] and not hits[t]["ddl"]
    ]
    dormant = [
        t
        for t in missing
        if not hits[t]["sql"] and not hits[t]["ddl"] and not hits[t]["ref"]
    ]

    lines = [
        "# Undeclared DB tables — triage (#1525)",
        "",
        "Generated by `scripts/schema_undeclared_report.py` from a fresh",
        "`diesel print-schema` run. These tables exist in the database but have no",
        "Diesel `table!` declaration. They are outside the epic's “declared once in",
        "`botschema`” rule because they are not accessed through Diesel's typed API.",
        "",
        f"- **raw SQL**: {len(raw_sql)} tables queried with `diesel::sql_query`",
        f"- **runtime DDL**: {len(runtime_ddl)} tables created by `CREATE TABLE` in code",
        f"- **identifier only**: {len(referenced)} tables named in docs/allowlists/APIs, never queried",
        f"- **dormant**: {len(dormant)} tables with no Rust reference (migration-only)",
        f"- **total**: {len(missing)}",
        "",
        "## Raw SQL access",
        "",
        "| Table | Referenced in |",
        "|---|---|",
    ]
    for t in raw_sql:
        refs = ", ".join(f"`{r}`" for r in hits[t]["sql"][:3])
        if len(hits[t]["sql"]) > 3:
            refs += f" (+{len(hits[t]['sql']) - 3})"
        lines.append(f"| `{t}` | {refs} |")
    lines += [
        "",
        "## Runtime DDL",
        "",
        "Created by SQL embedded in code rather than by a migration; still not a",
        "Diesel `table!` declaration.",
        "",
    ]
    lines += [f"- `{t}`" for t in runtime_ddl]
    lines += [
        "",
        "## Identifier only",
        "",
        "The name appears in documentation, API catalogs, allowlists or config, not in",
        "a query against the table.",
        "",
    ]
    lines += [f"- `{t}`: `{hits[t]['ref'][0]}`" for t in referenced]
    lines += [
        "",
        "## Dormant",
        "",
        "No Rust source references these tables; keep them only while a migration or",
        "external tool (NocoDB, reporting) still needs them.",
        "",
    ]
    lines += [f"- `{t}`" for t in dormant]
    lines.append("")

    report = "\n".join(lines)
    if args.markdown:
        with open(os.path.join(ROOT, args.markdown), "w", encoding="utf-8") as f:
            f.write(report)
        print(f"Markdown artifact -> {os.path.join(ROOT, args.markdown)}")
    else:
        print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
