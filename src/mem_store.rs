use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct Mem {
    store : RwLock<HashMap<String, String>>,
}

impl Mem {
     pub fn new() -> Mem {
          Mem {
              store : RwLock::new(HashMap::new()),
          }
     }

     pub fn get(&self, key: &str) -> Option<String> {
     let store =  match self.store.read() {
         Ok(store) => store,
         Err(_) => return None
     };
      store.get(key).cloned()
     }

    pub fn set(&mut self, key: &str, value: &str) {
        let mut store = match self.store.write() {
             Ok(store) => store,
            Err(_) => return
        };
        store.insert(key.to_lowercase(), value.to_string());
    }
}