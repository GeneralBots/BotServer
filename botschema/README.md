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
   (`YYYY-MM-DD-HHMMSS_name/{up.sql,down.sql}`). Never edit an applied migration.
2. **Apply it** to a dev database:
   `cd botserver && diesel migration run` (uses `botserver/diesel.toml`).
3. **Validate the real shape** against `botschema`: run
   `diesel print-schema` (against the migrated DB) and diff it with the
   corresponding `botschema` module; update the `table!` there to match.
4. **Commit** the migration *and* the `botschema` change together.
5. Regenerate the drift inventory so the baseline stays honest:
   `python3 scripts/schema_drift_inventory.py` and, if the change removes a
   non-canonical declaration, refresh the guard baseline with
   `python3 scripts/schema_guard.py --update`.

> Steps 2–3 need a live Postgres built from `botserver/migrations`. Verifying
> that `print-schema` output equals the committed schema on a fresh DB is the
> deliverable of [#1519](https://github.com/generalbots/generalbots/issues/1519).

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
