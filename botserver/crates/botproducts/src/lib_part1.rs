use super::*;

pub fn get_bot_context(pool: &DbPool, get_default_bot: &Option<GetDefaultBotFn>) -> Uuid {
    let Ok(mut conn) = pool.get() else {
        return Uuid::nil();
    };
    resolve_branch_id(&mut conn, get_default_bot)
}
pub(crate) fn resolve_branch_id(
    conn: &mut diesel::PgConnection,
    get_default_bot: &Option<GetDefaultBotFn>,
) -> Uuid {
    match get_default_bot {
        Some(f) => {
            let bid = f(conn);
            if bid == Uuid::nil() {
                Uuid::nil()
            } else {
                bid
            }
        }
        None => Uuid::nil(),
    }
}

/// Owner organisation of a branch, from the `branches` row itself.
///
/// `services.org_id` and `services.bot_id` are NOT NULL since
/// `6.0.10-01-products`, so every insert has to supply both; like
/// `botpeople`/`botcontacts` this returns the nil UUID when the branch cannot
/// be resolved rather than failing the request.
pub(crate) fn org_for_branch(pool: &DbPool, branch_id: Uuid) -> Uuid {
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        org_id: Uuid,
    }
    let Ok(mut conn) = pool.get() else {
        return Uuid::nil();
    };
    diesel::sql_query("SELECT org_id FROM branches WHERE id = $1 LIMIT 1")
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .get_result::<Row>(&mut conn)
        .map(|r| r.org_id)
        .unwrap_or_else(|_| Uuid::nil())
}

/// The branch's default bot, falling back to the branch's oldest bot.
pub(crate) fn bot_for_branch(pool: &DbPool, branch_id: Uuid) -> Uuid {
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }
    let Ok(mut conn) = pool.get() else {
        return Uuid::nil();
    };
    diesel::sql_query(
        "SELECT id FROM bots WHERE branch_id = $1 \
         ORDER BY is_default_for_branch DESC, created_at ASC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .get_result::<Row>(&mut conn)
    .map(|r| r.id)
    .unwrap_or_else(|_| Uuid::nil())
}
