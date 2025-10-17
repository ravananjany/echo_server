use std::collections::HashMap;
use std::fmt::format;
use std::sync::{Arc, RwLock};
use crate::command::InMemStore;

pub struct Mem {
    store: RwLock<HashMap<String, String>>,
}

impl Mem {
    pub fn new() -> Mem {
        Mem{
            store: RwLock::new(HashMap::new()),
        }
    }

}

impl InMemStore for &Mem {
     fn get(&self, key: String) -> Option<String> {
        let store = match self.store.read() {
            Ok(store) => store,
            Err(_) => return None,
        };
        store.get(&key).cloned()
    }

     fn set(&mut self, key: String, value: String) -> Option<String> {
         let mut store = match self.store.write() {
            Ok(store) => store,
            Err(_) => return None,
        };
        store.insert(key, value).clone()
    }
}


