use crate::mem::MemoryManager;
use crate::vfs::VFS;
use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Register {
    R0 = 0,  // General purpose / accumulator
    R1 = 1,  // General purpose
    R2 = 2,  // General purpose
    R3 = 3,  // General purpose
    R4 = 4,  // General purpose
    R5 = 5,  // General purpose
    R6 = 6,  // Stack pointer
    R7 = 7,  // Frame pointer
    R8 = 8,  // General purpose
    R9 = 9,  // General purpose
    R10 = 10, // General purpose
    R11 = 11, // General purpose
    R12 = 12, // General purpose
    R13 = 13, // General purpose
    R14 = 14, // Link register (return address)
    R15 = 15, // Program counter
    SP = 6,   // Alias for R6
    FP = 7,   // Alias for R7
    LR = 14,  // Alias for R14
    PC = 15,  // Alias for R15
}

impl Register {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0..=15 => Some(unsafe { std::mem::transmute(val) }),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Register::R0 => "r0",
            Register::R1 => "r1",
            Register::R2 => "r2",
            Register::R3 => "r3",
            Register::R4 => "r4",
            Register::R5 => "r5",
            Register::R6 => "sp",
            Register::R7 => "fp",
            Register::R8 => "r8",
            Register::R9 => "r9",
            Register::R10 => "r10",
            Register::R11 => "r11",
            Register::R12 => "r12",
            Register::R13 => "r13",
            Register::R14 => "lr",
            Register::R15 => "pc",
            Register::SP => "sp",
            Register::FP => "fp",
            Register::LR => "lr",
            Register::PC => "pc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Condition {
    EQ = 0b0000,  // Equal (Z=1)
    NE = 0b0001,  // Not equal (Z=0)
    CS = 0b0010,  // Carry set (C=1)
    CC = 0b0011,  // Carry clear (C=0)
    MI = 0b0100,  // Minus (N=1)
    PL = 0b0101,  // Plus (N=0)
    VS = 0b0110,  // Overflow (V=1)
    VC = 0b0111,  // No overflow (V=0)
    HI = 0b1000,  // Unsigned higher (C=1, Z=0)
    LS = 0b1001,  // Unsigned lower or same (C=0 or Z=1)
    GE = 0b1010,  // Signed greater or equal (N=V)
    LT = 0b1011,  // Signed less than (N!=V)
    GT = 0b1100,  // Signed greater than (Z=0, N=V)
    LE = 0b1101,  // Signed less or equal (Z=1 or N!=V)
    AL = 0b1110,  // Always
    NV = 0b1111,  // Never (reserved)
}

impl Condition {
    pub fn check(&self, flags: &Flags) -> bool {
        match self {
            Condition::EQ => flags.z,
            Condition::NE => !flags.z,
            Condition::CS => flags.c,
            Condition::CC => !flags.c,
            Condition::MI => flags.n,
            Condition::PL => !flags.n,
            Condition::VS => flags.v,
            Condition::VC => !flags.v,
            Condition::HI => flags.c && !flags.z,
            Condition::LS => !flags.c || flags.z,
            Condition::GE => flags.n == flags.v,
            Condition::LT => flags.n != flags.v,
            Condition::GT => !flags.z && (flags.n == flags.v),
            Condition::LE => flags.z || (flags.n != flags.v),
            Condition::AL => true,
            Condition::NV => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub n: bool, // Negative
    pub z: bool, // Zero
    pub c: bool, // Carry
    pub v: bool, // Overflow
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Opcode {
    // Arithmetic/Logic (register-register)
    ADD  = 0x00, // Rd = Rn + Rm
    SUB  = 0x01, // Rd = Rn - Rm
    MUL  = 0x02, // Rd = Rn * Rm
    DIV  = 0x03, // Rd = Rn / Rm
    MOD  = 0x04, // Rd = Rn % Rm
    AND  = 0x05, // Rd = Rn & Rm
    ORR  = 0x06, // Rd = Rn | Rm
    EOR  = 0x07, // Rd = Rn ^ Rm
    BIC  = 0x08, // Rd = Rn & ~Rm (bit clear)
    MOV  = 0x09, // Rd = Rm
    MVN  = 0x0A, // Rd = ~Rm
    NEG  = 0x0B, // Rd = -Rm
    LSL  = 0x0C, // Rd = Rn << Rm
    LSR  = 0x0D, // Rd = Rn >> Rm (logical)
    ASR  = 0x0E, // Rd = Rn >> Rm (arithmetic)
    ROR  = 0x0F, // Rd = Rn rotate right Rm
    
    // Arithmetic/Logic (immediate)
    ADDI = 0x10, // Rd = Rn + imm
    SUBI = 0x11, // Rd = Rn - imm
    MULI = 0x12, // Rd = Rn * imm
    ANDI = 0x13, // Rd = Rn & imm
    ORI  = 0x14, // Rd = Rn | imm
    EORI = 0x15, // Rd = Rn ^ imm
    MOVI = 0x16, // Rd = imm
    LSLI = 0x17, // Rd = Rn << imm
    LSRI = 0x18, // Rd = Rn >> imm
    ASRI = 0x19, // Rd = Rn >> imm (arithmetic)
    RORI = 0x1A, // Rd = Rn rotate right imm
    
    // Comparison
    CMP  = 0x20, // Compare Rn - Rm (sets flags)
    CMPI = 0x21, // Compare Rn - imm (sets flags)
    TST  = 0x22, // Test Rn & Rm (sets flags)
    TSTI = 0x23, // Test Rn & imm (sets flags)
    TEQ  = 0x24, // Test Rn ^ Rm (sets flags)
    
    // Memory
    STR  = 0x30, // Store Rt to [Rn + Rm]
    STRI = 0x31, // Store Rt to [Rn + imm]
    LDR  = 0x32, // Load Rt from [Rn + Rm]
    LDRI = 0x33, // Load Rt from [Rn + imm]
    STRB = 0x34, // Store byte
    STRIB = 0x35, // Store byte immediate
    LDRB = 0x36, // Load byte
    LDRIB = 0x37, // Load byte immediate
    PUSH = 0x38, // Push register to stack
    POP  = 0x39, // Pop register from stack
    PUSHA = 0x3A, // Push all registers
    POPA = 0x3B, // Pop all registers
    
    // Control flow
    B    = 0x40, // Branch (unconditional)
    BL   = 0x41, // Branch with link
    BX   = 0x42, // Branch to register
    BLX  = 0x43, // Branch with link to register
    BEQ  = 0x44, // Branch if equal
    BNE  = 0x45, // Branch if not equal
    BCS  = 0x46, // Branch if carry set
    BCC  = 0x47, // Branch if carry clear
    BMI  = 0x48, // Branch if minus
    BPL  = 0x49, // Branch if plus
    BVS  = 0x4A, // Branch if overflow
    BVC  = 0x4B, // Branch if no overflow
    BHI  = 0x4C, // Branch if higher
    BLS  = 0x4D, // Branch if lower or same
    BGE  = 0x4E, // Branch if greater or equal
    BLT  = 0x4F, // Branch if less than
    BGT  = 0x50, // Branch if greater than
    BLE  = 0x51, // Branch if less or equal
    
    // System calls
    SVC  = 0x60, // Supervisor call (syscall)
    SVC1 = 0x61, // Syscall with 1 arg
    SVC2 = 0x62, // Syscall with 2 args
    SVC3 = 0x63, // Syscall with 3 args
    
    // Interrupts/Exceptions
    NOP  = 0x70, // No operation
    HLT  = 0x71, // Halt
    RET  = 0x72, // Return from subroutine
    RETI = 0x73, // Return from interrupt
    YIELD = 0x74, // Yield to scheduler
    BRK  = 0x75, // Breakpoint
    WFI  = 0x76, // Wait for interrupt
    
    // Floating point (optional extension)
    FADD = 0x80,
    FSUB = 0x81,
    FMUL = 0x82,
    FDIV = 0x83,
    FCMP = 0x84,
    FMOV = 0x85,
    FCVT = 0x86,
    
    // Vector/SIMD (optional extension)
    VADD = 0x90,
    VSUB = 0x91,
    VMUL = 0x92,
    VLD  = 0x93,
    VST  = 0x94,
    
    // Crypto (optional extension)
    AES  = 0xA0,
    SHA  = 0xA1,
}

impl Opcode {
    pub fn from_u16(val: u16) -> Option<Self> {
        if val <= 0xA1 {
            Some(unsafe { std::mem::transmute(val) })
        } else {
            None
        }
    }

    pub fn is_conditional_branch(&self) -> bool {
        matches!(self, Opcode::BEQ..=Opcode::BLE)
    }

    pub fn condition(&self) -> Option<Condition> {
        match self {
            Opcode::BEQ => Some(Condition::EQ),
            Opcode::BNE => Some(Condition::NE),
            Opcode::BCS => Some(Condition::CS),
            Opcode::BCC => Some(Condition::CC),
            Opcode::BMI => Some(Condition::MI),
            Opcode::BPL => Some(Condition::PL),
            Opcode::BVS => Some(Condition::VS),
            Opcode::BVC => Some(Condition::VC),
            Opcode::BHI => Some(Condition::HI),
            Opcode::BLS => Some(Condition::LS),
            Opcode::BGE => Some(Condition::GE),
            Opcode::BLT => Some(Condition::LT),
            Opcode::BGT => Some(Condition::GT),
            Opcode::BLE => Some(Condition::LE),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: Opcode,
    pub rd: Option<Register>,
    pub rn: Option<Register>,
    pub rm: Option<Register>,
    pub imm: Option<u32>,
    pub cond: Option<Condition>,
    pub raw: u32,
}

impl Instruction {
    pub fn decode(word: u32) -> Result<Self> {
        let opcode_val = (word >> 24) as u16;
        let opcode = Opcode::from_u16(opcode_val)
            .ok_or_else(|| anyhow!("Invalid opcode: 0x{:04X}", opcode_val))?;

        let rd = Register::from_u8((word >> 20) as u8 & 0xF);
        let rn = Register::from_u8((word >> 16) as u8 & 0xF);
        let rm = Register::from_u8((word >> 12) as u8 & 0xF);
        let cond_val = (word >> 8) as u8 & 0xF;
        let cond = if cond_val != 0xF { Some(unsafe { std::mem::transmute(cond_val) }) } else { None };
        let imm = (word & 0xFF) as u32;

        Ok(Self {
            opcode,
            rd,
            rn,
            rm,
            imm: if imm != 0 { Some(imm) } else { None },
            cond,
            raw: word,
        })
    }

    pub fn encode(&self) -> u32 {
        let mut word = (self.opcode as u32) << 24;
        if let Some(rd) = self.rd {
            word |= (rd as u32) << 20;
        }
        if let Some(rn) = self.rn {
            word |= (rn as u32) << 16;
        }
        if let Some(rm) = self.rm {
            word |= (rm as u32) << 12;
        }
        if let Some(cond) = self.cond {
            word |= (cond as u32) << 8;
        }
        if let Some(imm) = self.imm {
            word |= imm & 0xFF;
        }
        word
    }
}

pub struct CPU {
    pub registers: [u32; 16],
    pub flags: Flags,
    pub memory: Arc<AsyncRwLock<MemoryManager>>,
    pub vfs: Arc<AsyncRwLock<VFS>>,
    pub current_pid: u32,
    pub cycles: u64,
    pub halted: bool,
    pub breakpoints: HashMap<u32, bool>,
    pub trace_mode: bool,
}

impl CPU {
    pub fn new(memory: Arc<AsyncRwLock<MemoryManager>>, vfs: Arc<AsyncRwLock<VFS>>) -> Self {
        let mut registers = [0u32; 16];
        registers[Register::SP as usize] = 0x00FF_F000; // Stack starts at top of user memory
        registers[Register::FP as usize] = 0x00FF_F000;
        
        Self {
            registers,
            flags: Flags::default(),
            memory,
            vfs,
            current_pid: 1,
            cycles: 0,
            halted: false,
            breakpoints: HashMap::new(),
            trace_mode: false,
        }
    }

    pub fn get_reg(&self, reg: Register) -> u32 {
        self.registers[reg as usize]
    }

    pub fn set_reg(&mut self, reg: Register, value: u32) {
        self.registers[reg as usize] = value;
    }

    pub fn get_pc(&self) -> u32 {
        self.get_reg(Register::PC)
    }

    pub fn set_pc(&mut self, value: u32) {
        self.set_reg(Register::PC, value);
    }

    pub fn get_sp(&self) -> u32 {
        self.get_reg(Register::SP)
    }

    pub fn set_sp(&mut self, value: u32) {
        self.set_reg(Register::SP, value);
    }

    pub fn get_lr(&self) -> u32 {
        self.get_reg(Register::LR)
    }

    pub fn set_lr(&mut self, value: u32) {
        self.set_reg(Register::LR, value);
    }

    fn update_flags_arith(&mut self, result: u32, carry: bool, overflow: bool) {
        self.flags.n = (result as i32) < 0;
        self.flags.z = result == 0;
        self.flags.c = carry;
        self.flags.v = overflow;
    }

    fn update_flags_logic(&mut self, result: u32, carry: bool) {
        self.flags.n = (result as i32) < 0;
        self.flags.z = result == 0;
        self.flags.c = carry;
    }

    async fn fetch(&mut self) -> Result<u32> {
        let pc = self.get_pc();
        let word = self.memory.read().await.read32(pc as u64).await?;
        self.set_pc(pc + 4);
        Ok(word)
    }

    async fn execute(&mut self, instr: Instruction) -> Result<bool> {
        if let Some(cond) = instr.cond {
            if !cond.check(&self.flags) {
                return Ok(false);
            }
        }

        if self.trace_mode {
            println!("{:08X}: {:?}", self.get_pc() - 4, instr);
        }

        if self.breakpoints.contains_key(&(self.get_pc() - 4)) {
            println!("Breakpoint hit at {:08X}", self.get_pc() - 4);
            return Ok(true); // Stop execution
        }

        match instr.opcode {
            // Arithmetic/Logic register-register
            Opcode::ADD => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let (result, carry) = rn.overflowing_add(rm);
                let overflow = ((rn as i32) >= 0) == ((rm as i32) >= 0) && ((result as i32) < 0) != ((rn as i32) < 0);
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_arith(result, carry, overflow);
            }
            Opcode::SUB => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let (result, carry) = rn.overflowing_sub(rm);
                let overflow = ((rn as i32) >= 0) != ((rm as i32) >= 0) && ((result as i32) < 0) != ((rn as i32) < 0);
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_arith(result, carry, overflow);
            }
            Opcode::MUL => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let result = rn.wrapping_mul(rm);
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::DIV => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                if rm == 0 {
                    return Err(anyhow!("Division by zero"));
                }
                let result = rn / rm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::MOD => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                if rm == 0 {
                    return Err(anyhow!("Modulo by zero"));
                }
                let result = rn % rm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::AND => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let result = rn & rm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::ORR => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let result = rn | rm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::EOR => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let result = rn ^ rm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::BIC => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let result = rn & !rm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::MOV => {
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                self.set_reg(instr.rd.unwrap_or(Register::R0), rm);
                self.update_flags_logic(rm, false);
            }
            Opcode::MVN => {
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let result = !rm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::NEG => {
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let (result, carry) = 0u32.overflowing_sub(rm);
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_arith(result, carry, false);
            }
            Opcode::LSL => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let shift = (rm & 0x1F) as u32;
                let result = rn << shift;
                let carry = shift > 0 && (rn >> (32 - shift)) & 1 == 1;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, carry);
            }
            Opcode::LSR => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let shift = (rm & 0x1F) as u32;
                let result = rn >> shift;
                let carry = shift > 0 && (rn >> (shift - 1)) & 1 == 1;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, carry);
            }
            Opcode::ASR => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let shift = (rm & 0x1F) as u32;
                let result = (rn as i32 >> shift) as u32;
                let carry = shift > 0 && (rn >> (shift - 1)) & 1 == 1;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, carry);
            }
            Opcode::ROR => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let shift = (rm & 0x1F) as u32;
                let result = rn.rotate_right(shift);
                let carry = shift > 0 && (rn >> (shift - 1)) & 1 == 1;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, carry);
            }

            // Immediate arithmetic/logic
            Opcode::ADDI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let (result, carry) = rn.overflowing_add(imm);
                let overflow = ((rn as i32) >= 0) == ((imm as i32) >= 0) && ((result as i32) < 0) != ((rn as i32) < 0);
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_arith(result, carry, overflow);
            }
            Opcode::SUBI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let (result, carry) = rn.overflowing_sub(imm);
                let overflow = ((rn as i32) >= 0) != ((imm as i32) >= 0) && ((result as i32) < 0) != ((rn as i32) < 0);
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_arith(result, carry, overflow);
            }
            Opcode::MULI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let result = rn.wrapping_mul(imm);
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::ANDI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let result = rn & imm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::ORI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let result = rn | imm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::EORI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let result = rn ^ imm;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, false);
            }
            Opcode::MOVI => {
                let imm = instr.imm.unwrap_or(0);
                self.set_reg(instr.rd.unwrap_or(Register::R0), imm);
                self.update_flags_logic(imm, false);
            }
            Opcode::LSLI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = (instr.imm.unwrap_or(0) & 0x1F) as u32;
                let result = rn << imm;
                let carry = imm > 0 && (rn >> (32 - imm)) & 1 == 1;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, carry);
            }
            Opcode::LSRI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = (instr.imm.unwrap_or(0) & 0x1F) as u32;
                let result = rn >> imm;
                let carry = imm > 0 && (rn >> (imm - 1)) & 1 == 1;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, carry);
            }
            Opcode::ASRI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = (instr.imm.unwrap_or(0) & 0x1F) as u32;
                let result = (rn as i32 >> imm) as u32;
                let carry = imm > 0 && (rn >> (imm - 1)) & 1 == 1;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, carry);
            }
            Opcode::RORI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = (instr.imm.unwrap_or(0) & 0x1F) as u32;
                let result = rn.rotate_right(imm);
                let carry = imm > 0 && (rn >> (imm - 1)) & 1 == 1;
                self.set_reg(instr.rd.unwrap_or(Register::R0), result);
                self.update_flags_logic(result, carry);
            }

            // Comparison
            Opcode::CMP => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let (result, carry) = rn.overflowing_sub(rm);
                let overflow = ((rn as i32) >= 0) != ((rm as i32) >= 0) && ((result as i32) < 0) != ((rn as i32) < 0);
                self.update_flags_arith(result, carry, overflow);
            }
            Opcode::CMPI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let (result, carry) = rn.overflowing_sub(imm);
                let overflow = ((rn as i32) >= 0) != ((imm as i32) >= 0) && ((result as i32) < 0) != ((rn as i32) < 0);
                self.update_flags_arith(result, carry, overflow);
            }
            Opcode::TST => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let result = rn & rm;
                self.update_flags_logic(result, false);
            }
            Opcode::TSTI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let result = rn & imm;
                self.update_flags_logic(result, false);
            }
            Opcode::TEQ => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let result = rn ^ rm;
                self.update_flags_logic(result, false);
            }

            // Memory
            Opcode::STR => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let rt = self.get_reg(instr.rd.unwrap_or(Register::R0));
                let addr = rn.wrapping_add(rm);
                self.memory.write().await.write32(addr as u64, rt).await?;
            }
            Opcode::STRI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let rt = self.get_reg(instr.rd.unwrap_or(Register::R0));
                let addr = rn.wrapping_add(imm);
                self.memory.write().await.write32(addr as u64, rt).await?;
            }
            Opcode::LDR => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let addr = rn.wrapping_add(rm);
                let value = self.memory.read().await.read32(addr as u64).await?;
                self.set_reg(instr.rd.unwrap_or(Register::R0), value);
            }
            Opcode::LDRI => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let addr = rn.wrapping_add(imm);
                let value = self.memory.read().await.read32(addr as u64).await?;
                self.set_reg(instr.rd.unwrap_or(Register::R0), value);
            }
            Opcode::STRB => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let rt = self.get_reg(instr.rd.unwrap_or(Register::R0)) as u8;
                let addr = rn.wrapping_add(rm);
                self.memory.write().await.write8(addr as u64, rt).await?;
            }
            Opcode::STRIB => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let rt = self.get_reg(instr.rd.unwrap_or(Register::R0)) as u8;
                let addr = rn.wrapping_add(imm);
                self.memory.write().await.write8(addr as u64, rt).await?;
            }
            Opcode::LDRB => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let rm = self.get_reg(instr.rm.unwrap_or(Register::R0));
                let addr = rn.wrapping_add(rm);
                let value = self.memory.read().await.read8(addr as u64).await?;
                self.set_reg(instr.rd.unwrap_or(Register::R0), value as u32);
            }
            Opcode::LDRIB => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let imm = instr.imm.unwrap_or(0);
                let addr = rn.wrapping_add(imm);
                let value = self.memory.read().await.read8(addr as u64).await?;
                self.set_reg(instr.rd.unwrap_or(Register::R0), value as u32);
            }
            Opcode::PUSH => {
                let reg = instr.rd.unwrap_or(Register::R0);
                let value = self.get_reg(reg);
                let mut sp = self.get_sp();
                sp = sp.wrapping_sub(4);
                self.memory.write().await.write32(sp as u64, value).await?;
                self.set_sp(sp);
            }
            Opcode::POP => {
                let reg = instr.rd.unwrap_or(Register::R0);
                let sp = self.get_sp();
                let value = self.memory.read().await.read32(sp as u64).await?;
                self.set_reg(reg, value);
                self.set_sp(sp.wrapping_add(4));
            }
            Opcode::PUSHA => {
                let mut sp = self.get_sp();
                for i in 0..14 { // Don't push PC
                    let value = self.get_reg(Register::from_u8(i).unwrap());
                    sp = sp.wrapping_sub(4);
                    self.memory.write().await.write32(sp as u64, value).await?;
                }
                self.set_sp(sp);
            }
            Opcode::POPA => {
                let mut sp = self.get_sp();
                for i in (0..14).rev() {
                    let value = self.memory.read().await.read32(sp as u64).await?;
                    self.set_reg(Register::from_u8(i).unwrap(), value);
                    sp = sp.wrapping_add(4);
                }
                self.set_sp(sp);
            }

            // Control flow
            Opcode::B => {
                let offset = instr.imm.unwrap_or(0) as i32;
                let pc = self.get_pc();
                self.set_pc(pc.wrapping_add((offset * 4) as u32));
            }
            Opcode::BL => {
                let offset = instr.imm.unwrap_or(0) as i32;
                let pc = self.get_pc();
                self.set_lr(pc);
                self.set_pc(pc.wrapping_add((offset * 4) as u32));
            }
            Opcode::BX => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                self.set_pc(rn);
            }
            Opcode::BLX => {
                let rn = self.get_reg(instr.rn.unwrap_or(Register::R0));
                let pc = self.get_pc();
                self.set_lr(pc);
                self.set_pc(rn);
            }
            Opcode::BEQ | Opcode::BNE | Opcode::BCS | Opcode::BCC |
            Opcode::BMI | Opcode::BPL | Opcode::BVS | Opcode::BVC |
            Opcode::BHI | Opcode::BLS | Opcode::BGE | Opcode::BLT |
            Opcode::BGT | Opcode::BLE => {
                if let Some(cond) = instr.opcode.condition() {
                    if cond.check(&self.flags) {
                        let offset = instr.imm.unwrap_or(0) as i32;
                        let pc = self.get_pc();
                        self.set_pc(pc.wrapping_add((offset * 4) as u32));
                    }
                }
            }

            // System calls
            Opcode::SVC | Opcode::SVC1 | Opcode::SVC2 | Opcode::SVC3 => {
                self.handle_syscall(instr.opcode).await?;
            }

            // Interrupts/Exceptions
            Opcode::NOP => {}
            Opcode::HLT => {
                self.halted = true;
                return Ok(true);
            }
            Opcode::RET => {
                let lr = self.get_lr();
                self.set_pc(lr);
            }
            Opcode::RETI => {
                // Return from interrupt - pop status and PC
                let sp = self.get_sp();
                let new_flags = self.memory.read().await.read32(sp as u64).await?;
                self.set_sp(sp + 4);
                let new_pc = self.memory.read().await.read32(sp as u64).await?;
                self.set_sp(sp + 8);
                // Restore flags from new_flags
                self.set_pc(new_pc);
            }
            Opcode::YIELD => {
                return Ok(true); // Yield to scheduler
            }
            Opcode::BRK => {
                println!("Breakpoint instruction at {:08X}", self.get_pc() - 4);
                return Ok(true);
            }
            Opcode::WFI => {
                // Wait for interrupt - just yield
                return Ok(true);
            }

            _ => {
                return Err(anyhow!("Unimplemented opcode: {:?}", instr.opcode));
            }
        }

        self.cycles += 1;
        Ok(false)
    }

    async fn handle_syscall(&mut self, opcode: Opcode) -> Result<()> {
        let syscall_num = self.get_reg(Register::R0);
        let arg1 = self.get_reg(Register::R1);
        let arg2 = self.get_reg(Register::R2);
        let arg3 = self.get_reg(Register::R3);

        // Simple syscall handling - in reality this would call into the kernel/VFS
        let result = match syscall_num {
            0 => 0, // exit
            1 => {
                // write(fd, buf, len)
                let fd = arg1;
                let buf = arg2;
                let len = arg3;
                let mut output = Vec::with_capacity(len as usize);
                for i in 0..len {
                    let byte = self.memory.read().await.read8(buf as u64 + i as u64).await?;
                    output.push(byte);
                }
                print!("{}", String::from_utf8_lossy(&output));
                len
            }
            2 => 0, // read
            3 => 0, // open
            4 => 0, // close
            5 => 0, // stat
            6 => 0, // fstat
            7 => 0, // lseek
            8 => 0, // mmap
            9 => 0, // munmap
            10 => 0, // brk
            11 => 0, // rt_sigaction
            12 => 0, // rt_sigprocmask
            13 => 0, // ioctl
            14 => 0, // pread
            15 => 0, // pwrite
            16 => 0, // readv
            17 => 0, // writev
            18 => 0, // access
            19 => 0, // pipe
            20 => 0, // select
            21 => 0, // sched_yield
            22 => 0, // mremap
            23 => 0, // msync
            24 => 0, // mincore
            25 => 0, // madvise
            26 => 0, // shmget
            27 => 0, // shmat
            28 => 0, // shmctl
            29 => 0, // dup
            30 => 0, // dup2
            31 => 0, // pause
            32 => 0, // nanosleep
            33 => 0, // getitimer
            34 => 0, // alarm
            35 => 0, // setitimer
            36 => 0, // getpid
            37 => 0, // sendfile
            38 => 0, // socket
            39 => 0, // connect
            40 => 0, // accept
            41 => 0, // sendto
            42 => 0, // recvfrom
            43 => 0, // sendmsg
            44 => 0, // recvmsg
            45 => 0, // shutdown
            46 => 0, // bind
            47 => 0, // listen
            48 => 0, // getsockname
            49 => 0, // getpeername
            50 => 0, // socketpair
            51 => 0, // setsockopt
            52 => 0, // getsockopt
            53 => 0, // clone
            54 => 0, // fork
            55 => 0, // vfork
            56 => 0, // execve
            57 => 0, // exit_group
            58 => 0, // wait4
            59 => 0, // kill
            60 => 0, // uname
            61 => 0, // semget
            62 => 0, // semop
            63 => 0, // semctl
            64 => 0, // shmdt
            65 => 0, // msgget
            66 => 0, // msgsnd
            67 => 0, // msgrcv
            68 => 0, // msgctl
            69 => 0, // fcntl
            70 => 0, // flock
            71 => 0, // fsync
            72 => 0, // fdatasync
            73 => 0, // truncate
            74 => 0, // ftruncate
            75 => 0, // getdents
            76 => 0, // getcwd
            77 => 0, // chdir
            78 => 0, // fchdir
            79 => 0, // rename
            80 => 0, // mkdir
            81 => 0, // rmdir
            82 => 0, // creat
            83 => 0, // link
            84 => 0, // unlink
            85 => 0, // symlink
            86 => 0, // readlink
            87 => 0, // chmod
            88 => 0, // fchmod
            89 => 0, // chown
            90 => 0, // fchown
            91 => 0, // lchown
            92 => 0, // umask
            93 => 0, // gettimeofday
            94 => 0, // getrlimit
            95 => 0, // getrusage
            96 => 0, // sysinfo
            97 => 0, // times
            98 => 0, // ptrace
            99 => 0, // getuid
            100 => 0, // syslog
            101 => 0, // getgid
            102 => 0, // setuid
            103 => 0, // setgid
            104 => 0, // geteuid
            105 => 0, // getegid
            106 => 0, // setpgid
            107 => 0, // getppid
            108 => 0, // getpgrp
            109 => 0, // setsid
            110 => 0, // setreuid
            111 => 0, // setregid
            112 => 0, // getgroups
            113 => 0, // setgroups
            114 => 0, // setresuid
            115 => 0, // getresuid
            116 => 0, // setresgid
            117 => 0, // getresgid
            118 => 0, // getpgid
            119 => 0, // setfsuid
            120 => 0, // setfsgid
            121 => 0, // getsid
            122 => 0, // capget
            123 => 0, // capset
            124 => 0, // rt_sigpending
            125 => 0, // rt_sigtimedwait
            126 => 0, // rt_sigqueueinfo
            127 => 0, // rt_sigsuspend
            128 => 0, // sigaltstack
            129 => 0, // utime
            130 => 0, // mknod
            131 => 0, // uselib
            132 => 0, // personality
            133 => 0, // ustat
            134 => 0, // statfs
            135 => 0, // fstatfs
            136 => 0, // sysfs
            137 => 0, // getpriority
            138 => 0, // setpriority
            139 => 0, // sched_setparam
            140 => 0, // sched_getparam
            141 => 0, // sched_setscheduler
            142 => 0, // sched_getscheduler
            143 => 0, // sched_get_priority_max
            144 => 0, // sched_get_priority_min
            145 => 0, // sched_rr_get_interval
            146 => 0, // mlock
            147 => 0, // munlock
            148 => 0, // mlockall
            149 => 0, // munlockall
            150 => 0, // vhangup
            151 => 0, // modify_ldt
            152 => 0, // pivot_root
            153 => 0, // _sysctl
            154 => 0, // prctl
            155 => 0, // arch_prctl
            156 => 0, // adjtimex
            157 => 0, // setrlimit
            158 => 0, // chroot
            159 => 0, // sync
            160 => 0, // acct
            161 => 0, // settimeofday
            162 => 0, // mount
            163 => 0, // umount2
            164 => 0, // swapon
            165 => 0, // swapoff
            166 => 0, // reboot
            167 => 0, // sethostname
            168 => 0, // setdomainname
            169 => 0, // iopl
            170 => 0, // ioperm
            171 => 0, // create_module
            172 => 0, // init_module
            173 => 0, // delete_module
            174 => 0, // get_kernel_syms
            175 => 0, // query_module
            176 => 0, // quotactl
            177 => 0, // nfsservctl
            178 => 0, // getpmsg
            179 => 0, // putpmsg
            180 => 0, // afs_syscall
            181 => 0, // tuxcall
            182 => 0, // security
            183 => 0, // gettid
            184 => 0, // readahead
            185 => 0, // setxattr
            186 => 0, // lsetxattr
            187 => 0, // fsetxattr
            188 => 0, // getxattr
            189 => 0, // lgetxattr
            190 => 0, // fgetxattr
            191 => 0, // listxattr
            192 => 0, // llistxattr
            193 => 0, // flistxattr
            194 => 0, // removexattr
            195 => 0, // lremovexattr
            196 => 0, // fremovexattr
            197 => 0, // tkill
            198 => 0, // time
            199 => 0, // futex
            200 => 0, // sched_setaffinity
            201 => 0, // sched_getaffinity
            202 => 0, // set_thread_area
            203 => 0, // io_setup
            204 => 0, // io_destroy
            205 => 0, // io_getevents
            206 => 0, // io_submit
            207 => 0, // io_cancel
            208 => 0, // get_thread_area
            209 => 0, // lookup_dcookie
            210 => 0, // epoll_create
            211 => 0, // epoll_ctl_old
            212 => 0, // epoll_wait_old
            213 => 0, // remap_file_pages
            214 => 0, // getdents64
            215 => 0, // set_tid_address
            216 => 0, // restart_syscall
            217 => 0, // semtimedop
            218 => 0, // fadvise64
            219 => 0, // timer_create
            220 => 0, // timer_settime
            221 => 0, // timer_gettime
            222 => 0, // timer_getoverrun
            223 => 0, // timer_delete
            224 => 0, // clock_settime
            225 => 0, // clock_gettime
            226 => 0, // clock_getres
            227 => 0, // clock_nanosleep
            228 => 0, // exit_group
            229 => 0, // epoll_wait
            230 => 0, // epoll_ctl
            231 => 0, // tgkill
            232 => 0, // utimes
            233 => 0, // vserver
            234 => 0, // mbind
            235 => 0, // set_mempolicy
            236 => 0, // get_mempolicy
            237 => 0, // mq_open
            238 => 0, // mq_unlink
            239 => 0, // mq_timedsend
            240 => 0, // mq_timedreceive
            241 => 0, // mq_notify
            242 => 0, // mq_getsetattr
            243 => 0, // kexec_load
            244 => 0, // waitid
            245 => 0, // add_key
            246 => 0, // request_key
            247 => 0, // keyctl
            248 => 0, // ioprio_set
            249 => 0, // ioprio_get
            250 => 0, // inotify_init
            251 => 0, // inotify_add_watch
            252 => 0, // inotify_rm_watch
            253 => 0, // migrate_pages
            254 => 0, // openat
            255 => 0, // mkdirat
            _ => {
                eprintln!("Unknown syscall: {}", syscall_num);
                0
            }
        };

        self.set_reg(Register::R0, result);
        Ok(())
    }

    pub async fn step(&mut self) -> Result<bool> {
        if self.halted {
            return Ok(true);
        }

        let word = self.fetch().await?;
        let instr = Instruction::decode(word)?;
        self.execute(instr).await
    }

    pub async fn run(&mut self, running: Arc<AsyncRwLock<bool>>) {
        while *running.read().await && !self.halted {
            if let Err(e) = self.step().await {
                eprintln!("CPU error: {}", e);
                break;
            }
            
            // Yield periodically
            if self.cycles % 1000 == 0 {
                tokio::task::yield_now().await;
            }
        }
    }

    pub fn dump_registers(&self) {
        println!("Registers:");
        for i in 0..16 {
            let reg = Register::from_u8(i).unwrap();
            println!("  {}: {:08X} ({})", reg.name(), self.registers[i], self.registers[i]);
        }
        println!("Flags: N={} Z={} C={} V={}", 
            self.flags.n as u8, self.flags.z as u8, self.flags.c as u8, self.flags.v as u8);
        println!("PC: {:08X}  SP: {:08X}  LR: {:08X}  Cycles: {}", 
            self.get_pc(), self.get_sp(), self.get_lr(), self.cycles);
    }

    pub fn add_breakpoint(&mut self, addr: u32) {
        self.breakpoints.insert(addr, true);
    }

    pub fn remove_breakpoint(&mut self, addr: u32) {
        self.breakpoints.remove(&addr);
    }

    pub fn set_trace(&mut self, enabled: bool) {
        self.trace_mode = enabled;
    }
}