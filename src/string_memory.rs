use std::collections::HashMap;

use crate::command_definition::AddrSize;


#[derive(Debug)]
pub struct StringMemory {
    buff: HashMap<usize, String>,
    index: usize,
}

impl StringMemory {
    pub fn new() -> Self {
        Self {
            buff: HashMap::new(),
            index: 0,
        }
    }

    pub fn insert_string(&mut self, s: String) -> usize {
        let key = self.index;
        self.index += 1;
        self.buff.insert(key, s);
        key
    }

    pub fn remove_strings(&mut self, string_mem: &HashMap<AddrSize, usize>) {
        for i in string_mem.values() {
            self.buff.remove(i);
        }
    }

    pub fn get_string(&self, index: usize) -> &str {
        let tmp = self.buff.get(&index);
        tmp.unwrap()
    }
}
