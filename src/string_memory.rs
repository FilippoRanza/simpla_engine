use std::collections::HashMap;

use crate::reference_memory::{ReferenceCount, ReferenceStack};

#[derive(Debug)]
pub struct StringMemory {
    buff: HashMap<usize, StringValue>,
    index: usize,
}

#[derive(Debug)]
enum StringType {
    Static,
    Dynamic
}

impl StringMemory {
    pub fn new() -> Self {
        Self {
            buff: HashMap::new(),
            index: 0,
        }
    }

    pub fn insert_static_string(&mut self, s: String) -> usize {
        self.insert_new_string(s, StringType::Static)
    }

    pub fn insert_string(&mut self, s: String) -> usize {
        self.insert_new_string(s, StringType::Dynamic)
    }
    
    

    fn insert_new_string(&mut self, s: String, str_type: StringType) -> usize {
        let key = self.index;
        self.index += 1;
        let str_val = StringValue::new(s, str_type);
        self.buff.insert(key, str_val);
        key
    }

    pub fn remove_strings(&mut self, string_mem: &Vec<usize>) {
        for i in string_mem {
            self.decrement(i);
        }
    }

    pub fn get_string(&self, index: usize) -> &str {
        let tmp = self.buff.get(&index);
        let str_val = tmp.unwrap();
        str_val.get_str()
    }

    pub fn binary_operation<F, T>(&mut self, callback: F, stack: &mut ReferenceStack) -> T
    where
        F: Fn(&str, &str) -> T,
    {
        let rhs_index = stack.pop(self);
        let lhs_index = stack.pop(self);

        let rhs = self.buff.get(&rhs_index).unwrap();
        let lhs = self.buff.get(&lhs_index).unwrap();

        callback(lhs.get_str(), rhs.get_str())
    }
}

impl ReferenceCount for StringMemory {
    fn increment(&mut self, index: &usize) {
        let tmp = self.buff.get_mut(index);
        let str_val = tmp.unwrap();
        str_val.incr_ref();
    }

    fn decrement(&mut self, index: &usize) {
        if let Some(str_val) = self.buff.get_mut(index) {
            str_val.decr_ref();
        }
    }

    fn clean(&mut self) {
        self.buff.retain(|_, v| v.ref_count > 0)
    }
}

#[derive(Debug)]
struct StringValue {
    string: String,
    ref_count: usize,
    str_type: StringType
}

impl StringValue {
    fn new(string: String, str_type: StringType) -> Self {
        Self {
            string,
            ref_count: 1,
            str_type
        }
    }

    fn incr_ref(&mut self) {
        if let StringType::Dynamic = self.str_type {
            self.ref_count += 1;
        }
    }

    fn decr_ref(&mut self) {
        if let StringType::Dynamic = self.str_type {
            if self.ref_count > 0 {
                self.ref_count -= 1;
            }
        }
    }

    fn get_str(&self) -> &str {
        &self.string
    }
}
