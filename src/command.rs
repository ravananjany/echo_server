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
                        return Command::Get(key.to_string().to_lowercase());
                    }
                }

                Some("SET") => {
                    if let (Some(k), Some(v)) = (args.get(1), args.get(2)) {
                        return Command::Set(k.to_string().to_lowercase(), v.to_string().to_string());
                    }
                }

                _ => return Command::Unknown,
            }
        }

        Command::Unknown
    }


    pub fn execute<T : InMemStore>(&self, mut mem:  T) -> Result<RespFrame, String> {
        match self {
            Command::Ping => Ok(RespFrame::SimpleString("PONG".into())),

            Command::Get(key) => {

                if let Some(value) = mem.get(key.to_string()) {
                     return Ok(RespFrame::SimpleString(value.to_lowercase().into()))
                }else {
                    return Ok(RespFrame::Error(format!("Key not found {}" , key.to_string())))
                }
            }

            Command::Set(key, value) => {
                           mem.set(key.to_string(), value.to_string());
                            return Ok(RespFrame::SimpleString("OK".into()))

            }

            Command::Unknown => Err("Unknown command".to_string()),

            Command::Echo(echo) => Ok(RespFrame::SimpleString("Ok".into())),
        }
    }
}

pub trait InMemStore {
    fn set(&mut self, key: String, value: String) ->Option<String>;
    fn get(&self, key: String) -> Option<String>;
}