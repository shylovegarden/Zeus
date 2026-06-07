use super::opcode::Opcode;

pub struct Machine {
    pub stack: Vec<f64>,
    pub memory: std::collections::HashMap<usize, f64>,
}

impl Machine {
    pub fn new() -> Self {
        Machine {
            stack: Vec::with_capacity(256),
            memory: std::collections::HashMap::new(),
        }
    }

    pub fn run(&mut self, bytecode: &[u8], constants: &[f64]) {
        let mut ip = 0; // instruction pointer

        while ip < bytecode.len() {
            let instruction: Opcode = bytecode[ip].into();
            ip += 1;

            match instruction {
                Opcode::OpConstant => {
                    let const_index = bytecode[ip] as usize;
                    ip += 1;
                    self.stack.push(constants[const_index]);
                }
                Opcode::OpAdd => {
                    let b = self.stack.pop().expect("Stack underflow on Add");
                    let a = self.stack.pop().expect("Stack underflow on Add");
                    self.stack.push(a + b);
                }
                Opcode::OpSubtract => {
                    let b = self.stack.pop().expect("Stack underflow on Subtract");
                    let a = self.stack.pop().expect("Stack underflow on Subtract");
                    self.stack.push(a - b);
                }
                Opcode::OpMultiply => {
                    let b = self.stack.pop().expect("Stack underflow on Multiply");
                    let a = self.stack.pop().expect("Stack underflow on Multiply");
                    self.stack.push(a * b);
                }
                Opcode::OpDivide => {
                    let b = self.stack.pop().expect("Stack underflow on Divide");
                    let a = self.stack.pop().expect("Stack underflow on Divide");
                    self.stack.push(a / b);
                }
                Opcode::OpPop => {
                    self.stack.pop();
                }
                Opcode::OpSetVar => {
                    let var_index = bytecode[ip] as usize;
                    ip += 1;
                    let val = self.stack.pop().expect("Stack underflow on SetVar");
                    self.memory.insert(var_index, val);
                }
                Opcode::OpGetVar => {
                    let var_index = bytecode[ip] as usize;
                    ip += 1;
                    let val = self.memory.get(&var_index).expect("Undefined variable");
                    self.stack.push(*val);
                }
                Opcode::OpReturn => {
                    return;
                }
            }
        }
    }
}
