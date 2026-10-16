#!/usr/bin/env python3
"""Compare the committed `table!` declarations against a live database (#1519).

The migrations are not the only writer of the schema, so `diesel print-schema`
against a FRESH database is the authority.  This tool takes that output (or any
file of Diesel `table!` declarations) and diffs every declaration in the tree
against it, per table:

  * columns the database has and the declaration does not (missing)
  * columns the declaration has and the database does not (ghost)
  * column type differences (`Varchar` vs `Nullable<Varchar>`, `Int4` vs `Int8`)
  * primary-key differences
  * tables the database has with no declaration, and vice versa

Usage:
    # fresh DB, then:
    (cd botserver && diesel migration run)
    diesel print-schema --schema public > /tmp/print_schema.rs
    python3 scripts/schema_db_audit.py --print-schema /tmp/print_schema.rs
    python3 scripts/schema_db_audit.py --print-schema /tmp/print_schema.rs --table users

Exit status: 0 when every declaration matches the database, 1 otherwise.
"""
from __future__ import annotations

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# `diesel::table! { name (pk) { … } }`, `table! { … }`, `pub table! { … }` and
# `botschema::table! { … }` all occur in the tree.
TABLE_RE = re.compile(
    r"(?:(?:\w+)::)*(?:pub\s+)?table!\s*\{\s*(\w+)\s*"
    r"\(([^)]*)\)\s*\{(.*?)\n\s*\}\s*\n\s*\}",
    re.S,
)
COLUMN_RE = re.compile(r"^\s*(?P<name>[\w\".]+)\s*->\s*(?P<ty>.+?),?\s*$")
SQL_NAME_RE = re.compile(r'#\[\s*sql_name\s*=\s*"([^"]+)"\s*\]')

# Diesel accepts several spellings for the same SQL type; compare canonically so
# `Integer` vs `Int4` is not reported as drift.
TYPE_ALIASES = {
    "Integer": "Int4",
    "BigInt": "Int8",
    "SmallInt": "Int2",
    "Double": "Float8",
    "Real": "Float4",
    "Boolean": "Bool",
}


def canonical_type(ty: str) -> str:
    """Normalize a Diesel type spelling (alias names and Nullable wrapping)."""
    ty = re.sub(r"\s+", "", ty)
    inner = None
    if ty.startswith("Nullable<") and ty.endswith(">"):
        inner, ty = ty[len("Nullable<") : -1], None
    if ty is None:
        return f"Nullable<{canonical_type(inner)}>"
    if ty.startswith("Array<") and ty.endswith(">"):
        return f"Array<{canonical_type(ty[6:-1])}>"
    return TYPE_ALIASES.get(ty, ty)


def parse_declarations(text: str):
    """Return {table: {"pk": [...], "columns": {name: type}}} for Rust source."""
    tables: dict[str, dict] = {}
    for match in TABLE_RE.finditer(re.sub(r"^\s*//.*$", "", text, flags=re.M)):
        table, pk_raw, body = match.group(1), match.group(2), match.group(3)
        columns: dict[str, str] = {}
        pending_sql_name = None
        for raw_line in body.splitlines():
            line = raw_line.strip()
            if not line:
                continue
            attr = SQL_NAME_RE.search(line)
            if attr:
                pending_sql_name = attr.group(1)
                continue
            col = COLUMN_RE.match(raw_line)
            if not col:
                continue
            name = pd = col.group("name")
            if pending_sql_name:
                pd = pending_sql_name.rstrip()
                pending_sql_name = None
            name = pd.strip().lstrip('"').replace("r#", "")
            columns[name] = canonical_type(col.group("ty").rstrip(","))
        pk = [p.strip().replace("r#", "") for p in pk_raw.split(",") if p.strip()]
        tables[table] = {"pk": pk, "columns": columns}
    return tables


def scan_tree():
    """Return {path: parsed} for every non-canonical declaration in the tree."""
    out: dict[str, dict] = {}
    for rel_root in ("botserver/src", "botserver/crates", "botlib/src", "botmodels/src"):
        base = os.path.join(ROOT, rel_root)
        if not os.path.isdir(base):
            continue
        for dirpath, _dirs, files in os.walk(base):
            for fn in files:
                if not fn.endswith(".rs"):
                    continue
                path = os.path.join(dirpath, fn)
                with open(path, encoding="utf-8", errors="replace") as fh:
                    text = fh.read()
                if "table!" not in text:
                    continue
                parsed = parse_declarations(text)
                if parsed:
                    out[os.path.relpath(path, ROOT)] = parsed
    return out


def collect_declarations():
    """Return {table: [(path, decl, is_canonical)]} for the whole tree."""
    declared: dict[str, list[tuple[str, dict, bool]]] = {}
    for path, tables in scan_tree().items():
        for table, decl in tables.items():
            declared.setdefault(table, []).append((path, decl, False))
    for dirpath, _dirs, files in os.walk(os.path.join(ROOT, "botschema")):
        for fn in files:
            if not fn.endswith(".rs"):
                continue
            path = os.path.join(dirpath, fn)
            with open(path, encoding="utf-8", errors="replace") as fh:
                parsed = parse_declarations(fh.read())
            for table, decl in parsed.items():
                declared.setdefault(table, []).append(
                    (os.path.relpath(path, ROOT), decl, True)
                )
    return declared


def diff_table(db_table, copies):
    """Return per-copy problem tuples for one table."""
    problems = []
    db_cols = db_table["columns"]
    for path, decl, is_canonical in copies:
        missing = [c for c in db_cols if c not in decl["columns"]]
        ghosts = [c for c in decl["columns"] if c not in db_cols]
        wrong_type = [
            (c, decl["columns"][c], db_cols[c])
            for c in decl["columns"]
            if c in db_cols and decl["columns"][c] != db_cols[c]
        ]
        pk_diff = [p for p in decl["pk"] if p not in db_table["pk"]] + [
            p for p in db_table["pk"] if p not in decl["pk"]
        ]
        if missing or ghosts or wrong_type or pk_diff:
            problems.append((path, is_canonical, missing, ghosts, wrong_type, pk_diff))
    return problems


def main() -> int:
    args = sys.argv[1:]
    schema_file = only = markdown = None
    quiet = False
    for i, arg in enumerate(args):
        if arg == "--print-schema" and i + 1 < len(args):
            schema_file = args[i + 1]
        if arg == "--table" and i + 1 < len(args):
            only = args[i + 1]
        if arg == "--markdown" and i + 1 < len(args):
            markdown = args[i + 1]
        if arg == "--quiet":
            quiet = True
    if not schema_file:
        print(__doc__)
        return 2

    with open(schema_file, encoding="utf-8", errors="replace") as fh:
        db = parse_declarations(fh.read())
    declared = collect_declarations()
    canonical = sorted(t for t, c in declared.items() if any(x[2] for x in c))

    names = [only] if only else sorted(set(db) | set(declared))
    mismatched = ghost_tables = missing_tables = canonical_bad = 0
    lines: list[str] = []
    for table in names:
        db_table = db.get(table)
        copies = declared.get(table, [])
        if db_table is None:
            ghost_tables += 1
            entry = (f"[{table}] GHOST — declared at "
                     f"{', '.join(p for p, *_rest in copies)} but absent from the DB")
        elif not copies:
            missing_tables += 1
            entry = f"[{table}] NO DECLARATION — DB has {len(db_table['columns'])} columns"
        else:
            problems = diff_table(db_table, copies)
            if not problems:
                continue
            mismatched += 1
            if any(p[1] for p in problems):
                canonical_bad += 1
            detail = [f"[{table}] MISMATCH ({len(copies)} declaration(s), "
                      f"DB has {len(db_table['columns'])} columns)"]
            for path, is_canonical, missing, ghosts, wrong_type, pk_diff in problems:
                tag = " [canonical]" if is_canonical else ""
                if missing:
                    detail.append(f"    {path}{tag}: missing {missing}")
                if ghosts:
                    detail.append(f"    {path}{tag}: ghost {ghosts}")
                if wrong_type:
                    detail.append(f"    {path}{tag}: type {wrong_type}")
                if pk_diff:
                    detail.append(
                        f"    {path}{tag}: pk differs on {sorted(set(pk_diff))}"
                    )
            entry = "\n".join(detail)
        lines.append(entry)
        if not quiet:
            print(entry)

    summary = (
        f"DB tables: {len(db)} | declared tables: {len(declared)} "
        f"(canonical {len(canonical)}) | mismatched: {mismatched} "
        f"(of which canonical: {canonical_bad}) | "
        f"no-declaration: {missing_tables} | ghost: {ghost_tables}"
    )
    print(f"\n{summary}")

    if markdown:
        with open(markdown, "w", encoding="utf-8") as fh:
            fh.write("# Diesel schema vs database (#1519)\n\n")
            fh.write(
                "Generated by `scripts/schema_db_audit.py` from a `diesel print-schema`"
                " run over a database built by `diesel migration run` on an empty"
                " PostgreSQL 16 database. Regenerate it whenever a migration or a"
                " `table!` declaration changes.\n\n"
            )
            fh.write(f"{summary}\n\n")
            fh.write(
                f"{mismatched} declared tables disagree with the database "
                f"({canonical_bad} of them in the canonical `botschema` crate) and "
                f"{ghost_tables} declarations name a table no migration creates.\n"
            )
            if missing_tables:
                fh.write(
                    f"{missing_tables} database tables have no declaration at all "
                    "(usually accessed through raw SQL).\n"
                )
            fh.write("\n")
            fh.write("## Details\n\n```\n")
            fh.write("\n".join(lines))
            fh.write("\n```\n")
        print(f"wrote {markdown}")
    return 1 if (mismatched or ghost_tables) else 0


if __name__ == "__main__":
    sys.exit(main())
