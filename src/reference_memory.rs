
use std::collections::HashMap;

use crate::command_definition::AddrSize;

pub type ReferenceIndex = usize;

pub trait ReferenceCount {
    fn increment(&mut self, addr: &ReferenceIndex);
    fn decrement(&mut self, addr: &ReferenceIndex);
    fn clean(&mut self);
}


pub struct ReferenceStack {
    stack: Vec<usize>,
}

impl ReferenceStack {

    pub fn new() -> Self {
        Self {
            stack: vec![]
        }
    }

    pub fn push(&mut self, ref_count: &mut dyn ReferenceCount, index: ReferenceIndex) {
        ref_count.increment(&index);
        self.stack.push(index);
    }

    pub fn pop(&mut self, ref_count: &mut dyn ReferenceCount) -> ReferenceIndex {
        let output = self.stack.pop().unwrap();
        ref_count.decrement(&output);
        output
    }
}
