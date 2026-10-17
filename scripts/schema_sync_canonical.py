#!/usr/bin/env python3
"""Bring a canonical `botschema` declaration in line with the live database (#1517).

`scripts/schema_db_audit.py` reports what disagrees; this tool repairs it.  The
repair is deliberately order-preserving: Diesel maps `#[derive(Queryable)]`
structs to a declaration **positionally**, so a table's committed column order
is a contract with every struct that reads it (`botschema/README.md`, "Column
order is part of the contract").  A canonical declaration is therefore updated
like a swapped-in consumer declaration:

  * columns the database has and the declaration lacks are **appended at the
    end** (position 1..n of the existing declaration does not move, so every
    existing `Queryable` struct keeps mapping to the same column);
  * columns whose **type** differs are corrected where they sit — those changes
    are visible to `cargo check` (a `Nullable<X>` column cannot be read into an
    `X` field, and vice versa);
  * **ghost** columns (declared but absent from the database) are reported and,
    with `--drop-ghosts`, removed — a ghost column makes every `SELECT` of the
    table fail at runtime, so the declaration, not the database, is wrong;
  * the primary key is corrected to the database's.

Usage:
    python3 scripts/schema_sync_canonical.py --print-schema /tmp/print_schema.rs            # report
    python3 scripts/schema_sync_canonical.py --print-schema /tmp/print_schema.rs --apply    # rewrite
    python3 scripts/schema_sync_canonical.py --print-schema /tmp/print_schema.rs --apply \
        --file core.rs --drop-ghosts                      # one module, ghosts removed

Filters: `--file NAME` (canonical file name, e.g. `core.rs`), `--table NAME`.

After an `--apply` run, re-run the audit and `cargo check` the consumers:
    python3 scripts/schema_db_audit.py --print-schema /tmp/print_schema.rs
    nohup cargo check -p botserver > /tmp/botserver_check.log 2>&1 &
"""
from __future__ import annotations

import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from schema_db_audit import canonical_type, parse_declarations  # noqa: E402
from schema_db_audit import SQL_NAME_RE  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BOTSCHEMA = os.path.join(ROOT, "botschema", "src")

# `diesel::table! { name (pk) { … } }` with the body captured as a group.
BLOCK_RE = re.compile(
    r"(?:(?:\w+)::)*(?:pub\s+)?table!\s*\{\s*(?P<table>\w+)\s*"
    r"\((?P<pk>[^)]*)\)\s*\{(?P<body>.*?)\n\s*\}\s*\n\s*\}",
    re.S,
)
COLUMN_RE = re.compile(r"^(?P<indent>\s*)(?P<name>[\w\".]+)\s*->\s*(?P<ty>.+?),?\s*$")
SQL_NAME_RE = re.compile(r'#\[\s*sql_name\s*=\s*"([^"]+)"\s*\]')

RUST_KEYWORDS = {
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
    "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
    "match", "mod", "move", "mut", "pub", "ref", "return", "self", "static",
    "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
}


def render_column(name: str, sql_type: str, indent: str) -> str:
    """Render one column line, quoting a name Diesel cannot use verbatim."""
    lines = []
    if name in RUST_KEYWORDS or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
        lines.append(f'{indent}#[sql_name = "{name}"]')
        ident = f"r#{name}" if name in RUST_KEYWORDS else f'"{name}"'
    else:
        ident = name
    lines.append(f"{indent}{ident} -> {sql_type},")
    return "\n".join(lines)


def ghost_columns(body: str, db_columns: dict):
    """Columns the declaration names that the database does not have."""
    names = []
    for raw in body.splitlines():
        col = COLUMN_RE.match(raw)
        if not col or raw.strip().startswith("//"):
            continue
        name = col.group("name").strip().lstrip('"').replace("r#", "")
        if name not in db_columns:
            names.append(name)
    return names


def rewrite_block(body: str, db_columns: dict, db_pk: list, drop_ghosts: bool):
    """Return (new_body, new_pk, notes) for one declaration body."""
    entries = []  # (name, indent, sql_type) in committed order
    indent = "        "
    pending_sql_name = None
    for raw in body.splitlines():
        stripped = raw.strip()
        attr = SQL_NAME_RE.search(stripped)
        if attr:
            pending_sql_name = attr.group(1)
            continue
        col = COLUMN_RE.match(raw)
        if not col or stripped.startswith("//"):
            continue
        name = (pending_sql_name or col.group("name")).strip().lstrip('"').replace("r#", "")
        pending_sql_name = None
        indent = col.group("indent") or indent
        entries.append((name, indent, col.group("ty").rstrip(",")))

    notes = []
    kept = []
    for name, col_indent, sql_type in entries:
        if name not in db_columns:
            if drop_ghosts:
                notes.append(f"ghost column dropped: {name}")
                continue
            notes.append(f"ghost column kept (use --drop-ghosts): {name}")
            kept.append((name, col_indent, canonical_type(sql_type)))
            continue
        fixed = db_columns[name]
        if canonical_type(sql_type) != fixed:
            notes.append(f"type {name}: {canonical_type(sql_type)} -> {fixed}")
        kept.append((name, col_indent, fixed))

    known = {name for name, _i, _t in kept}
    for name, sql_type in db_columns.items():
        if name in known:
            continue
        notes.append(f"appended column: {name} -> {sql_type}")
        kept.append((name, indent, sql_type))

    new_pk = list(db_pk)
    body_out = "\n".join(render_column(n, t, i) for n, i, t in kept)
    return body_out, new_pk, notes


def main() -> int:
    args = sys.argv[1:]
    schema_file = want_file = want_table = None
    apply_changes = drop_ghosts = skip_ghost_tables = False
    for i, arg in enumerate(args):
        if arg == "--print-schema" and i + 1 < len(args):
            schema_file = args[i + 1]
        if arg == "--file" and i + 1 < len(args):
            want_file = args[i + 1]
        if arg == "--table" and i + 1 < len(args):
            want_table = args[i + 1]
        if arg == "--apply":
            apply_changes = True
        if arg == "--drop-ghosts":
            drop_ghosts = True
        if arg == "--skip-ghost-tables":
            skip_ghost_tables = True
    if not schema_file:
        print(__doc__)
        return 2

    with open(schema_file, encoding="utf-8", errors="replace") as fh:
        db = parse_declarations(fh.read())

    changed_files = 0
    changed_tables = 0
    skipped_tables = 0
    for dirpath, _dirs, files in os.walk(BOTSCHEMA):
        for fn in sorted(files):
            if not fn.endswith(".rs"):
                continue
            if want_file and fn != want_file:
                continue
            path = os.path.join(dirpath, fn)
            with open(path, encoding="utf-8") as fh:
                text = fh.read()
            edits = []
            for match in BLOCK_RE.finditer(text):
                table = match.group("table")
                if want_table and table != want_table:
                    continue
                db_table = db.get(table)
                if db_table is None:
                    print(f"{fn}: [{table}] GHOST TABLE — no migration creates it")
                    continue
                ghosts = ghost_columns(match.group("body"), db_table["columns"])
                if ghosts and skip_ghost_tables:
                    skipped_tables += 1
                    rel = os.path.relpath(path, ROOT)
                    print(f"\n{rel} [{table}] SKIPPED — declaration has ghost "
                          f"column(s) {ghosts}; decide migration vs. code first")
                    continue
                new_body, new_pk, notes = rewrite_block(
                    match.group("body"), db_table["columns"], db_table["pk"], drop_ghosts
                )
                pk_diff = [p for p in new_pk if p not in match.group("pk")] + [
                    p.strip() for p in match.group("pk").split(",")
                    if p.strip() and p.strip() not in new_pk
                ]
                if not notes and not pk_diff:
                    continue
                changed_tables += 1
                rel = os.path.relpath(path, ROOT)
                print(f"\n{rel} [{table}]")
                for note in notes:
                    print(f"    {note}")
                if pk_diff:
                    print(f"    primary key: {sorted(set(pk_diff))}")
                if apply_changes:
                    pk_out = ", ".join(new_pk)
                    replacement = (
                        f"{match.group(0).split('(', 1)[0]}({pk_out}) {{\n"
                        f"{new_body}\n    }}\n}}"
                    )
                    edits.append((match.start(), match.end(), replacement))
            if not edits:
                continue
            for start, end, replacement in reversed(edits):
                text = text[:start] + replacement + text[end:]
            if apply_changes:
                with open(path, "w", encoding="utf-8") as fh:
                    fh.write(text)
                changed_files += 1

    verb = "rewrote" if apply_changes else "would rewrite"
    print(f"\n{verb} {changed_tables} declaration(s) in {changed_files} file(s)")
    if skipped_tables:
        print(
            f"skipped {skipped_tables} declaration(s) with ghost columns — they "
            "need a migration, not a rewrite (see botschema/README.md)"
        )
    return 1 if changed_tables else 0


if __name__ == "__main__":
    sys.exit(main())
