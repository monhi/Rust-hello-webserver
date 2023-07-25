use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub struct ThreadPool 
{
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;
/*
    We can be further confident that FnOnce is the trait we want to use 
    because the thread for running a request will only execute that request’s closure one time,
    which matches the Once in FnOnce.
    The Job type parameter also has the trait bound Send and the lifetime bound 'static, 
    which are useful in our situation: we need Send to transfer the closure from one thread to another and 'static 
    because we don’t know how long the thread will take to execute. 

    We still use the () after FnOnce because this FnOnce represents a closure that takes no parameters and returns the unit type (). 
    Just like function definitions, the return type can be omitted from the signature, but even if we have no parameters, we still need the parentheses.

*/

impl ThreadPool 
{
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool 
    {
        println!("ThreadPool new is called. size is {size}");
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        /*
            mpsc means: Multi-producer, single-consumer FIFO queue communication primitives.
            In this project receiver is assigned to Mutex and different owners can send data to it using sender.
        */

        let receiver = Arc::new(Mutex::new(receiver));
        /*
            To share ownership across multiple threads and allow the threads to mutate the value, we need to use Arc<Mutex<T>>. 
            The Arc type will let multiple workers own the receiver.
            And Mutex will ensure that only one worker gets a job from the receiver at a time.
        */

        let mut workers = Vec::with_capacity(size);
        /*
            Vec::with_capacity creates a vec with at least size elements.
        */

        for id in 0..size 
        {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        /*
            It seems that a single receiver is assigned to all worker threads.
            By using sender, we send messages to all worker threads.
            But because of using Mutex, just only one of them catches it.
        */

        ThreadPool 
        {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
/*
    We can be further confident that FnOnce is the trait we want to use 
    because the thread for running a request will only execute that request’s closure one time,
    which matches the Once in FnOnce.
    The Job type parameter also has the trait bound Send and the lifetime bound 'static, 
    which are useful in our situation: we need Send to transfer the closure from one thread to another and 'static 
    because we don’t know how long the thread will take to execute. 

    We still use the () after FnOnce because this FnOnce represents a closure that takes no parameters and returns the unit type (). 
    Just like function definitions, the return type can be omitted from the signature, but even if we have no parameters, we still need the parentheses.

*/
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }

    /*
        execute method just sends a job by using sender interface.
        As sender interface is connected to multiple receiver interfaces in different threads, all threads catch it.
        But Mutex lets one of threads to handle the job.
        As easy as that.
    */
}

impl Drop for ThreadPool 
{
    fn drop(&mut self) 
    {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() 
            {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker 
{
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker 
{
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker 
    {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");
                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
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