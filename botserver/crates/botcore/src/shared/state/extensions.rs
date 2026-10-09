use super::*;

impl Extensions {
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn insert<T: Send + Sync + 'static>(&self, value: T) {
        let mut map = self.map.write().await;
        map.insert(std::any::TypeId::of::<T>(), Arc::new(value));
    }

    pub fn insert_blocking<T: Send + Sync + 'static>(&self, value: T) {
        let map = self.map.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            if let Ok(rt) = rt {
                rt.block_on(async {
                    let mut guard = map.write().await;
                    guard.insert(std::any::TypeId::of::<T>(), Arc::new(value));
                });
            }
            let _ = tx.send(());
        });
        let _ = rx.recv();
    }

    pub async fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let map = self.map.read().await;
        map.get(&std::any::TypeId::of::<T>())
            .and_then(|boxed| Arc::clone(boxed).downcast::<T>().ok())
    }

    pub async fn contains<T: Send + Sync + 'static>(&self) -> bool {
        let map = self.map.read().await;
        map.contains_key(&std::any::TypeId::of::<T>())
    }

    pub async fn remove<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let mut map = self.map.write().await;
        map.remove(&std::any::TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast::<T>().ok())
    }

    pub async fn len(&self) -> usize {
        let map = self.map.read().await;
        map.len()
    }

    pub async fn is_empty(&self) -> bool {
        let map = self.map.read().await;
        map.is_empty()
    }
}

impl std::fmt::Debug for Extensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extensions").finish_non_exhaustive()
    }
}
