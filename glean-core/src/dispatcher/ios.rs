use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use dispatch::{Queue, QueueAttribute};

use super::DispatchError;

const NOT_FLUSHED: u8 = 0;
const FLUSHING: u8 = 1;
const IS_FLUSHED: u8 = 2;
const SHUTDOWN: u8 = 3;

/// todo
pub struct Dispatcher {
    guard: Arc<DispatchGuard>,
}

impl Dispatcher {
    /// Creates a new dispatcher with a maximum queue size.
    ///
    /// Launched tasks won't run until [`flush_init`] is called.
    ///
    /// [`flush_init`]: #method.flush_init
    pub fn new(max_queue_size: usize) -> Self {
        let queue = Queue::create("glean.dispatcher", QueueAttribute::Serial);
        let preinit_queue = Mutex::new(Vec::with_capacity(10));

        let guard = DispatchGuard {
            queue,

            flushed: AtomicU8::new(NOT_FLUSHED),
            max_queue_size,
            preinit_queue,
        };

        Dispatcher {
            guard: Arc::new(guard),
        }
    }

    pub fn guard(&self) -> Arc<DispatchGuard> {
        self.guard.clone()
    }

    pub fn join(self) -> Result<(), DispatchError> {
        self.guard.block_on_queue();
        Ok(())
    }

    pub fn flush_init(&mut self) -> Result<usize, DispatchError> {
        self.guard.flush_init()
    }

    pub fn block_on_queue(&self) {
        self.guard.block_on_queue()
    }
}

/// A clonable guard for a dispatch queue.
pub struct DispatchGuard {
    queue: Queue,
    flushed: AtomicU8,
    max_queue_size: usize,
    preinit_queue: Mutex<Vec<Box<dyn FnOnce() + Send + 'static>>>,
}

impl DispatchGuard {
    pub fn launch(&self, task: impl FnOnce() + Send + 'static) -> Result<(), DispatchError> {
        if self.flushed.load(Ordering::SeqCst) == IS_FLUSHED {
            self.queue.exec_async(task);
        } else {
            let mut queue = self.preinit_queue.lock().unwrap();
            queue.push(Box::new(task))
        }

        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), DispatchError> {
        self.flush_init().ok();
        self.flushed.store(SHUTDOWN, Ordering::SeqCst);
        Ok(())
    }

    pub fn block_on_queue(&self) {
        self.queue.exec_sync(|| {
            // intentionally left empty
        });
    }

    pub fn flush_init(&self) -> Result<usize, DispatchError> {
        if let Err(_old) = self.flushed.compare_exchange(NOT_FLUSHED, FLUSHING, Ordering::Acquire, Ordering::Relaxed) {
            return Err(DispatchError::AlreadyFlushed);
        }

        let over;
        let queue_size;
        {
            let mut queue = self.preinit_queue.lock().unwrap();
            queue_size = queue.len();
            over = queue_size.saturating_sub(self.max_queue_size);
            let end = std::cmp::min(queue_size, self.max_queue_size);

            for task in queue.drain(..end) {
                self.queue.exec_sync(task);
            }
        }
        // give others a chance to get the lock
        {
            let mut queue = self.preinit_queue.lock().unwrap();
            if queue.len() > over {
                let start = queue.len() - over;
                for task in queue.drain(start..) {
                    self.queue.exec_sync(task);
                }
            }
        }

        self.flushed.store(IS_FLUSHED, Ordering::SeqCst);
        Ok(over)
    }

    pub fn kill(&self) -> Result<(), DispatchError> {
        Ok(())
    }
}
