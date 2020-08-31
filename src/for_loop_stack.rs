use crate::command_definition::ForControl;

pub struct ForLoopStack {
    stack: Vec<i32>,
}

impl ForLoopStack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn process_command(&mut self, ctrl: &ForControl, int_stack: &mut Vec<i32>) {
        match ctrl {
            ForControl::Check => self.process_check(int_stack),
            ForControl::End => self.process_end(),
            ForControl::New => self.process_new(int_stack),
        }
    }

    fn process_check(&mut self, int_stack: &mut Vec<i32>) {
        let last = self.stack.last().unwrap();
        int_stack.push(*last);
    }

    fn process_end(&mut self) {
        self.stack.pop();
    }

    fn process_new(&mut self, int_stack: &mut Vec<i32>) {
        let top = int_stack.pop().unwrap();
        self.stack.push(top);
    }
}
