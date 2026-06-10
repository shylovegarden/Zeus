import sys

with open('src/vm/machine.rs', 'r') as f:
    content = f.read()

# Replace expect with safe result unwrapping
content = content.replace("pub fn run(&mut self, bytecode: &[u8], constants: &[f64]) {", "pub fn run(&mut self, bytecode: &[u8], constants: &[f64]) -> Result<(), String> {")

# Instead of doing manual replaces, let's just rewrite the run function body entirely using regex or string replace.
old_run = """    pub fn run(&mut self, bytecode: &[u8], constants: &[f64]) {
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
    }"""

new_run = """    pub fn run(&mut self, bytecode: &[u8], constants: &[f64]) -> Result<(), String> {
        let mut ip = 0; // instruction pointer

        while ip < bytecode.len() {
            let instruction: Opcode = bytecode[ip].into();
            ip += 1;

            match instruction {
                Opcode::OpConstant => {
                    if ip >= bytecode.len() { return Err("IP overflow".into()); }
                    let const_index = bytecode[ip] as usize;
                    ip += 1;
                    if const_index >= constants.len() { return Err("Invalid const index".into()); }
                    self.stack.push(constants[const_index]);
                }
                Opcode::OpAdd => {
                    let b = self.stack.pop().ok_or("Stack underflow on Add")?;
                    let a = self.stack.pop().ok_or("Stack underflow on Add")?;
                    self.stack.push(a + b);
                }
                Opcode::OpSubtract => {
                    let b = self.stack.pop().ok_or("Stack underflow on Subtract")?;
                    let a = self.stack.pop().ok_or("Stack underflow on Subtract")?;
                    self.stack.push(a - b);
                }
                Opcode::OpMultiply => {
                    let b = self.stack.pop().ok_or("Stack underflow on Multiply")?;
                    let a = self.stack.pop().ok_or("Stack underflow on Multiply")?;
                    self.stack.push(a * b);
                }
                Opcode::OpDivide => {
                    let b = self.stack.pop().ok_or("Stack underflow on Divide")?;
                    let a = self.stack.pop().ok_or("Stack underflow on Divide")?;
                    if b == 0.0 { return Err("Division by zero in VM".into()); }
                    self.stack.push(a / b);
                }
                Opcode::OpPop => {
                    self.stack.pop();
                }
                Opcode::OpSetVar => {
                    if ip >= bytecode.len() { return Err("IP overflow".into()); }
                    let var_index = bytecode[ip] as usize;
                    ip += 1;
                    let val = self.stack.pop().ok_or("Stack underflow on SetVar")?;
                    self.memory.insert(var_index, val);
                }
                Opcode::OpGetVar => {
                    if ip >= bytecode.len() { return Err("IP overflow".into()); }
                    let var_index = bytecode[ip] as usize;
                    ip += 1;
                    let val = self.memory.get(&var_index).ok_or("Undefined variable")?;
                    self.stack.push(*val);
                }
                Opcode::OpReturn => {
                    return Ok(());
                }
            }
        }
        Ok(())
    }"""
content = content.replace(old_run, new_run)

with open('src/vm/machine.rs', 'w') as f:
    f.write(content)

print('Success VM')
