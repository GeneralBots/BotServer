# #1497 continuation plan — Diesel schema ownership

**Write this plan's state numbers from `schema_guard.py` + `schema_drift_inventory.py`, not by hand.**

Current tree is committed and green: `git status` clean,
`cargo check -p botserver` exit 0, `python3 scripts/schema_guard.py` → OK at **73**.

## Where the numbers went

302 → 240 → 221 → 179 → 86 → **73** non-canonical `table!` declarations, driven by:
- **#1513** canonical crate `botschema` (closed)
- **#1514** inventory script + `issues/schema/schema_drift.md` (closed)
- **#1515** byte-identical dedupe, 49 → 0 (closed)
- **#1516** additive adoption, 18 → 0 (closed)
- **#1517** conflicting reconcile, 65 → 14 (open, in progress)
- **#1520** CI guard `scripts/schema_guard.py` (closed)
- **#1521** docs (closed)
- **#1518** ten `UserSession` models (open, not started)
- **#1519** DB truth audit `issues/schema/db_audit.md` (open, artifact exists)

## Remaining: 73 declarations / 38 tables / 22 files

```
 10  botproducts/src/schema.rs         3 conflicting + 7 unique
  9  botcloud/src/schema_ext.rs        8 unique + 1 conflicting
  7  botbasic_types/src/schema.rs      7 conflicting
  7  botcorebot/src/schema.rs          7 conflicting
  6  botsources/src/schema.rs          5 unique + 1 conflicting
  6  botpeople/src/lib_part1.rs        5 unique + 1 conflicting
  4  botwhatsapp/src/schema.rs         4 conflicting
  4  botcontacts/src/schema.rs         3 conflicting + 1 unique
  3  bottasks, botcoreoauth, botcompliance, botbilling   (3 each)
  2  botcoredirectory/{provisioning,api}.rs, botcoresession, botemail, botattendance
  1  botpaper, botmsteams, bottelegram, botlib/src/models/session.rs,
     botserver/src/main_module/routes/catalog.rs
```

`identical: 0 · additive: 0 · conflicting: 14 · unique: 37`

### Step A — the 37 `unique` tables (easiest: pure move)

Declared exactly once, outside `botschema`. Move the block verbatim into the
right `botschema/src/*.rs`, re-export it, done.

> **Scope corrected after execution (#1522 closed).** "Declared once" ≠
> "correct". Cross-checking all 37 against `issues/schema/db_audit.md` (#1519)
> showed only **13** matched the database: **22** disagree with it and **2**
> (`compiled_intents`, `safety_constraints`) are ghosts no migration creates.
> Only the 13 were safe to move; they are done (86 → 73). The other 24 are
> reconciliation work and belong to #1517/#1519, not here. **Always cross-check
> `db_audit.md` before calling a move mechanical.**

Moved (13, exact match): **botcloud (8)** `billing_customers`,
`billing_payment_methods`, `bot_domains`, `cloud_audit_log`,
`cloud_voucher_redemptions`, `cloud_vouchers`, `cloud_workspaces`,
`workspace_resources` · **botcompliance (3)** `compliance_controls`,
`compliance_frameworks`, `compliance_control_evidence` ·
**botcontacts (1)** `crm_audit_logs` · **botproducts (1)** `product_prices`.

Not moved (22 mismatched + 2 ghosts): **botautotask** `audit_log`, `auto_tasks`,
`compiled_intents`, `designer_changes`, `designer_pending_changes`,
`intent_classifications`, `safety_constraints` · **botsources**
`bank_transactions`, `connectors`, `delivery_transactions`,
`reconciliation_rules`, `reconciliation_runs` · **botpeople** `hr_clock_entries`,
`hr_holidays`, `hr_overtime_rules`, `hr_schedules`, `hr_work_periods` ·
**botproducts** `pos_sales`, `pos_sessions`, `product_price_lists`,
`product_promotions`, `product_stock`, `product_variations` · **bottasks**
`auto_tasks` · **botemail** `email_messages`.

A unique move is a *byte-identical* move → positional `Queryable` mapping is
unchanged → the lowest-risk batch available. Do botcloud + botcompliance first.

### Step B — the 14 `conflicting` tables

Need a shape decision per table, arbitrated by **`issues/schema/db_audit.md`**
(#1519, real `diesel print-schema` output), never by guessing.

- **`user_sessions` ×11** — botattendance, botbasic_types, botcorebot, botcoreoauth,
  botcoresession, botlib/src/models/session.rs, botmsteams, botpaper, bottasks,
  bottelegram, botwhatsapp. This is also **#1518** (the ten `UserSession` structs);
  one decision closes both.
- **`bot_configuration` ×7** — botbasic_types, botcorebot, botcoredirectory,
  botcoreoauth, botcoresession, botsources, botwhatsapp
- **`users` ×5** — botattendance, botcoredirectory, botcoreoauth, botwhatsapp
- **`organizations` ×4** — botbasic_types, botbilling, botcorebot, botproducts
- **`products` ×3** — botbilling, botproducts, `botserver/src/main_module/routes/catalog.rs`
- **`crm_contacts` ×3** — botcloud, botcontacts, botemail
- **`tasks` ×3** — botautotask, botcontacts, bottasks
- ×2 each: `bot_memories` (botbasic_types, botcorebot), `people` (botcontacts,
  botpeople), `price_lists` (botbilling, botproducts), `system_automations`
  (botbasic_types, botcorebot), `workflow_events` + `workflow_executions`
  (botbasic_types, botcorebot)
- ×1: `message_history` (botwhatsapp) — single copy, reconcile against db_audit

Suggested order: `message_history` → the ×2 pairs in botcorebot/botbasic_types
(one source of truth already exists in each pair) → `organizations`, `price_lists`
→ `products`, `crm_contacts`, `tasks` → `users` → `bot_configuration` →
`user_sessions` last (biggest, overlaps #1518).

## Non-negotiable mechanics (each one cost a rebuild to learn)

1. **Orphan rule E0117.** `diesel::joinable!` and
   `diesel::allow_tables_to_appear_in_same_query!` expand to
   `impl ForeignTrait for ForeignType {}`. Once a table moves to `botschema`,
   those macros **must be dropped from the consuming crate** (the canonical
   module already has them). Symptom:
   `only traits defined in the current crate can be implemented for types defined outside of the crate`.
2. **Column order is part of the contract.** `#[derive(Queryable)]` maps
   field *n* → column *n*. Equal column *sets* are not enough.
   - identical → swap is free
   - otherwise the canonical order must be set to the **consuming** order
     verbatim (verified safe only after confirming nothing outside `botschema`
     imports `botschema::<table>`), then the local block becomes
     `pub use botschema::<table>;`
3. **`botschema` does not re-export `dashboards`, `analytics`, `social` at the
   crate root** — their own same-named table would collide with the module name.
   Use the module path: `botschema::dashboards::conversational_queries`.
4. **Add the feature.** `botschema = { workspace = true, features = ["<mod>"] }`
   in the consumer's `Cargo.toml`; the feature gates the module.
5. **`cargo check` cannot see a positional mismatch.** It compiles, then mis-maps
   at runtime. Audit struct field order against the effective declaration after
   every reorder (script pattern in `botschema/README.md`, "Column order is
   part of the contract").

## Environment constraints (this box is nearly full)

- **Never build workspace-level.** `--workspace` / `--all-features` pulls the
  `allbutooxmlsdk` feature → ~1.1 GB of rlibs → ENOSPC. Check per crate:
  `cargo check -p <crate>` (add `--features mail` when code is behind `cfg`).
- Free space with `cargo clean -p ooxmlsdk` (~1.1 GB) or `cargo clean -p bottest`.
- Backgrounded `cargo` dies with the tool call → use
  `setsid nohup cargo check ... > /tmp/x.log 2>&1 < /dev/null &` then poll.
- `pkill -f "cargo check"` kills your own shell (the pattern matches the shell's
  own cmdline). Use `pkill -f "cargo [c]heck"`.

## Scripting traps (both already bit us)

- **Never `git checkout --` a file with uncommitted work.** It silently
  destroyed a whole in-flight batch; the recovery was mechanical but expensive.
  Commit before running a bulk script.
- **Apply replacements right-to-left.** Slicing several `table!` blocks with
  stale offsets corrupted `botschema/src/compliance.rs` into an unclosed
  delimiter. Collect all `(start, end, text)` per file, sort by `start`
  descending, then splice.
- Parse `table!` blocks with **brace matching**, not a regex ending in
  `\n\s*\}` — the `\s*` eats the inner closing brace and leaves an orphan `}`.
- Verify the parser before trusting it: compare the 210 canonical tables against
  the DB (`issues/schema/db_audit.md`).

## Close-out per batch

```bash
python3 scripts/schema_drift_inventory.py          # issues/schema/schema_drift.md
python3 scripts/schema_guard.py                    # must print OK, no new offenders
python3 scripts/schema_guard.py --update           # only after the count drops
cargo check -p <every crate touched>               # plus -p botserver at the end
git add -A botschema botserver/crates issues/schema Cargo.lock
git commit -m "refactor(#1517): …"
```

## Definition of done for the epic

- `schema_guard.py` baseline reaches **0**; any new `table!` outside `botschema`
  is then an immediate CI failure.
- `schema_drift_inventory.py` reports `identical 0 · additive 0 · conflicting 0`
  — every table declared once, in `botschema`.
- `db_audit.md` reports 0 mismatched canonical tables and 0 ghosts
  (currently: 171 mismatched, 128 canonical, 9 ghosts).
- #1516 / #1517 / #1518 / #1519 closed; #1497 epic closed.