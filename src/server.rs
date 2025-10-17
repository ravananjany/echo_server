use crate::command::Command;
use crate::{Counter, ThreadPool, command};
use futures::future::ok;
use redis_protocol::resp2::encode::encode;
use redis_protocol::resp2::types::{OwnedFrame, Resp2Frame};
use std::collections::HashMap;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::Add;
use std::sync::{Arc, Mutex, RwLock};

fn read_resp_line<R: Read>(reader: &mut BufReader<R>) -> io::Result<String> {
    //creating a buffer to read the data from stream
    let mut buffer = Vec::new();

    //reading byte by byte until we fine an \n which will include \r
    reader.read_until(b'\n', &mut buffer)?;

    dbg!(String::from_utf8_lossy(&buffer));

    // Check if the buffer is empty or doesn't end with \r\n
    if buffer.is_empty()
        || buffer.last() != Some(&b'\n')
        || buffer.get(buffer.len() - 2) != Some(&b'\r')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid RESP line ending or data",
        ));
    }

    // Convert bytes (excluding the final \r\n) to a UTF-8 string
    let line = str::from_utf8(&buffer[..buffer.len() - 2])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e)))?;

    dbg!(&line);

    Ok(line.to_string())
}
pub fn read_resp_request<T: std::io::Read>(stream: T) -> Result<Vec<String>, String> {
    let mut reader = BufReader::new(stream);

    //readint the array header using the above helper
    let array_header =
        read_resp_line(&mut reader).map_err(|e| format!("Error reading array header: {}", e))?;

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
            return Err(format!(
                "Expected bulk string prefix '$', got: {}",
                bulk_header
            ));
        }

        // Parse the length of the string
        let length: usize = bulk_header[1..]
            .parse()
            .map_err(|_| format!("Invalid bulk string length: {}", &bulk_header[1..]))?;

        // 3. Read the Bulk String Value (L bytes + \r\n terminator)
        let mut value_buffer = vec![0; length];
        reader
            .read_exact(&mut value_buffer)
            .map_err(|e| format!("Failed to read bulk string value: {}", e))?;

        // Read and discard the final \r\n terminator
        let mut terminator = [0; 2];
        reader
            .read_exact(&mut terminator)
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

pub fn handle_streams(mut stream: TcpStream, mem_store: Arc<RwLock<HashMap<String, String>>>) {
    let mut buf_reader = BufReader::new(&stream);
    let result = read_resp_request(&stream);

    let cmd = Command::from_input(result);

    let resp = cmd.execute(mem_store);

    let mut res_bytes: Vec<u8> = vec![0; 4096];

    if let Some(resp) = resp.ok() {
        if let Err(e) = write_frames(&resp, &mut res_bytes) {
            eprintln!("error while writing to internal buffer {}", e);
        }

        if let Err(e) = stream.write_all(&res_bytes) {
            eprintln!("error while writing to stream {}", e);
        }
    }
}

pub fn write_frames(frame: &OwnedFrame, out: &mut Vec<u8>) -> Result<(), String> {
    let len = frame.encode_len(false);
    out.reserve(len + len);
    encode(out, frame, false).map_err(|e| format!("Failed to encode frame: {}", e))?;
    Ok(())
}

pub fn handle_connection(
    mut stream: TcpStream,
    cc: Arc<Mutex<Counter>>,
    mem_store: Arc<RwLock<HashMap<String, String>>>,
) {
    let mut buf_reader = BufReader::new(&stream);
    let result = read_resp_request(&stream);

    let mut status_line = "HTTP/1.1 200 OK\r\n\r\n".to_string();
    let mut response = String::new();
    //readiong the args and computing on it
    if let Ok(args) = result {
        (status_line, response) = match args.get(0).map(String::as_str) {
            Some("GET") => {
                let mut c_r = CmdResponse {
                    status: "".to_string(),
                    body: "".to_string(),
                };
                if let Some(key) = args.get(1).map(String::as_str) {
                    c_r = get_command(mem_store, key.to_string());
                }
                dbg!(&c_r);

                (c_r.status, c_r.body)
            }

            Some("SET") => {
                let key = args.get(1).map(String::as_str).unwrap_or("");
                let val = args.get(2).map(String::as_str).unwrap_or("");

                set_command(key.to_string(), val.to_string(), mem_store);

                ("".to_string(), "+OK\r\n".to_string())
            }

            Some("PING") => ("".to_string(), "+PONG\r\n".to_string()),

            _ => ("".to_string(), "$-1\r\n".to_string()),
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
    let res = response;
    //format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}\r\n\r\n");

    stream.write(res.as_bytes()).unwrap();
}

#[derive(Debug)]
struct CmdResponse {
    status: String,
    body: String,
}

fn get_command(mem: Arc<RwLock<HashMap<String, String>>>, key: String) -> CmdResponse {
    let mut result = CmdResponse {
        status: "HTTP/1.1 200 OK ".to_string(),
        body: "$-1\r\n".to_string(),
    };

    if mem.read().unwrap().contains_key(&key.to_lowercase()) {
        let guard = mem.read().unwrap();
        let value = guard.get(&key.to_lowercase());
        result.body = format!("+{}\r\n", value.unwrap().to_lowercase()).to_string();
    }

    result
}

fn set_command(
    key: String,
    value: String,
    mem: Arc<RwLock<HashMap<String, String>>>,
) -> CmdResponse {
    mem.write()
        .unwrap()
        .insert(key.to_lowercase(), value.to_lowercase());

    CmdResponse {
        status: "HTTP/1.1 200 OK".to_string(),
        body: "write success ".to_string(),
    }
}

pub fn server_start() -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    println!("Listening on: {}", listener.local_addr().unwrap());

    let mem_store_og = Arc::new(RwLock::new(HashMap::<String, String>::new()));

    // let memory_store = Arc::new(Mutex::new(crate::mem_store::Mem::new()));

    let cc = Arc::new(Mutex::new(Counter::new()));

    let pool = ThreadPool::new(2, Arc::clone(&cc), Arc::clone(&mem_store_og));

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(move |_shared_cc, mem_store_cc| {
            dbg!("here it comes");
            //server::handle_connection(stream , shared_cc , mem_store_cc)
            handle_streams(stream, mem_store_cc)
        });
    }
    Ok(())
}

// TEST MODULE

#[cfg(test)]
mod tests {
    // Import everything from the outer scope, allowing access to functions like parse_redis_request
    use super::*;
    use futures::future::err;
    use std::io::Cursor;

    #[test]
    fn test_read_resp_request_request() {
        // The raw RESP for: GET mykey
        let raw_request: &[u8] = b"*2\r\n$3\r\nGET\r\n$5\r\nMYKEY\r\n";

        let simulated_stream = Cursor::new(raw_request);

        let result = read_resp_request(*simulated_stream.get_ref());

        assert!(result.is_ok());
    }

    #[test]
    fn test_read_resp_request_set() {
        let raw_request: &[u8] = b"*3\r\n$3\r\nSET\r\n$5\r\nmykey\r\n$6\r\nkeykey\r\n";

        let simulated_stream = Cursor::new(raw_request);

        let result = read_resp_request(*simulated_stream.get_ref());

        let args = result.unwrap();

        assert_eq!(args[2], "KEYKEY");
    }

    #[test]
    fn test_read_resp_request_line() {
        let raw_request: &[u8] = b"*2\r\n$3\r\nGET\r\n$5\r\nMYKEY\r\n";
        let simulated_stream = Cursor::new(raw_request);

        let mut reader = BufReader::new(*simulated_stream.get_ref());

        for i in 0..2 {
            let result = read_resp_line(&mut reader);

            if i == 0 {
                assert_eq!(result.unwrap(), "*2");
            } else if i == 1 {
                assert_eq!(result.unwrap(), "$3");
            } else if i == 2 {
                assert_eq!(result.unwrap(), "GET");
            }
        }
    }
}
