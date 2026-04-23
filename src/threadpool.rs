use std::{
    ops::Index,
    sync::{
        Arc, Condvar, Mutex,
        atomic::Ordering,
        mpsc::{Receiver, SyncSender},
    },
    thread::Scope,
};

use crate::{
    board::Board,
    numa::NumaReplicatedAccessToken,
    search::{self, Report},
    thread::{SharedContext, Status, ThreadData},
    time::TimeManager,
};

pub struct ThreadPool {
    pub workers: Vec<WorkerThread>,
    pub vector: Vec<ThreadData>,
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
        shared.numa_context.set_thread_count(1);

        let workers = make_worker_threads(1);
        let data = make_thread_data(shared, &workers);

        Self { workers, vector: data }
    }

    pub fn set_count(&mut self, threads: usize) {
        let threads = threads.clamp(1, ThreadPool::available_threads());
        let shared = self.vector[0].shared.clone();

        shared.numa_context.set_thread_count(threads);

        self.workers.drain(..).for_each(WorkerThread::join);
        self.workers = make_worker_threads(threads);

        std::mem::drop(self.vector.drain(..));
        self.vector = make_thread_data(shared, &self.workers);
    }

    pub fn main_thread(&mut self) -> &mut ThreadData {
        &mut self.vector[0]
    }

    pub const fn len(&self) -> usize {
        self.vector.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ThreadData> {
        self.vector.iter()
    }

    pub fn clear(&mut self) {
        let shared = self.vector[0].shared.clone();

        shared.numa_context.set_thread_count(self.workers.len());

        std::mem::drop(self.vector.drain(..));
        self.vector = make_thread_data(shared, &self.workers);
    }

    pub fn execute_searches(
        &mut self, time_manager: TimeManager, report: Report, multi_pv: usize, board: &Board,
        shared: &Arc<SharedContext>,
    ) {
        shared.tt.increment_age();

        shared.nodes.reset();
        shared.tb_hits.reset();
        shared.soft_stop_votes.store(0, Ordering::Release);
        shared.status.set(Status::RUNNING);
        shared.best_stats.iter().for_each(|x| {
            x.store((self.main_thread().previous_best_score + 32768) as u32, Ordering::Release);
        });

        std::thread::scope(|scope| {
            let mut handlers = Vec::new();

            let thread_count = self.vector.len();

            let (t1, rest) = self.vector.split_first_mut().unwrap();
            let (w1, rest_workers) = self.workers.split_first().unwrap();

            let tm = time_manager.clone();
            handlers.push(scope.spawn_into(
                move || {
                    t1.multi_pv = multi_pv;
                    t1.time_manager = tm;
                    t1.board = (*board).clone();

                    search::start(t1, report, thread_count);
                    shared.status.set(Status::STOPPED);
                },
                w1,
            ));

            for (index, (t, w)) in rest.iter_mut().zip(rest_workers).enumerate() {
                let tm = time_manager.clone();
                handlers.push(scope.spawn_into(
                    move || {
                        t.id = index + 1;
                        t.time_manager = tm;
                        t.board = (*board).clone();

                        search::start(t, Report::None, thread_count);
                    },
                    w,
                ));
            }

            for handler in handlers {
                handler.join();
            }
        });
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

fn make_worker_thread() -> WorkerThread {
    let (sender, receiver) = make_work_channel();

    let handle = std::thread::spawn(move || {
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
    std::iter::repeat_with(make_worker_thread).take(num_threads).collect()
}

fn make_thread_data(shared: Arc<SharedContext>, worker_threads: &[WorkerThread]) -> Vec<ThreadData> {
    std::thread::scope(|scope| -> Vec<ThreadData> {
        let cfg = shared.numa_context.get_numa_config();
        let should_bind = cfg.suggests_binding_threads(worker_threads.len());
        let numa_nodes = cfg.distribute_threads_among_numa_nodes(worker_threads.len());

        let handles = worker_threads
            .iter()
            .enumerate()
            .map(|(index, worker)| {
                let (tx, rx) = std::sync::mpsc::channel();
                let shared = shared.clone();
                let cfg = cfg.clone();
                let numa_node = numa_nodes[index];
                let join_handle = scope.spawn_into(
                    move || {
                        let token = if should_bind {
                            cfg.bind_current_thread_to_numa_node(numa_node)
                        } else {
                            NumaReplicatedAccessToken::new(0)
                        };
                        tx.send(Box::new(ThreadData::new(shared, token))).unwrap();
                    },
                    worker,
                );
                (rx, join_handle)
            })
            .collect::<Vec<_>>();

        let mut thread_data: Vec<ThreadData> = Vec::with_capacity(handles.len());
        for (rx, handle) in handles {
            let td = rx.recv().unwrap();
            thread_data.push(*td);
            handle.join();
        }

        thread_data
    })
}
