//! Persistence for `app_permissions` grants and the `consent_audit` trail.
//!
//! A grant is *effective* when `granted = true` and its (possibly capped)
//! expiry is in the future. Sensitive classes (see
//! [`crate::models::ALWAYS_REPROMPT`]) have their stored expiry clamped to the
//! end of the current UTC month, which forces a fresh prompt every cycle even
//! after an "always" decision.

use chrono::{DateTime, Datelike, Utc};
use chrono::TimeZone;
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use crate::models::{is_reprompt_class, AppPermissionRow, ConsentAuditRow};
use crate::schema::{app_permissions, consent_audit};

/// Outcome label written to `consent_audit.outcome`.
pub const OUTCOME_GRANTED: &str = "granted";
pub const OUTCOME_PENDING: &str = "pending";
pub const OUTCOME_DENIED: &str = "denied";

/// Last instant of the current UTC month (first instant of the next month).
pub fn end_of_month_utc(now: DateTime<Utc>) -> DateTime<Utc> {
    let (year, month) = (now.year(), now.month());
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    match Utc.with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0) {
        chrono::LocalResult::Single(t) => t,
        _ => now,
    }
}

/// Effective remaining lifetime of a grant at `now`, applying the sensitive
/// class cycle cap. Returns `None` when the grant must be re-prompted.
pub fn effective_expiry(row: &AppPermissionRow, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !row.granted {
        return None;
    }
    let raw = row.expires_at?;
    let capped = if is_reprompt_class(&row.action_class) {
        let cycle_end = end_of_month_utc(now);
        if raw > cycle_end {
            cycle_end
        } else {
            raw
        }
    } else {
        raw
    };
    if capped > now {
        Some(capped)
    } else {
        None
    }
}

/// Parameters for [`grant`] bundled to keep the signature compact.
pub struct GrantSpec<'a> {
    pub user_id: Uuid,
    pub app_id: &'a str,
    pub action_class: &'a str,
    pub scope: &'a Value,
    pub granted_via: &'a str,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Inserts a new grant or refreshes the existing one for the unique
/// `(user_id, app_id, action_class)` triple. Sensitive classes without an
/// explicit expiry get end-of-month UTC so "always" never outlives one cycle.
pub fn grant(
    conn: &mut PgConnection,
    spec: GrantSpec<'_>,
) -> Result<AppPermissionRow, String> {
    let now = Utc::now();
    let expires_at = spec.expires_at.or_else(|| {
        is_reprompt_class(spec.action_class).then(|| end_of_month_utc(now))
    });
    let new_row = AppPermissionRow {
        id: Uuid::new_v4(),
        user_id: spec.user_id,
        org_id: None,
        branch_id: None,
        app_id: spec.app_id.to_string(),
        action_class: spec.action_class.to_string(),
        scope: spec.scope.clone(),
        granted: true,
        granted_via: spec.granted_via.to_string(),
        expires_at,
        granted_at: now,
    };

    diesel::insert_into(app_permissions::table)
        .values(&new_row)
        .on_conflict((
            app_permissions::user_id,
            app_permissions::app_id,
            app_permissions::action_class,
        ))
        .do_update()
        .set((
            app_permissions::scope.eq(spec.scope),
            app_permissions::granted.eq(true),
            app_permissions::granted_via.eq(spec.granted_via),
            app_permissions::expires_at.eq(expires_at),
            app_permissions::granted_at.eq(now),
        ))
        .returning(app_permissions::all_columns)
        .get_result(conn)
        .map_err(|e| format!("grant upsert failed: {e}"))
}

/// Revokes one owned grant. Returns `true` when a row was deleted.
pub fn revoke(conn: &mut PgConnection, permission_id: Uuid, owner_user_id: Uuid) -> Result<bool, String> {
    use app_permissions::dsl;
    let deleted = diesel::delete(
        app_permissions::dsl::app_permissions
            .filter(dsl::id.eq(permission_id))
            .filter(dsl::user_id.eq(owner_user_id)),
    )
    .execute(conn)
    .map_err(|e| format!("revoke failed: {e}"))?;
    Ok(deleted > 0)
}

/// Lists every stored grant (including expired/revoked rows) for a user.
pub fn list_for_user(conn: &mut PgConnection, user_id: Uuid) -> Result<Vec<AppPermissionRow>, String> {
    use app_permissions::dsl;
    app_permissions::dsl::app_permissions
        .filter(dsl::user_id.eq(user_id))
        .order(dsl::granted_at.desc())
        .load::<AppPermissionRow>(conn)
        .map_err(|e| format!("list grants failed: {e}"))
}

/// Returns the currently effective grant for `(user, app, class)`, honoring
/// `granted = true` and the sensitive-class expiry recompute. `None` means a
/// prompt (or denial) is required.
pub fn effective_grant(
    conn: &mut PgConnection,
    user_id: Uuid,
    app_id: &str,
    action_class: &str,
) -> Result<Option<AppPermissionRow>, String> {
    use app_permissions::dsl;
    let row = dsl::app_permissions
        .filter(dsl::user_id.eq(user_id))
        .filter(dsl::app_id.eq(app_id))
        .filter(dsl::action_class.eq(action_class))
        .filter(dsl::granted.eq(true))
        .first::<AppPermissionRow>(conn)
        .optional()
        .map_err(|e| format!("grant lookup failed: {e}"))?;
    let now = Utc::now();
    Ok(row.filter(|r| effective_expiry(r, now).is_some()))
}

/// Appends one audit entry; audit failures are logged and never abort the
/// enforcement flow.
pub fn audit(
    conn: &mut PgConnection,
    permission_id: Option<Uuid>,
    user_id: Option<Uuid>,
    request: &Value,
    outcome: &str,
    decided_by: Option<Uuid>,
) {
    let row = ConsentAuditRow {
        id: Uuid::new_v4(),
        permission_id,
        user_id,
        request: request.clone(),
        outcome: outcome.to_string(),
        decided_by,
        created_at: Utc::now(),
    };
    if let Err(e) =
        diesel::insert_into(consent_audit::table).values(&row).execute(conn)
    {
        tracing::error!("consent audit insert failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(class: &str, granted: bool, expires_in_secs: i64) -> AppPermissionRow {
        AppPermissionRow {
            id: Uuid::new_v4(),
            user_id: Uuid::nil(),
            org_id: None,
            branch_id: None,
            app_id: "crm".to_string(),
            action_class: class.to_string(),
            scope: serde_json::json!({}),
            granted,
            granted_via: "prompt".to_string(),
            expires_at: Some(Utc::now() + chrono::Duration::seconds(expires_in_secs)),
            granted_at: Utc::now(),
        }
    }

    #[test]
    fn expiry_is_honored() {
        let now = Utc::now();
        assert!(effective_expiry(&row("read", true, 3600), now).is_some());
        assert!(effective_expiry(&row("read", true, -1), now).is_none());
        assert!(effective_expiry(&row("read", true, 0), now).is_none());
        assert!(effective_expiry(&row("read", false, 3600), now).is_none());
    }

    #[test]
    fn payment_is_capped_to_current_cycle() {
        let now = Utc::now();
        let long_lived = row("payment", true, 90 * 24 * 3600);
        let capped = effective_expiry(&long_lived, now).expect("capped expiry present");
        assert_eq!(capped, end_of_month_utc(now));

        let long_lived_read = row("read", true, 90 * 24 * 3600);
        let untouched = effective_expiry(&long_lived_read, now).expect("expiry present");
        assert_ne!(untouched, end_of_month_utc(now));
    }

    #[test]
    fn payment_within_cycle_survives() {
        let now = Utc::now();
        let soon = row("payment", true, 600);
        let kept = effective_expiry(&soon, now).expect("within cycle");
        assert_eq!(Some(kept), soon.expires_at);
    }

    #[test]
    fn month_rollover_december() {
        let dec = Utc.with_ymd_and_hms(2026, 12, 15, 10, 0, 0).unwrap();
        let end = end_of_month_utc(dec);
        assert_eq!(end.year(), 2027);
        assert_eq!(end.month(), 1);
        let jan = Utc.with_ymd_and_hms(2026, 1, 31, 23, 59, 59).unwrap();
        assert_eq!(end_of_month_utc(jan).month(), 2);
    }

    #[test]
    fn payment_grant_without_expiry_gets_cycle_end() {
        let now = Utc::now();
        let mut no_expiry = row("payment", true, 3600);
        no_expiry.expires_at = None;
        assert!(effective_expiry(&no_expiry, now).is_none());

        let explicit = row("payment", true, 3600);
        assert_eq!(
            effective_expiry(&explicit, now),
            Some(explicit.expires_at.expect("set by helper"))
        );
    }
}
