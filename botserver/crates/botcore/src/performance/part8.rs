use super::*;

impl<T: Clone + Send + Sync + 'static> BatchProcessor<T> {
    pub fn new<F, Fut>(batch_size: usize, flush_interval_ms: u64, processor: F) -> Self
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let processor_arc: BatchProcessorFunc<T> =
            Arc::new(move |items| Box::pin(processor(items)));

        let batch_processor = Self {
            batch_size,
            buffer: Arc::new(RwLock::new(Vec::new())),
            processor: processor_arc,
        };

        let buffer = batch_processor.buffer.clone();
        let processor = batch_processor.processor.clone();
        let batch_size_clone = batch_size;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_millis(flush_interval_ms)
            );
            loop {
                interval.tick().await;
                let mut buf = buffer.write().await;
                if !buf.is_empty() {
                    let items: Vec<T> = buf.drain(..).collect();
                    drop(buf);

                    for chunk in items.chunks(batch_size_clone) {
                        processor(chunk.to_vec()).await;
                    }
                }
            }
        });

        batch_processor
    }

    pub async fn add(&self, item: T) {
        let mut buffer = self.buffer.write().await;
        buffer.push(item);

        if buffer.len() >= self.batch_size {
            let items: Vec<T> = buffer.drain(..).collect();
            drop(buffer);
            (self.processor)(items).await;
        }
    }

    pub async fn flush(&self) {
        let mut buffer = self.buffer.write().await;
        if !buffer.is_empty() {
            let items: Vec<T> = buffer.drain(..).collect();
            drop(buffer);
            (self.processor)(items).await;
        }
    }

    pub async fn pending_count(&self) -> usize {
        let buffer = self.buffer.read().await;
        buffer.len()
    }
}
