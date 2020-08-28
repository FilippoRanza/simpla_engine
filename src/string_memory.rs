use std::collections::HashMap;

use crate::command_definition::AddrSize;


#[derive(Debug)]
pub struct StringMemory {
    buff: HashMap<usize, StringValue>,
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
        let str_val = StringValue::new(s);
        self.buff.insert(key, str_val);
        key
    }

    pub fn remove_strings(&mut self, string_mem: &HashMap<AddrSize, usize>) {
        for i in string_mem.values() {
            self.remove_reference(*i);
        }
    }

    pub fn get_string(&mut self, index: usize) -> &str {
        let tmp = self.buff.get_mut(&index);
        let str_val = tmp.unwrap();
        //str_val.incr_ref();
        str_val.get_str()
    }

    pub fn increment_reference(&mut self, index: &usize) {
        let tmp = self.buff.get_mut(index);
        let str_val = tmp.unwrap();
        str_val.incr_ref();
    }

    pub fn remove_reference(&mut self, index: usize) {
        let clean = if let Some(str_val) = self.buff.get_mut(&index) {
            str_val.decr_ref()
        } else {
            false
        };

        if clean {
            self.buff.remove(&index);
        }
    }

}

#[derive(Debug)]
struct StringValue {
    string: String,
    ref_count: usize
}

impl StringValue {
    fn new(string: String) -> Self {
        Self {
            string,
            ref_count: 1
        }
    }

    fn incr_ref(&mut self) {
        self.ref_count += 1;
    }


    fn decr_ref(&mut self) -> bool {
        self.ref_count -= 1;
        self.ref_count == 0
    }

    fn get_str(&self) -> &str {
        &self.string
    }
}

