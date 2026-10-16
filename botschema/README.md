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
the data artifact the migration children consume.## Column order is part of the contract

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
