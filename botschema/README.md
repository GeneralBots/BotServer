# `botschema` — the single Diesel schema source of truth

`botschema` is the **only** crate in the workspace that may declare Diesel
`table!` / `joinable!` / `allow_tables_to_appear_in_same_query!` macros. Every
other crate re-exports from here instead of declaring its own copy. This closes
the drift tracked in [#1497](https://github.com/generalbots/generalbots/issues/1497)
(`bots` was declared 16×, `user_sessions` 12×, `users` 10×, …).

## Decision (issue #1513)

**Home = the existing `botschema` crate.** Rationale:

- It is already a default workspace member and already holds ~209 tables.
- Its only dependencies are `diesel`, `chrono`, `uuid`, `serde`, `serde_json` —
  it depends on **no** first-party bot crate, so wiring it into any consumer
  can never introduce a dependency cycle.
- A new `botdbschema` crate would duplicate this for no gain.

Consumers enable only the feature(s) for the tables they need (see below), so a
crate that only touches core tables does not pull in the whole schema.

## Layout

Modules are grouped by domain and gated behind cargo features so downstream
crates opt in minimally:

| Module | Feature | Notes |
|---|---|---|
| `core`, `core::part2` | *(always on)* | tenants, organizations, branches, bots, users, sessions, … |
| `drive`, `desktop`, `database_manager`, `email_integration` | *(always on)* | frequently shared |
| `tasks`, `mail`, `people`, `tickets`, `billing`, `attendant`, `calendar`, `goals`, `canvas`, `workspaces`, `compliance`, `meet`, `research`, `learn`, `project`, `dashboards`, `analytics`, `social` | matching feature name | domain tables |

Feature-gated modules re-export their tables at the crate root (via
`pub use self::<module>::*;`) so consumers can `use botschema::<table>;`.

## Consuming the schema

Add the dependency (pick the features you need):

```toml
[dependencies]
botschema = { workspace = true, features = ["people", "billing"] }
```

And import tables directly:

```rust
use botschema::{bots, users, crm_contacts};
// each is a diesel table module: bots::dsl, bots::columns, bots::table, …
```

If you are replacing a local `table!` block, keep the call sites stable by
re-exporting from your old module path:

```rust
// in <crate>/src/schema.rs — replaces the old `table! { bots … }` block
pub use botschema::bots;
```

`<crate>::schema::bots::dsl` then resolves exactly as before.

## How to change the schema

1. **Write a migration** under `botserver/migrations/`
   (`<version>-name/{up.sql,down.sql}`). Never edit an applied migration, and
   mind the **version order**: `diesel` compares the dash-stripped directory
   names as strings, so a migration runs after everything whose name sorts
   lower. Use the dotted `6.7.1-…` form — a date-prefixed name such as
   `2026-09-11-210100-…` sorts *before* `6.0.0-01-core` and runs against an
   empty database (see the bootstrap defect below).
2. **Apply it** to a dev database:
   `cd botserver && diesel migration run` (uses `botserver/diesel.toml`).
3. **Validate the real shape** against `botschema`: `diesel print-schema` over
   the migrated DB, then
   `python3 scripts/schema_db_audit.py --print-schema <file>`, which diffs every
   `table!` in the tree against the database and lists missing columns, ghost
   columns, type differences and primary-key differences per file. Update the
   `table!` in `botschema` to match.
4. **Commit** the migration *and* the `botschema` change together.
5. Regenerate the drift inventory so the baseline stays honest:
   `python3 scripts/schema_drift_inventory.py` and, if the change removes a
   non-canonical declaration, refresh the guard baseline with
   `python3 scripts/schema_guard.py --update`.

### Database verification (#1519)

The audit runs without a container engine — the PostgreSQL binaries extract to
`/tmp` and `initdb` a throwaway cluster as a normal user. `diesel_cli` 2.3 with
the `postgres` feature (`cargo install diesel_cli --no-default-features
--features postgres`) and a `libpq` on the box are the only other requirements:

```bash
# 1. throwaway cluster (PostgreSQL from the zonky embedded-postgres jar)
mkdir -p /tmp/pgdist && cd /tmp/pgdist
curl -sO "https://repo1.maven.org/maven2/io/zonky/test/postgres/embedded-postgres-binaries-linux-amd64/16.2.0/embedded-postgres-binaries-linux-amd64-16.2.0.jar"
python3 -c "import zipfile; zipfile.ZipFile('embedded-postgres-binaries-linux-amd64-16.2.0.jar').extract('postgres-linux-x86_64.txz', '.')"
mkdir -p /tmp/pgsock && tar -xJf postgres-linux-x86_64.txz -C /tmp/pgdist
PATH=/tmp/pgdist/bin:$PATH initdb -D /tmp/pgdata -U postgres --auth=trust
PATH=/tmp/pgdist/bin:$PATH pg_ctl -D /tmp/pgdata -l /tmp/pgdata/server.log \
  -o "-p 54329 -k /tmp/pgsock" -w start
# 2. apply every migration with diesel itself
cd botserver && DATABASE_URL=postgres://postgres@127.0.0.1:54329/gbtest \
  diesel migration run            # 166 migrations, exit 0; a second run is a no-op
# 3. dump the real shape and diff it against the committed declarations
DATABASE_URL=postgres://postgres@127.0.0.1:54329/gbtest \
  diesel print-schema --schema public > /tmp/print_schema.rs
python3 scripts/schema_db_audit.py --print-schema /tmp/print_schema.rs
```

The resulting snapshot is checked in as `issues/schema/db_audit.md`: of the 471
tables the migrations build, 298 are declared somewhere in the tree, **171 of
those declarations disagree with the database (128 of them canonical)**, 10
declarations name a table no migration creates (`billing_subscriptions`, and
the `botmarketing` / `botautotask` / `botvideo` tables) — those are runtime
query failures, not cosmetics — and 183 database tables have no declaration at
all (touched through raw SQL only).

The first run of this workflow exposed a **bootstrap defect**: six migrations
were named `2026-…`, which sorts before `6.0.0-01-core`, so a fresh
`diesel migration run` began with `ALTER TABLE user_email_accounts …` against
an empty database, aborted, and left the remaining 160 migrations unapplied
(the runtime logs the migration error and continues, so only a fresh database
shows it). They are now `6.7.0`–`6.7.5`; the two `ADD CONSTRAINT` statements
that were not `IF NOT EXISTS`-guarded sit in `DO` blocks so a database that
already applied them under the old version re-runs them harmlessly.

## CI guard

`scripts/schema_guard.py` (run in CI as the "no new `table!` outside
botschema" step) fails the build when a `table!` appears outside this crate or
the non-canonical count rises above the checked-in baseline
(`issues/schema/table_baseline.json`). The baseline only ever moves **down**;
once the migration children land it reaches 0 and any stray `table!` is an
immediate CI failure.

## Drift inventory

`python3 scripts/schema_drift_inventory.py` regenerates
`issues/schema/schema_drift.md`, which classifies every non-canonical
declaration as identical / additive / conflicting and names the file:line —
the data artifact the migration children consume.

`python3 scripts/schema_migration_truth.py [--table NAME] [--json OUT]` derives
each table's column list from the migrations themselves (including the
`EXECUTE format(...)` loops that 6.5.15.1-consolidated uses for `branch_id`).
Its `--audit` mode lists every declaration naming a column no migration
creates — the evidence base for the 65 conflicting tables (#1517). Migrations
are not the only schema writer (the runtime may add columns through
`sync_bot_tables()`), so treat a hit as a lead to confirm against a live
`diesel print-schema` (#1519), not as proof of a dead column.

## Column order is part of the contract

The drift inventory classifies by column *sets* and types, but `diesel`
maps `#[derive(Queryable)]` structs to the table **positionally**: field *n* of
the struct reads column *n* of the declaration. Consequences for replacing a
local `table!` with `pub use botschema::<table>;`:

- **Identical** declarations swap mechanically (column order is equal by
  definition).
- **Additive** declarations swap when the local order is the canonical order
  *plus extras appended at the end* — then the extras can be appended to the
  canonical declaration and every existing `Queryable` struct keeps mapping
  correctly (e.g. `user_email_accounts`).
- If the two orders differ anywhere else the declaration cannot simply be
  dropped. The order to keep is then derived, not guessed:

  1. **Check who reads the canonical declaration.** If nothing outside this
     crate imports `botschema::<table>` (verify with
     `grep -rn 'botschema::<table>' --include=*.rs`), the canonical column
     order is free — no `Queryable` struct depends on it.
  2. **Adopt the consuming declaration's order verbatim**, including every
     column the canonical copy was missing. The consuming structs were written
     against it, so after the swap they map to the same columns as before: a
     positional no-op. This also repairs the canonical copy, which for those
     tables was missing real migration-backed columns (`support_tickets`
     lacked `record_type` from `6.5.33-tickets-itsm-unification`, the
     `okr_*` tables lacked `org_id`, the `compliance_*` tables lacked
     `org_id`/`bot_id`).
  3. **Then** replace the local declaration with `pub use botschema::<table>;`.

  Extra columns therefore land wherever the consumer has them (not necessarily
  at the end) — the invariant that matters is *the canonical order equals the
  consuming order*, not prefix compatibility. All 18 additive declarations
  were adopted this way in [#1516](https://github.com/generalbots/generalbots/issues/1516)
  (`user_email_accounts` first, then the remaining 17); the inventory now
  reports **additive 0 / identical 0**, leaving 65 conflicting tables, whose
  orders disagree in ways a swap cannot repair.
  If a second consumer of such a table ever appears, it must adopt the same
  order (or select columns explicitly).

**Verification for adopted tables:** `Queryable` order is invisible to
`cargo check`, so a positional mismatch compiles and mis-maps at runtime. After
changing a canonical order, compare every consuming struct's field order with
the declaration it now compiles against (crate-local `table!` if it still
has one, otherwise the canonical one). For the #1516 batch that check covered
161 positionally-mapped structs with **0 order mismatches**; structs whose
field count differs from the column count are the `Insertable`/`New*` ones,
which map by name.

### `allow_tables_to_appear_in_same_query!` after a swap

The macro expands to `impl TableNotEqual<A::table> for B::table {}`, so it can
only name tables declared in the invoking crate. When a table moves to
`botschema`, drop it from the consuming crate's list. If both tables of a pair
are canonical, the impl belongs in the `botschema` module that declares them —
note that `botschema` does not re-export the `dashboards`, `analytics` and
`social` modules at the crate root (their own `dashboards`/`analytics`/`social`
tables would collide with the module name), so reach those tables through the
module path: `botschema::dashboards::conversational_queries`.
