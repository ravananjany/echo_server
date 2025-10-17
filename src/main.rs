mod command;
mod server;

use std::collections::HashMap;

use echo_server::{Counter, ThreadPool};
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::ops::Add;
use std::sync::{Arc, Mutex, RwLock};
use std::{fs, io, vec};

fn main() {
    println!("Hello, world!");
    server_start();
}

fn server_start() {
    server::server_start();
}

/*
fn server_start()  -> Result<() ,String> {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

     println!("Listening on: {}", listener.local_addr().unwrap());

    let mem_store_og = Arc::new(RwLock::new(HashMap::<String, String>::new()));

    let memory_store = Arc::new(Mutex::new(crate::mem_store::Mem::new()));

    let cc = Arc::new(Mutex::new(Counter::new()));

    let pool = ThreadPool::new(2 , Arc::clone(&cc) , Arc::clone(&memory_store));

    for stream in listener.incoming() {
        let stream = stream.unwrap();


        pool.execute(move | shared_cc , mem_store_cc | {
            dbg!("here it comes");
            //server::handle_connection(stream , shared_cc , mem_store_cc)
            lib::handle_streams(stream,  mem_store_cc)
        });
    }
    Ok(())


}
**/
