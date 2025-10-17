use redis_protocol::resp2::types::OwnedFrame as RespFrame;
use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::error::Error;
use std::io::Lines;
use std::sync::{Arc, Mutex, RwLock};

pub enum Command {
    Ping,
    Echo(String),
    Get(String),
    Set(String, String),
    Unknown,
}

impl Command {
    pub fn from_input(input: Result<Vec<String>, String>) -> Self {
        if let Ok(args) = input {
            match args.get(0).map(String::as_str) {
                Some("PING") => {
                    return Command::Ping;
                }

                Some("GET") => {
                    if let Some(key) = args.get(1).map(String::as_str) {
                        return Command::Get(key.to_string());
                    }
                }

                Some("SET") => {
                    if let (Some(k), Some(v)) = (args.get(1), args.get(2)) {
                        return Command::Set(k.to_string(), v.to_string());
                    }
                }

                _ => return Command::Unknown,
            }
        }

        Command::Unknown
    }

    pub fn execute(&self, mem: Arc<RwLock<HashMap<String, String>>>) -> Result<RespFrame, String> {
        match self {
            Command::Ping => Ok(RespFrame::SimpleString("PONG".into())),

            Command::Get(get) => {
                let value = "value-set-by-dummy".to_string();
                Ok(RespFrame::SimpleString("PONG".into()))
            }

            Command::Set(key, value) => Ok(RespFrame::SimpleString("Ok".into())),

            Command::Unknown => Err("Unknown command".to_string()),

            Command::Echo(echo) => Ok(RespFrame::SimpleString("Ok".into())),
        }
    }
}
