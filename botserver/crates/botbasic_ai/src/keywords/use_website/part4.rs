use super::*;

pub fn get_active_websites_for_session(
    conn_pool: &botcore::shared::utils::DbPool,
    session_id: Uuid,
) -> Result<Vec<(String, String)>, String> {
    let mut conn = conn_pool
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    #[derive(QueryableByName, Debug)]
    struct ActiveWebsiteResult {
        #[diesel(sql_type = diesel::sql_types::Text)]
        website_url: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        collection_name: String,
    }

    let results: Vec<ActiveWebsiteResult> = diesel::sql_query(
        "SELECT website_url, collection_name
         FROM session_website_associations
         WHERE session_id = $1 AND is_active = true
         ORDER BY added_at DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .load(&mut conn)
    .map_err(|e| format!("Failed to get active websites: {}", e))?;

    Ok(results
        .into_iter()
        .map(|r| (r.website_url, r.collection_name))
        .collect())
}
pub(super) fn sanitize_url_for_collection(url: &str) -> String {
    url.replace("http://", "")
        .replace("https://", "")
        .replace(['/', ':', '.'], "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>()
        .to_lowercase()
}
