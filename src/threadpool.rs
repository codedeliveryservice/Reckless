use std::{
    ops::Index,
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{Receiver, SyncSender},
    },
    thread::Scope,
};

use crate::{
    search::{self, Report},
    thread::{SharedContext, Status, ThreadData},
    time::{Limits, TimeManager},
};

pub struct ThreadPool {
    pub workers: Vec<WorkerThread>,
    pub vector: Vec<Box<ThreadData>>,
}

impl ThreadPool {
    pub fn available_threads() -> usize {
        const MINIMUM_THREADS: usize = 512;

        match std::thread::available_parallelism() {
            Ok(threads) => (4 * threads.get()).max(MINIMUM_THREADS),
            Err(_) => MINIMUM_THREADS,
        }
    }

    pub fn new(shared: Arc<SharedContext>) -> Self {
        let workers = make_worker_threads(1);
        let data = make_thread_data(shared, &workers);

        Self { workers, vector: data }
    }

    pub fn set_count(&mut self, threads: usize) {
        let shared = self.vector[0].shared.clone();

        self.workers.drain(..).for_each(WorkerThread::join);
        self.workers = make_worker_threads(threads);

        std::mem::drop(self.vector.drain(..));
        self.vector = make_thread_data(shared, &self.workers);
    }

    pub fn main_thread(&mut self) -> &mut ThreadData {
        &mut self.vector[0]
    }

    pub fn len(&self) -> usize {
        self.vector.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Box<ThreadData>> {
        self.vector.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<ThreadData>> {
        self.vector.iter_mut()
    }

    pub fn clear(&mut self) {
        let shared = self.vector[0].shared.clone();

        std::mem::drop(self.vector.drain(..));
        self.vector = make_thread_data(shared, &self.workers);
    }

    pub fn execute_searches(&mut self, time_manager: TimeManager, report: Report, shared: &Arc<SharedContext>) {
        shared.tt.increment_age();

        shared.nodes.reset();
        shared.tb_hits.reset();
        shared.status.set(Status::RUNNING);

        std::thread::scope(|scope| {
            let mut handlers = Vec::new();

            let (t1, rest) = self.vector.split_first_mut().unwrap();
            let (w1, rest_workers) = self.workers.split_first().unwrap();

            handlers.push(scope.spawn_into(
                || {
                    t1.time_manager = time_manager;

                    search::start(t1, report);
                    shared.status.set(Status::STOPPED);
                },
                w1,
            ));

            for (index, (t, w)) in rest.iter_mut().zip(rest_workers).enumerate() {
                handlers.push(scope.spawn_into(
                    move || {
                        t.id = index + 1;
                        t.time_manager = TimeManager::new(Limits::Infinite, 0, 0);

                        search::start(t, Report::None);
                    },
                    w,
                ));
            }

            for handler in handlers {
                handler.join();
            }
        })
    }
}

impl Index<usize> for ThreadPool {
    type Output = ThreadData;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vector[index]
    }
}

pub struct WorkerThread {
    handle: std::thread::JoinHandle<()>,
    comms: WorkSender,
}

impl WorkerThread {
    pub fn join(self) {
        drop(self.comms); // Drop the sender to signal the worker thread to finish
        self.handle.join().expect("Worker thread panicked");
    }
}

// Handle for communicating with a worker thread.
// Contains a sender for sending messages to the worker thread,
// and a receiver for receiving messages from the worker thread.
struct WorkSender {
    // INVARIANT: Each send must be matched by a receive.
    sender: SyncSender<Box<dyn FnOnce() + Send>>,
    completion_signal: Arc<(Mutex<bool>, Condvar)>,
}

/// Handle for the receiver side of a worker thread.
struct WorkReceiver {
    receiver: Receiver<Box<dyn FnOnce() + Send>>,
    completion_signal: Arc<(Mutex<bool>, Condvar)>,
}

fn make_work_channel() -> (WorkSender, WorkReceiver) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(0);
    let completion_signal = Arc::new((Mutex::new(false), Condvar::new()));

    (
        WorkSender { sender, completion_signal: Arc::clone(&completion_signal) },
        WorkReceiver { receiver, completion_signal },
    )
}

pub struct ReceiverHandle<'scope> {
    completion_signal: &'scope Arc<(Mutex<bool>, Condvar)>,
    received: bool,
}

impl ReceiverHandle<'_> {
    pub fn join(mut self) {
        let (lock, cvar) = &**self.completion_signal;
        let mut completed = lock.lock().unwrap();
        while !*completed {
            completed = cvar.wait(completed).unwrap();
        }
        drop(completed);
        self.received = true;
    }
}

impl Drop for ReceiverHandle<'_> {
    fn drop(&mut self) {
        // When the receiver handle is dropped, we ensure that we have received something.
        assert!(self.received, "ReceiverHandle was dropped without receiving a value");
    }
}

pub trait ScopeExt<'scope, 'env> {
    fn spawn_into<F>(&'scope self, f: F, comms: &'scope WorkerThread) -> ReceiverHandle<'scope>
    where
        F: FnOnce() + Send + 'scope;
}

impl<'scope, 'env> ScopeExt<'scope, 'env> for Scope<'scope, 'env> {
    fn spawn_into<'comms, F>(&'scope self, f: F, thread: &'scope WorkerThread) -> ReceiverHandle<'scope>
    where
        F: FnOnce() + Send + 'scope,
    {
        // Safety: This file is structured such that threads never hold the data longer than is permissible.
        let f = unsafe {
            std::mem::transmute::<Box<dyn FnOnce() + Send + 'scope>, Box<dyn FnOnce() + Send + 'static>>(Box::new(f))
        };

        // Reset the completion flag before sending the task
        {
            let (lock, _) = &*thread.comms.completion_signal;
            let mut completed = lock.lock().unwrap();
            *completed = false;
        }

        thread.comms.sender.send(f).expect("Failed to send function to worker thread");

        ReceiverHandle {
            completion_signal: &thread.comms.completion_signal,
            // Important: We start with `received` as false.
            received: false,
        }
    }
}

fn make_worker_thread(id: Option<usize>) -> WorkerThread {
    let (sender, receiver) = make_work_channel();

    let handle = std::thread::spawn(move || {
        #[cfg(feature = "numa")]
        if let Some(id) = id {
            crate::numa::bind_thread(id);
        }
        #[cfg(not(feature = "numa"))]
        let _ = id;

        while let Ok(work) = receiver.receiver.recv() {
            work();
            let (lock, cvar) = &*receiver.completion_signal;
            let mut completed = lock.lock().unwrap();
            *completed = true;
            drop(completed); // Release the lock before notifying
            cvar.notify_one();
        }
    });

    WorkerThread { handle, comms: sender }
}

fn make_worker_threads(num_threads: usize) -> Vec<WorkerThread> {
    #[cfg(feature = "numa")]
    {
        let concurrency = std::thread::available_parallelism().map_or(1, |n| n.get());
        (0..num_threads).map(|id| make_worker_thread((num_threads >= concurrency / 2).then_some(id))).collect()
    }
    #[cfg(not(feature = "numa"))]
    {
        (0..num_threads).map(|_| make_worker_thread(None)).collect()
    }
}

fn make_thread_data(shared: Arc<SharedContext>, worker_threads: &[WorkerThread]) -> Vec<Box<ThreadData>> {
    std::thread::scope(|scope| -> Vec<Box<ThreadData>> {
        let handles = worker_threads
            .iter()
            .map(|worker| {
                let (tx, rx) = std::sync::mpsc::channel();
                let shared = shared.clone();
                let join_handle = scope.spawn_into(
                    move || {
                        tx.send(Box::new(ThreadData::new(shared))).unwrap();
                    },
                    worker,
                );
                (rx, join_handle)
            })
            .collect::<Vec<_>>();

        let mut thread_data: Vec<Box<ThreadData>> = Vec::with_capacity(handles.len());
        for (rx, handle) in handles {
            let td = rx.recv().unwrap();
            thread_data.push(td);
            handle.join();
        }

        thread_data
    })
}
