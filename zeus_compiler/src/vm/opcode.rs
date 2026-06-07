#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    /// Load a constant value (index into a constants table follows)
    OpConstant = 0x01,
    /// Add the top two stack values
    OpAdd = 0x02,
    /// Subtract the top two stack values
    OpSubtract = 0x03,
    /// Multiply the top two stack values
    OpMultiply = 0x04,
    /// Divide the top two stack values
    OpDivide = 0x05,
    
    /// Pop the top value off the stack
    OpPop = 0x06,
    
    /// Define a global/local variable (operand is the variable's index/name ID)
    OpSetVar = 0x07,
    /// Get a variable's value
    OpGetVar = 0x08,
    
    /// Return from the VM / end of execution
    OpReturn = 0x09,
}

impl From<u8> for Opcode {
    fn from(byte: u8) -> Self {
        match byte {
            0x01 => Opcode::OpConstant,
            0x02 => Opcode::OpAdd,
            0x03 => Opcode::OpSubtract,
            0x04 => Opcode::OpMultiply,
            0x05 => Opcode::OpDivide,
            0x06 => Opcode::OpPop,
            0x07 => Opcode::OpSetVar,
            0x08 => Opcode::OpGetVar,
            0x09 => Opcode::OpReturn,
            _ => panic!("Unknown opcode: {}", byte),
        }
    }
}
