use super::*;

pub fn set_embedding_server_ready(ready: bool) {
    EMBEDDING_SERVER_READY.store(ready, Ordering::SeqCst);
}
pub fn is_embedding_server_ready() -> bool {
    EMBEDDING_SERVER_READY.load(Ordering::SeqCst)
}
