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
   columns, type differences and primary-key differences per file.
4. **Repair the canonical declaration** with
   `python3 scripts/schema_sync_canonical.py --print-schema <file> --table
   <name>` (add `--apply` to rewrite). It appends the columns the database has
   and the declaration lacks, corrects the types in place, and corrects the
   primary key — see "Repairing a canonical declaration" below for the rules it
   follows, why it never reorders a table it leaves in place, and why one table
   (with its readers) is the unit of work. Tables whose declaration names a
   column the database does *not* have are reported and skipped
   (`--skip-ghost-tables`): that column needs a migration, not a rewrite (see
   "Ghost columns").
5. **Commit** the migration *and* the `botschema` change together.
6. Regenerate the drift inventory so the baseline stays honest:
   `python3 scripts/schema_drift_inventory.py` and, if the change removes a
   non-canonical declaration, refresh the guard baseline with
   `python3 scripts/schema_guard.py --update`.

### Fresh-database verification (#1519)

The migrations are not the only writer of the schema (the bootstrap path also
creates tables), so the authority is `diesel print-schema` against a **fresh**
database:

```bash
# throwaway cluster, e.g. port 55432
createdb -p 55432 botserver_fresh
cd botserver
DATABASE_URL=postgres://postgres@localhost:55432/botserver_fresh diesel migration run
DATABASE_URL=postgres://postgres@localhost:55432/botserver_fresh \
  diesel print-schema --schema public > /tmp/print_schema.rs
cd ..
python3 scripts/schema_db_audit.py --print-schema /tmp/print_schema.rs
python3 scripts/schema_undeclared_report.py --print-schema /tmp/print_schema.rs \
  --markdown issues/schema/undeclared_tables.md
```

Acceptance is `mismatched: 0` (canonical) and `ghost: 0`. Tables reported as
having **no declaration** are triaged by `schema_undeclared_report.py`: `raw
SQL` tables are accessed through `sql_query`/string SQL and are documented with
their call sites; `dormant` tables have no Rust reference and survive only for
migrations or external tools.

### Repairing a canonical declaration

`schema_sync_canonical.py` treats the database as the authority, but it never
reorders a declaration. Diesel maps `#[derive(Queryable)]` structs to a
declaration **positionally**, so the committed order is a contract with every
struct that reads it: a column the database has and the declaration lacks is
appended at the end (positions `1..n` do not move, so a struct that reads the
first `n` columns keeps reading the same ones), a wrong type is corrected where
it sits, and the primary key is corrected in place. Type corrections are
visible to `cargo check` — a `Nullable<X>` column cannot be read into an `X`
field and vice versa — so the compiler lists the structs that must follow the
repair. Appended columns are *not* silent either: a `table.load::<T>()` selects
the declaration's columns, so `T` needs the new fields.

Two hard constraints make this a **per-module** job, never a wholesale rewrite:

- **Every appended column breaks the positional readers of that table.** All
  `#[derive(Queryable)]` structs bound to the table must gain the new fields,
  because `table.load::<T>()` selects the declaration's columns. Reconciling
  the whole crate at once grew **96 canonical tables** and pulled in structs
  across ~40 feature crates; repair one module (and its readers) per commit
  with `--table <name>`.
- **Diesel caps a declaration at 32 columns** (`32-column-tables`, which every
  crate here enables). `products` has **71 columns in the database** against 22
  declared, so it cannot be reconciled at all until that cap is raised — and
  `128-column-tables` roughly triples the build time of `diesel` itself, which
  is a cost for the whole workspace. The audit therefore keeps reporting
  `products` as a mismatch, and reconciling it needs an explicit decision on
  that build cost.

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

The snapshot is checked in as `issues/schema/db_audit.md`. **Two independent
fresh databases were built** (the throwaway cluster above and a
`diesel database setup` run) and their `print-schema` output is byte-identical,
which is what makes the physical shape a fact rather than a local accident.
After the canonical repair of this change: of the 471 tables the migrations
build, 297 are declared somewhere in the tree, **110 of those declarations
disagree with the database (16 of them canonical)**, 9 declarations name a
table no migration creates (six in `botmarketing`, two in `botautotask`, one in
`botvideo`) — a table that does not exist is a runtime query failure, not a
cosmetic one; `botautotask` is a default dependency and its safety layer runs
`SELECT … FROM safety_constraints` — and 183 database tables have no
declaration at all (touched through raw SQL only). The 94 non-canonical
declarations still disagreeing are the remaining migration children (#1517);
they resolve into the canonical copy as each feature crate re-exports from
here.

The first run of this workflow exposed a **bootstrap defect**: six migrations
were named `2026-…`, which sorts before `6.0.0-01-core`, so a fresh
`diesel migration run` began with `ALTER TABLE user_email_accounts …` against
an empty database, aborted, and left the remaining 160 migrations unapplied
(the runtime logs the migration error and continues, so only a fresh database
shows it). They are now `6.7.0`–`6.7.5`; the two `ADD CONSTRAINT` statements
that were not `IF NOT EXISTS`-guarded sit in `DO` blocks so a database that
already applied them under the old version re-runs them harmlessly.

### Ghost columns — a declaration the database does not have

48 columns across **16 canonical declarations** name a column no database has.
A ghost column is not cosmetic: every `SELECT` that mentions it fails at
runtime with `column … does not exist`, including the implicit `SELECT *` that
`table.load::<T>()` issues. Three mechanisms put them there, all checked
against `botserver/migrations/`:

- **A silently no-op `CREATE TABLE IF NOT EXISTS` restatement** (21 of the 48).
  `6.5.15.1-consolidated` restates tables that an earlier migration already
  created; because the guard makes the statement a no-op, the columns it adds
  never exist — `users.is_admin` (restated at
  `6.5.15.1-consolidated/up.sql:2909`, while `6.0.0-01-core` already created
  `users` without it), `workflow_events.execution_id/event_type/payload`,
  `projects.*`, `basic_tools.file_hash`,
  `bot_configuration.is_encrypted/config_type`. The same mechanism explains most
  of the non-canonical mismatches: the later shape is what the declaration was
  written against, the earlier shape is what runs.
- **A column no migration ever creates** — `crm_leads`
  `first_name/last_name/email/phone/company/status/score/notes/assigned_to`
  (`6.0.7-01-people/up.sql:82` creates `crm_leads` with the deal shape:
  `title`, `value`, `stage`, `probability`, …), `email_templates
  .subject/body/variables`, `global_email_signatures.content/is_default`,
  `billing_grace_periods.subscription_id/starts_at/ends_at/reason`,
  `billing_usage_alerts.recipients/is_active`, `distribution_lists
  .members/is_active`, `kb_collections.description`,
  `kb_group_associations.collection_id/created_at`,
  `inventory_movements.reference/updated_at`.
- **A column dropped by a later migration** —
  `marketing_campaigns.channel/scheduled_at` (`6.2.4-01-campaigns` creates
  them, `6.5.46-marketing-campaigns-branch-schema` removes them) and
  `distribution_lists.stalwart_principal_id`.

The 16 declarations, which `schema_sync_canonical.py` reports and skips:
`basic_tools` (file_hash), `billing_grace_periods`
(subscription_id, starts_at, ends_at, reason), `billing_usage_alerts`
(recipients, is_active), `bot_configuration` (is_encrypted, config_type),
`compliance_checks` (framework, control_id, control_name, score, evidence,
notes), `crm_leads` (first_name, last_name, email, phone, company, status,
score, notes, assigned_to), `distribution_lists` (members, is_active,
stalwart_principal_id), `email_templates` (subject, body, variables),
`global_email_signatures` (content, is_default), `inventory_movements`
(reference, updated_at), `kb_collections` (description),
`kb_group_associations` (collection_id, created_at), `marketing_campaigns`
(channel, scheduled_at), `projects` (organization_id, start_date, end_date,
owner_id, settings), `users` (is_admin), `workflow_events` (execution_id,
event_type, payload).

Each one needs a product decision, not a rewrite — **add the column**
(`ALTER TABLE … ADD COLUMN IF NOT EXISTS …`, plus a backfill) when the code that
reads it is the intended behaviour, or **delete the column and its readers**
when it is not. `projects` is the boundary case: `6.5.49-vibe-projects` and
`6.5.67-project-persistence` are named as if they own that table, so its
`organization_id/start_date/end_date/owner_id/settings` may belong to a
migration that was never written.

## DB-truth repairs applied to canonical declarations

Each shape below is derived from the migrations that actually execute (the
`CREATE TABLE` that runs first wins; `6.5.15.1-consolidated` restates most core
tables with `IF NOT EXISTS` and therefore never applies) plus every later
`ALTER TABLE`:

| table | correction | evidence |
|---|---|---|
| `bots` | `is_public` / `is_default_for_branch` are `Bool` (NOT NULL) | `6.5.15.1-consolidated/up.sql:576` declares both NOT NULL, but the incremental path (`6.3.2-01-add-bots-public`, `6.5.15-org-branches`) added them nullable. Migration `6.7.6-bots-flags-not-null` backfills and enforces NOT NULL |
| `user_sessions` | 18 columns, `user_id Varchar`, `bot_id Uuid`, `title Varchar`, `current_tool Nullable<Varchar>`, plus `answer_mode` / `message_count` / `total_tokens` / `last_activity` | `6.0.0-01-core/up.sql:137` creates the table with the legacy 12-column shape; `6.0.0:490`, `6.1.5-01-mail:266`, `6.5.23-branch-scope-cleanup:732` and `6.5.30-user-sessions-schema-fix` append `tenant_id`, `active_email_account_id`, `branch_id` (NOT NULL after a nil-UUID backfill), `session_id`, `data`, `expires_at`. The 8-column shape in `6.5.15.1-consolidated:2892` never runs |
| `services` | `org_id`, `bot_id`, `tax_rate` added; `currency` / `attributes` / `is_active` stay non-nullable | `6.0.10-01-products/up.sql:27` creates it, `6.5.23:1324` adds `branch_id` NOT NULL, `6.5.40-services-tax` adds `tax_rate` |
| `product_categories` | `org_id` / `bot_id` are `Nullable<Uuid>`; `display_order` and `updated_at` added | `6.0.10-01-products/up.sql:45`, then `6.5.15.1:83-93` and `6.5.98-product-categories-display-order` add `display_order`/`updated_at` and drop NOT NULL on `org_id`/`bot_id` (the botproducts seed omits them) |
| `products` | 22 declared columns (`6.0.10-01-products`' 23 minus `org_id`/`bot_id`, plus `branch_id`, `category_id`, `is_public`); 21 of the 45 `6.0.2-01-core` additions dropped | `6.0.10-01-products/up.sql:1` creates the modelled shape; `6.5.23:1314`, `6.5.15.1:99` and `6.5.22-products-is-public` add the three extras. `6.7.7-products-drop-unused-columns` removes the `6.0.2` fiscal/shipping/SEO columns that no Rust module, no bot template and no later migration names. The other 24 `6.0.2` columns stay because bot templates write them into `products` directly (`store-server.gbdialog/get-ncm.bas` writes `tax_code`/`tax_class`, `scan-barcode.bas` writes `global_trade_number`/`net_weight`/`brand`) |

Ghost columns removed with the swap: the 14 `bots` copies carried `bot_id`,
`avatar_url`, `settings` and `metadata`, none of which any migration creates.
They are the `6.5.15.1` restatement mechanism in reverse — the declaration was
written against the shape of a table that was never built, so every query
naming them would have failed with `column bots.avatar_url does not exist`.
`services` had the same problem in `botbilling`/`botproducts` (`sku`, `price`,
`is_recurring`, `billing_cycle`); both copies are gone now, and the
`Service` / `ProductCategory` structs in `botproducts/src/lib.rs` were
re-ordered to the canonical column order at the same time (`Queryable` binds
positionally, so a correct column *set* with the wrong order still mis-maps).
`services.org_id` / `bot_id` are NOT NULL, so `create_service` resolves them
from the branch (`org_for_branch` / `bot_for_branch`, mirroring
`botpeople`), while `product_categories` keeps both NULL on insert — that is
exactly what `6.5.15.1`/`6.5.98` dropped NOT NULL for.

### Removing columns: the migration is the source, the declaration follows

When the database carries columns nothing reads, the fix is a `DROP COLUMN IF
EXISTS` migration — not a declaration that models them. `products` is the worked
example (`6.7.7-products-drop-unused-columns`): the candidate set is "every column
the migrations create minus every column the canonical declaration models", then
filtered by a whole-tree search for the column name (`botserver`, `botui`,
`bottemplates` — including the generated `botserver/work/templates`, which holds
the bot `.bas` sources — excluding `botserver/migrations` and `botschema/src`).
Only columns with zero hits are dropped; `tax_code`, `tax_class`,
`global_trade_number`, `net_weight` and `brand` survived that filter precisely
because a bot template writes them, which no Rust-only audit would have caught.
A `down.sql` must restore the original `6.0.2-01-core` definitions (values are
lost either way).

The `products` columns that remain undeclared (`org_id`, `bot_id`, `tax_code`,
`tax_class`, `global_trade_number`, `net_weight`, `brand`, `slug`, …) are live
through bot scripts and `botbrazil`, so declaring them is the next step, and it
has to clear the Diesel 32-column tuple limit first (subset selects /
`QueryableByName` instead of a whole-row `Queryable`).

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
