use super::*;

pub(super) fn resolve_branch(headers: &HeaderMap) -> Uuid {
    branch_from_claims(headers).unwrap_or_else(Uuid::nil)
}
pub(super) fn ok<T: Serialize>(value: T) -> Result<Json<T>, (StatusCode, String)> {
    Ok(Json(value))
}
pub(super) fn decimal_to_string(d: Decimal) -> String {
    d.to_string()
}

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------
