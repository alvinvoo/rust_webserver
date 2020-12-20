use std::thread;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

// We create a buffer struct (Worker) first before using thread::spawn, becoz spwan expects to get
// some code as soon as the thread is created
// the worker structs should fetch code to run from a queue head in ThreadPool and send that code
// to its thread to run
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>
}

impl Worker {
    pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();
            //the lock is held during the call to recv, but it is released before the call to job(), allowing multiple requests to be serviced concurrently

            match message {
                Message::NewJob(job) => {
                    println!("Worker {} got a job; executing.", id);

                    job();
                }
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);

                    break;
                }
            }
        });
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

//struct Job; 
type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message { // a wrapper - such that ThreadPool can terminate properly
    NewJob(Job),
    Terminate,
}

// 1. First, create a channel and ThreadPool hold on as sender of channel
// 2. Worker hold on to receiver of channel
// 3. Job type (think of an container) to hold closures (to send down to channel)
// 4. ThreadPool::execute method to send job (with closures) down to channel
// 5. Worker loop and execute closures of job
pub struct ThreadPool {
    workers: Vec<Worker>, //receiver
    sender: mpsc::Sender<Message> //sender - the pool is the sender itself
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.// usigned size (64 or 32 bit depending on the CPU arch)
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        // mpsc - multiple producer, single consumer
        // straight off wont work with multiple consumers
        // this is multiple threads ownerships (one receiver, shared by multiple workers) -
        // Arc<Mutex<T>>
        // Arc will let multiple workers own the receiver
        // Mutex will ensure only one worker gets a job from the receiver as one time
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // create some threads and store them in the vector
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    // `execute` method, refer to std::thread::spawn
    // pub fn spawn<F, T>(f: F) -> JoinHandle<T> 
   //     where
   //         F: FnOnce() -> T,
   //         F: Send + 'static,
   //         T: Send + 'static, 
    pub fn execute<F>(&self, f: F) 
    where 
        F: FnOnce() + Send + 'static {
            let job = Box::new(f);

            self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        // need two separate loops
        // 1. to send terminate message to all workers thread
        // 2. to join on all worker's thread
        //
        for _ in &self.workers {
            // send terminate message to break idling workers' loop
            // since this is in channel; this can be out of order (unblocking)
            self.sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            // the take method on Option takes the Some variant out and leaves None in its place
            // `join` forces thread to finish off before continuing (blocking)
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }

        // if we put both 1. + 2. together in one loop (essentially unblocking and
        // blocking code together), then imagine there are 2 workers where worker 1 is busying
        // while the shutting down initiates
        // the terminate (unblocked) message will get sent to worker 2 while worker 1 blocks at
        // join(), this means worker 1 would never gets the terminate message and still loop
        // on forever (deadlock)
    }
}
