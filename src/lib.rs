// src/lib.rs
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    // use Option to release ownership at shutdown
    sender: Option<mpsc::Sender<Job>>,
}

// struct Job;
type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

// 1. When spawn a thread, it runs the worker code, each worker has id
// 2. the worker code runs a loop on polling the request queue
// 3. the request is processed by invoking the closure
//
// use Worker to send closure to thread for execution
// each worker holds its JoinHandle
// therefore the threadpool is holding a vector of workers
//
// internally, threadpool uses a queue to manage requests
// and in rust we use channel for thread-safe queue
// one-producer-many-consumers
// 1. threadpool creates a channel and hold on to the sender
// 2. worker hold on to the receiver
// 3. struct Job to hold the closure sent down the channel
// 4. the execute method will send the job through the sender
// 5. in worker thread, worker will loop over its receiver and
// executes the closure
impl ThreadPool {
    /// Creates a new ThreadPool.
    ///
    /// The size is the number of threads in that pool.
    ///
    /// # Panics
    /// The `new` function will panic if size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        // channel is multi-producer, single-consumer
        let (sender, receiver) = mpsc::channel();

        // wrap around receiver with mutex and shared_ptr
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // create some threads and store them in the vector
            // receiver is a shared_ptr and guarded by a mutex
            // since it can be accessed by multiple threads concurrently
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    // execute takes a closure f as its arg
    pub fn execute<F>(&self, f: F)
    where
        // FnOnce means a closure that takes void and returns void
        F: FnOnce() + Send + 'static,
    {
        // execute just enqueues f and some worker will poll the queue
        // to actually run the closure
        let job = Box::new(f);

        // unwrap means throw if error
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

// graceful shutdown, let workers finish processing current request
// before shutdown
impl Drop for ThreadPool {
    fn drop(&mut self) {
        // drop sender so no new requests are accepted
        // closes channel
        drop(self.sender.take());
        // removes all workers from the vector and returns iterator on them
        for worker in &mut self.workers.drain(..) {
            println!("Shutting down worker {}", worker.id);

            worker.thread.join().unwrap();
        }
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        // spawn a thread for this worker
        // the returned thread is a JoinHandle
        // TODO(lyan): use std::thread::Builder::spawn
        // which returns Result, instead of panic (throw exception)
        let thread = thread::spawn(move || {
            // we need to run a loop to keep polling the queue
            // and run job
            loop {
                // lock acquires mutex
                // unwrap may panic, could use expect for error handling
                // recv polls job from channel
                // the recv call may block if channel is empty
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(job) => {
                        // id is a member of Worker
                        println!("Worker {id} got a job, executing.");
                        // job is a closure that is callable
                        job();
                    }
                    Err(_) => {
                        println!("Worker {id} disconnected; shutting down.");
                        break;
                    }
                }
            }
        });

        Worker { id, thread }
    }

    // the worker thread should run a loop
    // that poll queue and execute the closure
    // also need a mechanism for terminate
}
