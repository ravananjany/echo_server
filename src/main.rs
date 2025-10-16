use std::collections::HashMap;
use std::{fs, io, vec};
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

fn parse_command(parts : &Vec<String>) -> Cmd {

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

//parsing the redis-client request

fn read_resp_line<R : Read>(reader : &mut BufReader<R>) -> io::Result<String> {

   //creating a buffer to read the data from stream
    let mut buffer  = Vec::new();

    //reading byte by byte until we fine an \n which will include \r
    reader.read_until(b'\n', &mut buffer)?;

    dbg!(String::from_utf8_lossy(&buffer));

    // Check if the buffer is empty or doesn't end with \r\n
    if buffer.is_empty() || buffer.last() != Some(&b'\n') || buffer.get(buffer.len() - 2) != Some(&b'\r') {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid RESP line ending or data"));
    }

    // Convert bytes (excluding the final \r\n) to a UTF-8 string
    let line = str::from_utf8(&buffer[..buffer.len() - 2])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e)))?;

    dbg!(&line);

    Ok(line.to_string())

}


fn read_resp_request<T : std::io::Read>(stream : T) -> Result<Vec<String>,String> {

   let mut reader = BufReader::new(stream);

    //readint the array header using the above helper
    let array_header = read_resp_line(&mut reader).map_err(|e| format!("Error reading array header: {}", e))?;

    if !array_header.starts_with('*') {
        return Err(format!("Expected array prefix '*', got: {}", array_header));
    }

    // Parse the number of arguments
    let num_elements: usize = array_header[1..]
        .parse()
        .map_err(|_| format!("Invalid array size: {}", &array_header[1..]))?;


    let mut arguments = Vec::with_capacity(num_elements);


    // 2. Loop through and parse each Bulk String ($L\r\nvalue\r\n)
    for _ in 0..num_elements {
        // Read Bulk String Header ($L)
        let bulk_header = read_resp_line(&mut reader)
            .map_err(|e| format!("Failed to read bulk header: {}", e))?;

        if !bulk_header.starts_with('$') {
            return Err(format!("Expected bulk string prefix '$', got: {}", bulk_header));
        }

        // Parse the length of the string
        let length: usize = bulk_header[1..]
            .parse()
            .map_err(|_| format!("Invalid bulk string length: {}", &bulk_header[1..]))?;

        // 3. Read the Bulk String Value (L bytes + \r\n terminator)
        let mut value_buffer = vec![0; length];
        reader.read_exact(&mut value_buffer)
            .map_err(|e| format!("Failed to read bulk string value: {}", e))?;

        // Read and discard the final \r\n terminator
        let mut terminator = [0; 2];
        reader.read_exact(&mut terminator)
            .map_err(|e| format!("Failed to read terminator: {}", e))?;

        if &terminator != b"\r\n" {
            return Err("Missing bulk string terminator \\r\\n".to_string());
        }

        // Convert the value buffer to a String
        let value = String::from_utf8(value_buffer)
            .map_err(|e| format!("Bulk string value not valid UTF-8: {}", e))?;

        arguments.push(value.to_uppercase());
    }

    Ok(arguments)
}
fn handle_connection(mut stream: TcpStream , cc: Arc<Mutex<Counter>> , mem_store: Arc<RwLock<HashMap<String, String>>> ) {
    let mut buf_reader = BufReader::new(&stream);
    let result  = read_resp_request(&stream);


    let mut status_line = "HTTP/1.1 200 OK\r\n\r\n".to_string();
    let mut response = String::new();
    //readiong the args and computing on it
   if let Ok(args) =  result {

       (status_line , response) = match args.get(0).map(String::as_str) {
           Some("GET") => {
               let mut c_r = CmdResponse{ status: "".to_string(), body: "".to_string() };
               if let Some(key) = args.get(1).map(String::as_str) {
                        c_r =  get_command(mem_store , key.to_string());
               }

               (c_r.status , c_r.body)
           },
           _=> ("".to_string()  , "".to_string())
       }


   }

    /*
    let request_line = buf_reader.lines().next().unwrap().unwrap().to_uppercase();
    // let http_request: Vec<_> = buf_reader
    //     .lines()
    //     .map(|result| result.unwrap())
    //     .take_while(|line| !line.is_empty())
    //     .collect();
    //
    // println!("Request: {http_request:#?}");




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

     */

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

fn get_command(mem : Arc<RwLock<HashMap<String ,String>>> , key : String) -> CmdResponse {
    let mut result = CmdResponse { status: "HTTP/1.1 200 OK ".to_string(), body: "not found".to_string() };
    if mem.read().unwrap().contains_key(&key.to_lowercase()) {
        result.body = mem.read().unwrap().get(&key).unwrap().clone();
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


// TEST MODULE

#[cfg(test)]
mod tests{
    // Import everything from the outer scope, allowing access to functions like parse_redis_request
    use super::*;
    use std::io::Cursor;
    use futures::future::err;

    #[test]
    fn test_read_resp_request_request() {

        // The raw RESP for: GET mykey
        let raw_request: &[u8] = b"*2\r\n$3\r\nGET\r\n$5\r\nMYKEY\r\n";


        let simulated_stream = Cursor::new(raw_request);

        let result = read_resp_request(*simulated_stream.get_ref());

        assert!(result.is_ok());

    }

    #[test]
    fn test_read_resp_request_SET() {
        let raw_request: &[u8] = b"*3\r\n$3\r\nSET\r\n$5\r\nmykey\r\n$6\r\nkeykey\r\n";

        let simulated_stream = Cursor::new(raw_request);

        let result = read_resp_request(*simulated_stream.get_ref());

        let args =  result.unwrap();

        assert_eq!(args[2],"KEYKEY");

    }


    #[test]
    fn test_read_resp_request_line() {
        let raw_request: &[u8] = b"*2\r\n$3\r\nGET\r\n$5\r\nMYKEY\r\n";
        let simulated_stream = Cursor::new(raw_request);

        let mut reader = BufReader::new(*simulated_stream.get_ref());

        for i in 0..2{

            let result = read_resp_line(&mut reader);

            if i == 0{
                assert_eq!(result.unwrap(), "*2");
            }else if i == 1{
                assert_eq!(result.unwrap(), "$3");
            }else if i == 2 {
                assert_eq!(result.unwrap(), "GET");
            }

        }



    }
}