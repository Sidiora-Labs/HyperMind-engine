use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Semaphore;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtractionLimits {
    pub maximum_parallel: usize,
    pub stop_dispatch_on_failure: bool,
}

impl ExtractionLimits {
    #[must_use]
    pub const fn new(maximum_parallel: usize, stop_dispatch_on_failure: bool) -> Self {
        Self {
            maximum_parallel,
            stop_dispatch_on_failure,
        }
    }

    #[must_use]
    pub const fn permits(self) -> usize {
        if self.maximum_parallel == 0 {
            1
        } else {
            self.maximum_parallel
        }
    }
}

#[derive(Debug)]
pub struct ExtractionOutcomes<T, E> {
    pub extracted: Vec<(usize, T)>,
    pub failed: Vec<(usize, E)>,
    pub skipped: Vec<usize>,
    pub aborted: Vec<usize>,
}

impl<T, E> ExtractionOutcomes<T, E> {
    #[must_use]
    pub fn total(&self) -> usize {
        self.extracted.len() + self.failed.len() + self.skipped.len() + self.aborted.len()
    }
}

pub async fn extract_bounded<I, T, E, F>(
    items: Vec<I>,
    limits: ExtractionLimits,
    work: F,
) -> ExtractionOutcomes<T, E>
where
    I: Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
    F: Fn(I) -> Result<T, E> + Send + Sync + 'static,
{
    let slots = Arc::new(Semaphore::new(limits.permits()));
    let stopped = Arc::new(AtomicBool::new(false));
    let work = Arc::new(work);
    let mut running = Vec::with_capacity(items.len());
    let mut skipped = Vec::new();
    let mut sources = items.into_iter().enumerate();
    loop {
        let Some((index, item)) = sources.next() else {
            break;
        };
        let Ok(permit) = Arc::clone(&slots).acquire_owned().await else {
            skipped.push(index);
            skipped.extend(sources.by_ref().map(|(pending, _)| pending));
            break;
        };
        if limits.stop_dispatch_on_failure && stopped.load(Ordering::Acquire) {
            drop(permit);
            skipped.push(index);
            skipped.extend(sources.by_ref().map(|(pending, _)| pending));
            break;
        }
        let item_work = Arc::clone(&work);
        let item_stopped = Arc::clone(&stopped);
        running.push((
            index,
            tokio::task::spawn_blocking(move || {
                let outcome = item_work(item);
                if outcome.is_err() {
                    item_stopped.store(true, Ordering::Release);
                }
                drop(permit);
                outcome
            }),
        ));
    }
    let mut extracted = Vec::with_capacity(running.len());
    let mut failed = Vec::new();
    let mut aborted = Vec::new();
    for (index, task) in running {
        match task.await {
            Ok(Ok(value)) => extracted.push((index, value)),
            Ok(Err(error)) => failed.push((index, error)),
            Err(_) => aborted.push(index),
        }
    }
    extracted.sort_by_key(|(index, _)| *index);
    failed.sort_by_key(|(index, _)| *index);
    skipped.sort_unstable();
    aborted.sort_unstable();
    ExtractionOutcomes {
        extracted,
        failed,
        skipped,
        aborted,
    }
}
