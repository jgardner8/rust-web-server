use std::{
    sync::{Mutex, mpsc},
    thread,
};

use crate::arc::Arc;
use crate::vec::Vec;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        let job = Box::new(f);

        let sender = self.sender.as_ref().unwrap(); // unwrap is safe - None isn't used before drop
        
        sender.send(job).expect("Fatal: mpsc::Receiver has been deallocated");
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take()); // manually drop sender first so workers stop looking for new jobs

        for worker in self.workers.drain() {
            println!("Shutting down worker {}", worker.id);

            worker.thread.join().unwrap_or_else(|_| panic!("Fatal: worker {} panicked", worker.id));
        }
    }
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().expect("Fatal: mutex holder panicked").recv();

                match message {
                    Ok(job) => {
                        job();
                    }
                    Err(_) => {
                        println!("Worker {id} shutting down");
                        break;
                    }
                }
            }
        });

        Worker { id, thread }
    }
}
