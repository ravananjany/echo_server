use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::ops::Add;
use std::sync::{Arc, Mutex, RwLock};
use echo_server::{Counter, ThreadPool};

fn main() {
    println!("Hello, world!");

    server_start();
}

fn server_start() {
    let listener = TcpListener::bind("127.0.0.1:7000").unwrap();

     println!("Listening on: {}", listener.local_addr().unwrap());

    let mem_store = Arc::new(RwLock::new(HashMap::<String, String>::new()));

    let cc = Arc::new(Mutex::new(Counter::new()));

    let pool = ThreadPool::new(2 , Arc::clone(&cc) , Arc::clone(&mem_store));

    for stream in listener.incoming() {
        let stream = stream.unwrap();


        pool.execute(move | shared_cc , mem_store_cc | {
            dbg!("here it comes");
            handle_connection(stream , shared_cc , mem_store_cc)
        });
    }
}

fn parse_command(parts : Vec<String>) -> Cmd {

      let mut  val = "".to_string();

      let cmd = parts.get(0).unwrap().to_string();

      if cmd != "GET" {
          val = parts.get(2).unwrap().to_string();
      }

      Cmd{
          key : parts.get(1).unwrap().to_lowercase(),
          value : val,
      }
}

fn handle_connection(mut stream: TcpStream , cc: Arc<Mutex<Counter>> , mem_store: Arc<RwLock<HashMap<String, String>>> ) {
    let mut buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap().to_uppercase();
    // let http_request: Vec<_> = buf_reader
    //     .lines()
    //     .map(|result| result.unwrap())
    //     .take_while(|line| !line.is_empty())
    //     .collect();
    //
    // println!("Request: {http_request:#?}");

    dbg!(&request_line[..]);

    let parts: Vec<String> = request_line.split_whitespace().map(String::from).collect();

    let  (status_line , response) = match parts.get(0).map(String::as_str) {
        Some("GET") => {
            let cmd_response =   get_command( &parse_command(parts), mem_store);

            (cmd_response.status , cmd_response.body)
        },

        Some("PING") =>{
            ("HTTP/1.1 200 OK\r\n\r\n".to_string(),"PONG".to_string())
        }

        Some("SET") => {

         let  c_r = set_command(parse_command(parts), mem_store);
            (c_r.status , c_r.body)
        }

        Some("SIZE") => {
            ("HTTP/1.1 200 OK\r\n\r\n".to_string(), mem_store.read().unwrap().len().to_string())
        }

        _ => ("Error 500 Internal".to_string(),"".to_string())
    };


    /*
    let (status_line, response) = match &request_line[..] {

        "PING" => ("HTTP/1.1 200 OK PONG".to_string(), "PONG".to_string()),

        "COUNT" => {
            ("HTTP/1.1 200 OK  ".to_string() ,  cc.lock().unwrap().get_count().to_string())
        },

        "GET " => {
                  let cmd_response =   get_command(&Cmd{key:"red".to_string() , value:"".to_string()} , mem_store);
                 (cmd_response.status , cmd_response.body)
        },

        _ => ("HTTP/1.1 404 NOT FOUND".to_string(), "404.html".to_string()),

    };
**/


    let mut contents = String::new().add(response.as_str());
    let length = contents.len();
    let response =
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}\r\n\r\n");


    stream.write(response.as_bytes()).unwrap();
}

struct CmdResponse {
    status: String,
    body: String,
}


struct Cmd {
    key : String,
    value: String,
}

fn get_command(cmd: &Cmd , mem : Arc<RwLock<HashMap<String ,String>>>) -> CmdResponse {
    let mut result = CmdResponse { status: "HTTP/1.1 200 OK ".to_string(), body: "not found".to_string() };
    if mem.read().unwrap().contains_key(&cmd.key.to_lowercase()) {
        result.body = mem.read().unwrap().get(&cmd.key).unwrap().clone();
    }
    result
}


fn set_command(cmd : Cmd , mem : Arc<RwLock<HashMap<String ,String>>>) -> CmdResponse  {

    mem.write().unwrap().insert(cmd.key.to_lowercase() , cmd.value.to_lowercase());

    CmdResponse{
        status: "HTTP/1.1 200 OK".to_string(),
        body: "write success ".to_string(),
    }

}