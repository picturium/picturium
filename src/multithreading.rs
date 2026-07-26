use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::available_parallelism;
use tokio::sync::Semaphore;
use crate::config::SharedConfig;

#[derive(Clone, Debug)]
pub struct MultiThreading {
    pub total_workers: usize,
    available_workers: Arc<AtomicUsize>,
    available_queue_size: Arc<AtomicUsize>,
    semaphore: Arc<Semaphore>,
}

impl MultiThreading {
    pub fn new(config: &SharedConfig) -> Self {
        let total_workers = Self::calculate_total_workers_number(config);
        let semaphore = Arc::new(Semaphore::new(total_workers));

        Self {
            total_workers,
            available_workers: Arc::new(AtomicUsize::new(total_workers)),
            available_queue_size: Arc::new(AtomicUsize::new(config.server.queue_size)),
            semaphore,
        }
    }

    pub fn get_available_workers(&self) -> usize {
        self.available_workers.load(Ordering::Relaxed)
    }

    pub fn get_available_queue_size(&self) -> usize {
        self.available_queue_size.load(Ordering::Relaxed)
    }

    pub async fn get_permit(&'_ self) -> Option<tokio::sync::SemaphorePermit<'_>> {
        if !self.queue_request() {
            return None;
        }

        let permit = self.semaphore.acquire().await.unwrap();

        self.release_queue();
        self.reserve_worker();

        Some(permit)
    }

    fn queue_request(&self) -> bool {
        let available = self.available_queue_size.load(Ordering::Relaxed);

        if available == 0 {
            return false;
        }

        self.available_queue_size.fetch_sub(1, Ordering::Relaxed);
        true
    }

    fn release_queue(&self) {
        self.available_queue_size.fetch_add(1, Ordering::Relaxed);
    }

    fn reserve_worker(&self) {
        self.available_workers.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn release_worker(&self) {
        self.available_workers.fetch_add(1, Ordering::Relaxed);
    }

    fn calculate_total_workers_number(config: &SharedConfig) -> usize {
        match config.server.workers {
            workers if workers > 0 => workers,
            _ => available_parallelism().unwrap_or(NonZeroUsize::new(1).unwrap()).get() * 2,
        }
    }
}
