use std::{sync::mpsc , thread};
use std::collections::HashMap;
use std::ops::{Add, Mul};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::sync::mpsc::Receiver;


pub struct Counter {
    count: u64,
}

impl Counter {
    pub fn new() -> Counter {
         Counter{
             count: 0
         }
    }
    pub fn add_count(&mut self) {

           self.count +=1;

    }

    pub fn get_count(&self) -> u64 {
       self.count
    }

}


pub struct  ThreadPool {
    workers : Vec<Worker>,
    sender : mpsc::Sender<Job>,
    counter : Arc<Mutex<u64>>,
    cc : Arc<Mutex<Counter>>,
    mem  : Arc<RwLock<HashMap<String,String>>>,
}

impl ThreadPool {
     pub fn new(size: usize , cc : Arc<Mutex<Counter>> , mem : Arc<RwLock<HashMap<String,String>>>) -> ThreadPool {
        let (sender ,  rec) =  mpsc::channel();

        let receiver = Arc::new(Mutex::new(rec));

        let mut  workers =  Vec::with_capacity(size);

         for id in 0..size {
             workers.push(Worker::new(id , Arc::clone(&receiver)));
         }

         let counter = Arc::new(Mutex::new(0));

         ThreadPool{ workers ,sender ,   counter , cc , mem} }


    pub fn execute<F>(&self, f: F) where F: FnOnce(Arc<Mutex<Counter>> , Arc<RwLock<HashMap<String,String>>>)  + Send + 'static {


        let counter_cc = Arc::clone(&self.cc);
        counter_cc.lock().unwrap().add_count();

        let mem_cc = Arc::clone(&self.mem);

        let job = Box::new(move || f(counter_cc , mem_cc ));


        /*
        let counter_clone = Arc::clone(&self.counter);
        let job = Box::new(move || {
            {
                let mut guard = counter_clone.lock().expect("Mutex poisoned in counter update");
                *guard += 1;
            }
        });
**/

        self.sender.send(job).unwrap();
    }


    pub  fn get_count(&self) -> u64 {
        *self.counter.lock().unwrap()
    }



}

type  Job = Box<dyn FnOnce() + Send + 'static>;


struct Worker {
    id: usize,
    thread : thread::JoinHandle<()>,
  //  number_of_request : u64,
}


impl Worker {
    fn new(id: usize ,receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {

        let thread = thread::spawn(move || {

            loop {

                dbg!("waiting for message here ....... {}" , id);

                let msg = receiver.lock().unwrap().recv();

                match msg {
                    Ok(job) => {

                        println!("Worker {} got a job; executing.", id);
                        job();
                    }
                    Err(_) => {
                        println!("Worker {} got an error.", id);
                        break;
                    }

                }
            }

        });

        Worker{id , thread  }
    }
}