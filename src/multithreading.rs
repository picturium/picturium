use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, PoisonError};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread::available_parallelism;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, SemaphorePermit};
use crate::config::SharedConfig;

#[derive(Clone, Debug)]
pub struct MultiThreading {
    pub total_workers: usize,
    available_workers: Arc<AtomicUsize>,
    available_queue_size: Arc<AtomicUsize>,
    semaphore: Arc<Semaphore>,
    jobs: Arc<Mutex<HashMap<u64, Job>>>,
    next_job: Arc<AtomicU64>,
}

#[derive(Debug)]
struct Job {
    uri: String,
    started: Instant,
}

pub struct Worker<'a> {
    _permit: SemaphorePermit<'a>,
    threading: &'a MultiThreading,
    id: u64,
}

impl Drop for Worker<'_> {
    fn drop(&mut self) {
        self.threading.jobs.lock().unwrap_or_else(PoisonError::into_inner).remove(&self.id);
        self.threading.available_workers.fetch_add(1, Ordering::Relaxed);
    }
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
            jobs: Arc::new(Mutex::new(HashMap::new())),
            next_job: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn get_available_workers(&self) -> usize {
        self.available_workers.load(Ordering::Relaxed)
    }

    pub fn get_available_queue_size(&self) -> usize {
        self.available_queue_size.load(Ordering::Relaxed)
    }

    /// Returns running jobs as `(uri, running for)`, oldest first.
    pub fn get_active_jobs(&self) -> Vec<(String, Duration)> {
        let mut jobs: Vec<_> = self.jobs.lock().unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(|job| (job.uri.clone(), job.started.elapsed()))
            .collect();

        jobs.sort_by(|a, b| b.1.cmp(&a.1));
        jobs
    }

    pub async fn get_worker(&self, uri: String) -> Option<Worker<'_>> {
        if !self.queue_request() {
            return None;
        }

        // The wait is canceled when the client disconnects, the slot must still be returned
        let queued = scopeguard::guard((), |_| self.release_queue());
        let permit = self.semaphore.acquire().await.unwrap();
        drop(queued);

        let id = self.next_job.fetch_add(1, Ordering::Relaxed);
        self.jobs.lock().unwrap_or_else(PoisonError::into_inner).insert(id, Job { uri, started: Instant::now() });
        self.available_workers.fetch_sub(1, Ordering::Relaxed);

        Some(Worker { _permit: permit, threading: self, id })
    }

    fn queue_request(&self) -> bool {
        self.available_queue_size
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |available| available.checked_sub(1))
            .is_ok()
    }

    fn release_queue(&self) {
        self.available_queue_size.fetch_add(1, Ordering::Relaxed);
    }

    fn calculate_total_workers_number(config: &SharedConfig) -> usize {
        match config.server.workers {
            workers if workers > 0 => workers,
            _ => available_parallelism().unwrap_or(NonZeroUsize::new(1).unwrap()).get() * 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn worker_tracks_and_releases_its_job() {
        let mut config = Config::default();
        config.server.workers = 2;
        let threading = MultiThreading::new(&Arc::new(config));

        let worker = threading.get_worker("/a.jpg?w=100".into()).await.unwrap();
        assert_eq!(threading.get_available_workers(), 1);
        assert_eq!(threading.get_active_jobs()[0].0, "/a.jpg?w=100");

        drop(worker);
        assert_eq!(threading.get_available_workers(), 2);
        assert!(threading.get_active_jobs().is_empty());
    }
}
