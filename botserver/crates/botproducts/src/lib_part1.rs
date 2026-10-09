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
