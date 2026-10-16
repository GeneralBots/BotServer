#!/usr/bin/env python3
"""Derive each table's column list from the migrations (no live Postgres).

Scans `botserver/migrations/*/up.sql` in directory-name order, applying
`CREATE TABLE [IF NOT EXISTS] ...`, `ALTER TABLE ... ADD COLUMN`, `DROP COLUMN`,
`RENAME COLUMN` and `ALTER COLUMN ... TYPE` — including the `EXECUTE format(...)`
loops that 6.5.15.1-consolidated uses to add `branch_id`/`tenant_id` to a table
list.  Output feeds `--audit`, which reports every `table!` declaration whose
columns the migrations never create: the raw evidence for reconciling the 65
conflicting tables (#1517).

Caveat: migrations are not the only writer of the schema.  The runtime may add
columns through `sync_bot_tables()` and a deployment may have applied manual
DDL, so a column missing here is a *lead*, not proof — confirm against a live
`diesel print-schema` (#1519) before deleting a declaration.

Usage:
    python3 scripts/schema_migration_truth.py [--table NAME] [--json OUT]
    python3 scripts/schema_migration_truth.py --audit
"""
from __future__ import annotations

import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MIGRATIONS = os.path.join(ROOT, "botserver", "migrations")

# Postgres table elements that are not column definitions.
CONSTRAINT_STARTS = (
    "constraint", "primary", "unique", "foreign", "check", "exclude", "like",
)
CREATE_RE = re.compile(
    r"create\s+table\s+(?:if\s+not\s+exists\s+)?(?:public\.)?\"?(\w+)\"?\s*\(",
    re.I | re.S,
)
ALTER_STMT_RE = re.compile(
    r"alter\s+table\s+(?:if\s+exists\s+)?(?:only\s+)?(?:public\.)?\"?(\w+)\"?\s+([^;]+);",
    re.I | re.S,
)
ADD_COL_RE = re.compile(
    r"add\s+column\s+(?:if\s+not\s+exists\s+)?\"?(\w+)\"?\s+", re.I
)
DROP_COL_RE = re.compile(
    r"drop\s+column\s+(?:if\s+exists\s+)?\"?(\w+)\"?", re.I
)
RENAME_COL_RE = re.compile(
    r"rename\s+column\s+\"?(\w+)\"?\s+to\s+\"?(\w+)\"?", re.I
)
ALTER_TYPE_RE = re.compile(
    r"alter\s+column\s+\"?(\w+)\"?\s+(?:set\s+data\s+)?type\s+", re.I
)


def split_top_level(body: str):
    """Split a CREATE TABLE body on commas that sit outside parentheses."""
    items, depth, current = [], 0, []
    for ch in body:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            items.append("".join(current).strip())
            current = []
        else:
            current.append(ch)
    if "".join(current).strip():
        items.append("".join(current).strip())
    return items


def normalize_type(raw: str) -> str:
    """Reduce a SQL type expression to a lowercase comparable token."""
    t = " ".join(raw.split())
    t = re.sub(r"\s+(not\s+null|null|default\s+.+|primary\s+key|unique|"
               r"references\s+.+|check\s*\(.*|generated\s+.+)$", "", t, flags=re.I)
    t = t.rstrip(",").strip().lower()
    return t


COLUMN_ITEM_RE = re.compile(r'^"?([A-Za-z_][A-Za-z0-9_]*)"?\s+(\S.*)$', re.S)


def column_item(item: str):
    """Return (name, rest) when a CREATE TABLE body item declares a column."""
    m = COLUMN_ITEM_RE.match(item.strip())
    if not m:
        return None
    name = m.group(1)
    if name.lower() in CONSTRAINT_STARTS:
        return None
    return name.lower(), m.group(2)


def first_action(rest: str) -> str:
    """Take the first COMPLETE action of a possibly multi-action clause."""
    return split_top_level(rest)[0]


def create_body(sql: str, start: int):
    """Return the parenthesised body of a CREATE TABLE and its end offset."""
    depth, i = 0, start
    begin = sql.find("(", start)
    if begin < 0:
        return None, start
    i = begin
    while i < len(sql):
        if sql[i] == "(":
            depth += 1
        elif sql[i] == ")":
            depth -= 1
            if depth == 0:
                return sql[begin + 1 : i], i + 1
        i += 1
    return None, start


ARRAY_RE = re.compile(r"ARRAY\s*\[(.*?)\]", re.S)
FORMAT_EXEC_RE = re.compile(r"execute\s+format\s*\(\s*'(.*?)'\s*,", re.I | re.S)


def expand_dynamic_sql(text: str) -> str:
    """Inline `EXECUTE format('<sql>', …)` for each table array in the file.

    Migrations here reconcile columns with a `FOREACH t IN ARRAY [...]` loop
    whose body runs one `EXECUTE format('ALTER TABLE public.%I …', t)` for every
    listed table, so the literal SQL text never appears per table. Expanding the
    template once per array element makes those columns visible.
    """
    arrays = [re.findall(r"'([^']+)'", group) for _, group in
              ((m.start(), m.group(1)) for m in ARRAY_RE.finditer(text))]
    templates = [m.group(1) for m in FORMAT_EXEC_RE.finditer(text)]
    if not arrays or not templates:
        return text
    expanded = []
    for names in arrays:
        for template in templates:
            for name in names:
                if not re.fullmatch(r"[a-z_][a-z0-9_]*", name):
                    continue
                expanded.append("\n" + template.replace("%I", name) + ";\n")
    if not expanded:
        return text
    at = text.find("DO")
    insert_at = at if at >= 0 else len(text)
    return text[:insert_at] + "".join(expanded) + text[insert_at:]


def build_truth():
    tables: dict[str, dict] = {}
    for name in sorted(os.listdir(MIGRATIONS)):
        up = os.path.join(MIGRATIONS, name, "up.sql")
        if not os.path.isfile(up):
            continue
        with open(up, encoding="utf-8", errors="replace") as fh:
            sql = fh.read()
        if not re.search(r"\b(create\s+table|alter\s+table)\b", sql, re.I):
            continue
        text = expand_dynamic_sql(re.sub(r"--[^\n]*", "", sql))
        events = []
        for m in CREATE_RE.finditer(text):
            events.append((m.start(), "create", m))
        for m in ALTER_STMT_RE.finditer(text):
            events.append((m.start(), "alter", m))
        for _, kind, m in sorted(events, key=lambda e: e[0]):
            if kind == "create":
                body, _ = create_body(text, m.end() - 1)
                if body is None:
                    continue
                key = m.group(1).lower()
                entry = tables.setdefault(key, {"created_in": name, "columns": []})
                fresh = []
                for item in split_top_level(body):
                    parsed = column_item(item)
                    if parsed is None:
                        continue
                    col, rest = parsed
                    fresh.append(
                        {"name": col, "sql_type": normalize_type(rest), "source": name}
                    )
                known = {c["name"] for c in entry["columns"]}
                if not entry["columns"]:
                    entry["columns"] = fresh
                elif any(c["name"] not in known for c in fresh):
                    # A later `CREATE TABLE IF NOT EXISTS` that knows more columns
                    # than the first one is the shape a fresh install gets (see
                    # 6.5.15.1-consolidated); later ALTERs still apply on top.
                    entry["columns"] = fresh
                    entry["restated_in"] = name
            else:
                apply_alter(tables, m.group(1).lower(), m.group(2), name)
    return tables


def apply_alter(tables: dict, key: str, clause: str, migration: str) -> None:
    """Apply one `ALTER TABLE` action list to the derived table state."""
    for m in ADD_COL_RE.finditer(clause):
        col = m.group(1).lower()
        entry = tables.setdefault(key, {"created_in": None, "columns": []})
        if any(c["name"] == col for c in entry["columns"]):
            continue
        entry["columns"].append(
            {
                "name": col,
                "sql_type": normalize_type(first_action(clause[m.end() :])),
                "source": migration,
            }
        )
    for m in DROP_COL_RE.finditer(clause):
        entry = tables.get(key)
        if entry:
            col = m.group(1).lower()
            entry["columns"] = [c for c in entry["columns"] if c["name"] != col]
    for m in RENAME_COL_RE.finditer(clause):
        entry = tables.get(key)
        if entry:
            old, new = m.group(1).lower(), m.group(2).lower()
            for c in entry["columns"]:
                if c["name"] == old:
                    c["name"] = new
    for m in ALTER_TYPE_RE.finditer(clause):
        entry = tables.get(key)
        if entry:
            col = m.group(1).lower()
            for c in entry["columns"]:
                if c["name"] == col:
                    c["sql_type"] = normalize_type(first_action(clause[m.end() :]))


TABLE_DECL_RE = re.compile(
    r"(?:diesel::)?table!\s*\{\s*(\w+)\s*\((\w+)\)\s*\{(.*?)\n\s*\}", re.S
)


def declared_tables(root: str):
    """Yield (table, path, columns) for every `table!` under `root`."""
    for base, _dirs, files in os.walk(root):
        if "/target/" in base or base.endswith("/target"):
            continue
        for fname in files:
            if not fname.endswith(".rs"):
                continue
            path = os.path.join(base, fname)
            with open(path, encoding="utf-8", errors="replace") as fh:
                text = fh.read()
            if "table!" not in text:
                continue
            rel = os.path.relpath(path, ROOT)
            for m in TABLE_DECL_RE.finditer(text):
                cols = [
                    line.strip().split("->")[0].strip()
                    for line in m.group(3).splitlines()
                    if "->" in line and not line.strip().startswith("//")
                ]
                yield m.group(1), rel, cols


def audit(truth, roots):
    """Report declarations whose columns the migrations never create."""
    unknown_table, unknown_column = [], []
    for root in roots:
        for table, rel, cols in declared_tables(os.path.join(ROOT, root)):
            entry = truth.get(table)
            if entry is None:
                unknown_table.append((table, rel))
                continue
            physical = {c["name"] for c in entry["columns"]}
            ghosts = [c for c in cols if c not in physical]
            if ghosts:
                unknown_column.append((table, rel, ghosts))
    return unknown_table, unknown_column


def main():
    truth = build_truth()
    want = None
    out_json = None
    do_audit = "--audit" in sys.argv
    args = sys.argv[1:]
    for i, a in enumerate(args):
        if a == "--table" and i + 1 < len(args):
            want = args[i + 1].lower()
        if a == "--json" and i + 1 < len(args):
            out_json = args[i + 1]

    if out_json:
        with open(out_json, "w", encoding="utf-8") as fh:
            json.dump(truth, fh, indent=1, sort_keys=True)
        print(f"wrote {out_json}")

    if do_audit:
        unknown_table, unknown_column = audit(truth, ("botschema", "botserver"))
        print(f"tables derived from migrations: {len(truth)}")
        print(f"tables declared but absent from migrations: {len(unknown_table)}")
        for table, rel in unknown_table:
            print(f"  {table} ({rel})")
        print(f"\ndeclarations naming a column the migrations never create: {len(unknown_column)}")
        for table, rel, ghosts in unknown_column:
            print(f"  {table} ({rel}): {ghosts}")
        return

    names = [want] if want else sorted(truth)
    for name in names:
        entry = truth.get(name)
        if entry is None:
            print(f"{name}: not found in migrations")
            continue
        print(f"{name} (created in {entry['created_in']}, {len(entry['columns'])} columns)")
        for c in entry["columns"]:
            print(f"  {c['name']:<34} {c['sql_type']:<26} +{c['source']}")
    print(f"\ntables derived from migrations: {len(truth)}")


if __name__ == "__main__":
    main()
