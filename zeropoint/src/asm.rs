//! ZEROPOINT Assembler
//! 
//! This assembler converts ZEROPOINT assembly language into machine code
//! following the official ISA specification.

use anyhow::{Result, anyhow, Context};
use std::collections::HashMap;
use crate::cpu::{Opcode, Register, Condition, Instruction};

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub value: u32,
    pub is_label: bool,
}

#[derive(Debug, Clone)]
pub struct AssembledProgram {
    pub code: Vec<u32>,
    pub symbols: HashMap<String, u32>,
    pub entry_point: Option<u32>,
}

pub struct Assembler {
    symbols: HashMap<String, Symbol>,
    current_address: u32,
    errors: Vec<AssemblerError>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AssemblerError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            current_address: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn assemble(&mut self, source: &str) -> Result<AssembledProgram> {
        self.symbols.clear();
        self.current_address = 0;
        self.errors.clear();
        self.warnings.clear();

        // First pass: collect labels and calculate addresses
        let lines: Vec<&str> = source.lines().collect();
        self.first_pass(&lines)?;

        if !self.errors.is_empty() {
            return Err(anyhow!("Assembly failed with {} errors", self.errors.len()));
        }

        // Second pass: generate machine code
        let mut code = Vec::new();
        self.current_address = 0;
        
        for (line_num, line) in lines.iter().enumerate() {
            if let Some(instr) = self.parse_instruction(line, line_num + 1)? {
                code.push(instr.encode());
                self.current_address += 4;
            }
        }

        if !self.errors.is_empty() {
            return Err(anyhow!("Assembly failed with {} errors", self.errors.len()));
        }

        Ok(AssembledProgram {
            code,
            symbols: self.symbols.iter().map(|(k, v)| (k.clone(), v.value)).collect(),
            entry_point: self.symbols.get("_start").map(|s| s.value),
        })
    }

    fn first_pass(&mut self, lines: &[&str]) -> Result<()> {
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with("//") {
                continue;
            }

            // Check for label definition
            if trimmed.ends_with(':') {
                let label_name = trimmed[..trimmed.len()-1].trim();
                if label_name.is_empty() {
                    self.errors.push(AssemblerError {
                        line: line_num + 1,
                        column: 0,
                        message: "Empty label name".to_string(),
                    });
                    continue;
                }
                
                if self.symbols.contains_key(label_name) {
                    self.errors.push(AssemblerError {
                        line: line_num + 1,
                        column: 0,
                        message: format!("Duplicate label: {}", label_name),
                    });
                    continue;
                }
                
                self.symbols.insert(label_name.to_string(), Symbol {
                    name: label_name.to_string(),
                    value: self.current_address,
                    is_label: true,
                });
                continue;
            }

            // Parse instruction to calculate size
            if let Some(_) = self.parse_instruction_for_size(trimmed)? {
                self.current_address += 4;
            }
        }
        Ok(())
    }

    fn parse_instruction_for_size(&self, line: &str) -> Result<Option<u32>> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }
        
        // Check if it's a known instruction
        let mnemonic = parts[0].to_uppercase();
        if self.is_valid_mnemonic(&mnemonic) {
            return Ok(Some(4));
        }
        
        // Check for directives
        if mnemonic.starts_with('.') {
            return Ok(Some(0)); // Directives don't generate code in this simple version
        }
        
        Ok(None)
    }

    fn is_valid_mnemonic(&self, mnemonic: &str) -> bool {
        matches!(mnemonic,
            "ADD" | "SUB" | "MUL" | "DIV" | "MOD" |
            "AND" | "ORR" | "EOR" | "BIC" | "MOV" | "MVN" | "NEG" |
            "LSL" | "LSR" | "ASR" | "ROR" |
            "ADDI" | "SUBI" | "MULI" | "ANDI" | "ORI" | "EORI" | "MOVI" |
            "LSLI" | "LSRI" | "ASRI" | "RORI" |
            "CMP" | "CMPI" | "TST" | "TSTI" | "TEQ" |
            "STR" | "STRI" | "LDR" | "LDRI" | "STRB" | "STRIB" | "LDRB" | "LDRIB" |
            "PUSH" | "POP" | "PUSHA" | "POPA" |
            "B" | "BL" | "BX" | "BLX" |
            "BEQ" | "BNE" | "BCS" | "BCC" | "BMI" | "BPL" | "BVS" | "BVC" |
            "BHI" | "BLS" | "BGE" | "BLT" | "BGT" | "BLE" |
            "SVC" | "SVC1" | "SVC2" | "SVC3" |
            "NOP" | "HLT" | "RET" | "RETI" | "YIELD" | "BRK" | "WFI"
        )
    }

    fn parse_instruction(&mut self, line: &str, line_num: usize) -> Result<Option<Instruction>> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with("//") || trimmed.ends_with(':') {
            return Ok(None);
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }

        let mnemonic = parts[0].to_uppercase();
        let args = &parts[1..];

        // Parse opcode
        let opcode = self.parse_opcode(&mnemonic)
            .ok_or_else(|| anyhow!("Unknown instruction: {}", mnemonic))?;

        // Parse condition code suffix (e.g., ADDEQ)
        let (base_opcode, cond) = self.parse_condition_suffix(&mnemonic, opcode);

        let mut instr = Instruction {
            opcode: base_opcode,
            rd: None,
            rn: None,
            rm: None,
            imm: None,
            cond,
            raw: 0,
        };

        // Parse operands based on instruction type
        self.parse_operands(&mut instr, args, line_num)?;

        Ok(Some(instr))
    }

    fn parse_opcode(&self, mnemonic: &str) -> Option<Opcode> {
        // Handle conditional suffixes first
        let base = mnemonic.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        let suffix_len = mnemonic.len() - base.len();
        
        let opcode_str = if suffix_len == 2 {
            // Could be conditional suffix
            &mnemonic[..mnemonic.len()-2]
        } else {
            mnemonic
        };

        match opcode_str {
            "ADD" => Some(Opcode::ADD),
            "SUB" => Some(Opcode::SUB),
            "MUL" => Some(Opcode::MUL),
            "DIV" => Some(Opcode::DIV),
            "MOD" => Some(Opcode::MOD),
            "AND" => Some(Opcode::AND),
            "ORR" => Some(Opcode::ORR),
            "EOR" => Some(Opcode::EOR),
            "BIC" => Some(Opcode::BIC),
            "MOV" => Some(Opcode::MOV),
            "MVN" => Some(Opcode::MVN),
            "NEG" => Some(Opcode::NEG),
            "LSL" => Some(Opcode::LSL),
            "LSR" => Some(Opcode::LSR),
            "ASR" => Some(Opcode::ASR),
            "ROR" => Some(Opcode::ROR),
            "ADDI" => Some(Opcode::ADDI),
            "SUBI" => Some(Opcode::SUBI),
            "MULI" => Some(Opcode::MULI),
            "ANDI" => Some(Opcode::ANDI),
            "ORI" => Some(Opcode::ORI),
            "EORI" => Some(Opcode::EORI),
            "MOVI" => Some(Opcode::MOVI),
            "LSLI" => Some(Opcode::LSLI),
            "LSRI" => Some(Opcode::LSRI),
            "ASRI" => Some(Opcode::ASRI),
            "RORI" => Some(Opcode::RORI),
            "CMP" => Some(Opcode::CMP),
            "CMPI" => Some(Opcode::CMPI),
            "TST" => Some(Opcode::TST),
            "TSTI" => Some(Opcode::TSTI),
            "TEQ" => Some(Opcode::TEQ),
            "STR" => Some(Opcode::STR),
            "STRI" => Some(Opcode::STRI),
            "LDR" => Some(Opcode::LDR),
            "LDRI" => Some(Opcode::LDRI),
            "STRB" => Some(Opcode::STRB),
            "STRIB" => Some(Opcode::STRIB),
            "LDRB" => Some(Opcode::LDRB),
            "LDRIB" => Some(Opcode::LDRIB),
            "PUSH" => Some(Opcode::PUSH),
            "POP" => Some(Opcode::POP),
            "PUSHA" => Some(Opcode::PUSHA),
            "POPA" => Some(Opcode::POPA),
            "B" => Some(Opcode::B),
            "BL" => Some(Opcode::BL),
            "BX" => Some(Opcode::BX),
            "BLX" => Some(Opcode::BLX),
            "BEQ" => Some(Opcode::BEQ),
            "BNE" => Some(Opcode::BNE),
            "BCS" => Some(Opcode::BCS),
            "BCC" => Some(Opcode::BCC),
            "BMI" => Some(Opcode::BMI),
            "BPL" => Some(Opcode::BPL),
            "BVS" => Some(Opcode::BVS),
            "BVC" => Some(Opcode::BVC),
            "BHI" => Some(Opcode::BHI),
            "BLS" => Some(Opcode::BLS),
            "BGE" => Some(Opcode::BGE),
            "BLT" => Some(Opcode::BLT),
            "BGT" => Some(Opcode::BGT),
            "BLE" => Some(Opcode::BLE),
            "SVC" => Some(Opcode::SVC),
            "SVC1" => Some(Opcode::SVC1),
            "SVC2" => Some(Opcode::SVC2),
            "SVC3" => Some(Opcode::SVC3),
            "NOP" => Some(Opcode::NOP),
            "HLT" => Some(Opcode::HLT),
            "RET" => Some(Opcode::RET),
            "RETI" => Some(Opcode::RETI),
            "YIELD" => Some(Opcode::YIELD),
            "BRK" => Some(Opcode::BRK),
            "WFI" => Some(Opcode::WFI),
            _ => None,
        }
    }

    fn parse_condition_suffix(&self, mnemonic: &str, opcode: Opcode) -> (Opcode, Option<Condition>) {
        // Check for conditional branch opcodes first
        if opcode.is_conditional_branch() {
            return (opcode, opcode.condition());
        }

        // Check for conditional suffix on other instructions (e.g., ADDEQ)
        let suffixes: [(&str, Condition); 14] = [
            ("EQ", Condition::EQ), ("NE", Condition::NE),
            ("CS", Condition::CS), ("CC", Condition::CC),
            ("MI", Condition::MI), ("PL", Condition::PL),
            ("VS", Condition::VS), ("VC", Condition::VC),
            ("HI", Condition::HI), ("LS", Condition::LS),
            ("GE", Condition::GE), ("LT", Condition::LT),
            ("GT", Condition::GT), ("LE", Condition::LE),
        ];

        for (suffix, cond) in suffixes {
            if mnemonic.ends_with(suffix) && mnemonic.len() > suffix.len() {
                let base = &mnemonic[..mnemonic.len()-suffix.len()];
                if self.parse_opcode(base).is_some() {
                    return (self.parse_opcode(base).unwrap(), Some(cond));
                }
            }
        }

        (opcode, None)
    }

    fn parse_operands(&mut self, instr: &mut Instruction, args: &[&str], line_num: usize) -> Result<()> {
        let opcode = instr.opcode;
        
        // Parse comma-separated operands
        let operand_str = args.join(" ");
        let operands: Vec<&str> = operand_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

        match opcode {
            // 3-register instructions: ADD Rd, Rn, Rm
            Opcode::ADD | Opcode::SUB | Opcode::MUL | Opcode::DIV | Opcode::MOD |
            Opcode::AND | Opcode::ORR | Opcode::EOR | Opcode::BIC |
            Opcode::LSL | Opcode::LSR | Opcode::ASR | Opcode::ROR => {
                if operands.len() != 3 {
                    return Err(anyhow!("Expected 3 operands for {}", opcode));
                }
                instr.rd = Some(self.parse_register(operands[0], line_num)?);
                instr.rn = Some(self.parse_register(operands[1], line_num)?);
                instr.rm = Some(self.parse_register(operands[2], line_num)?);
            }

            // 2-register instructions: MOV Rd, Rm
            Opcode::MOV | Opcode::MVN | Opcode::NEG => {
                if operands.len() != 2 {
                    return Err(anyhow!("Expected 2 operands for {}", opcode));
                }
                instr.rd = Some(self.parse_register(operands[0], line_num)?);
                instr.rm = Some(self.parse_register(operands[1], line_num)?);
            }

            // Immediate arithmetic/logic: ADDI Rd, Rn, #imm
            Opcode::ADDI | Opcode::SUBI | Opcode::MULI | Opcode::ANDI | Opcode::ORI | Opcode::EORI |
            Opcode::LSLI | Opcode::LSRI | Opcode::ASRI | Opcode::RORI => {
                if operands.len() != 3 {
                    return Err(anyhow!("Expected 3 operands for {}", opcode));
                }
                instr.rd = Some(self.parse_register(operands[0], line_num)?);
                instr.rn = Some(self.parse_register(operands[1], line_num)?);
                instr.imm = Some(self.parse_immediate(operands[2], line_num)?);
            }

            // MOVI Rd, #imm
            Opcode::MOVI => {
                if operands.len() != 2 {
                    return Err(anyhow!("Expected 2 operands for MOVI"));
                }
                instr.rd = Some(self.parse_register(operands[0], line_num)?);
                instr.imm = Some(self.parse_immediate(operands[1], line_num)?);
            }

            // Compare instructions: CMP Rn, Rm
            Opcode::CMP | Opcode::TST | Opcode::TEQ => {
                if operands.len() != 2 {
                    return Err(anyhow!("Expected 2 operands for {}", opcode));
                }
                instr.rn = Some(self.parse_register(operands[0], line_num)?);
                instr.rm = Some(self.parse_register(operands[1], line_num)?);
            }

            // Compare immediate: CMPI Rn, #imm
            Opcode::CMPI | Opcode::TSTI => {
                if operands.len() != 2 {
                    return Err(anyhow!("Expected 2 operands for {}", opcode));
                }
                instr.rn = Some(self.parse_register(operands[0], line_num)?);
                instr.imm = Some(self.parse_immediate(operands[1], line_num)?);
            }

            // Memory register-register: STR Rt, [Rn, Rm]
            Opcode::STR | Opcode::LDR | Opcode::STRB | Opcode::LDRB => {
                if operands.len() != 2 && operands.len() != 3 {
                    return Err(anyhow!("Expected 2 or 3 operands for {}", opcode));
                }
                instr.rd = Some(self.parse_register(operands[0], line_num)?); // Rt
                
                // Parse [Rn, Rm] or [Rn]
                let mem_operand = operands[1];
                if mem_operand.starts_with('[') && mem_operand.ends_with(']') {
                    let inner = &mem_operand[1..mem_operand.len()-1];
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                    instr.rn = Some(self.parse_register(parts[0], line_num)?);
                    if parts.len() > 1 {
                        instr.rm = Some(self.parse_register(parts[1], line_num)?);
                    }
                } else {
                    return Err(anyhow!("Invalid memory operand: {}", mem_operand));
                }
            }

            // Memory immediate: STRI Rt, [Rn, #imm]
            Opcode::STRI | Opcode::LDRI | Opcode::STRIB | Opcode::LDRIB => {
                if operands.len() != 2 {
                    return Err(anyhow!("Expected 2 operands for {}", opcode));
                }
                instr.rd = Some(self.parse_register(operands[0], line_num)?); // Rt
                
                let mem_operand = operands[1];
                if mem_operand.starts_with('[') && mem_operand.ends_with(']') {
                    let inner = &mem_operand[1..mem_operand.len()-1];
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                    instr.rn = Some(self.parse_register(parts[0], line_num)?);
                    if parts.len() > 1 {
                        instr.imm = Some(self.parse_immediate(parts[1], line_num)?);
                    }
                } else {
                    return Err(anyhow!("Invalid memory operand: {}", mem_operand));
                }
            }

            // Stack operations: PUSH Rn
            Opcode::PUSH | Opcode::POP => {
                if operands.len() != 1 {
                    return Err(anyhow!("Expected 1 operand for {}", opcode));
                }
                instr.rd = Some(self.parse_register(operands[0], line_num)?);
            }

            // Stack all: PUSHA, POPA - no operands
            Opcode::PUSHA | Opcode::POPA => {
                if !operands.is_empty() {
                    return Err(anyhow!("{} takes no operands", opcode));
                }
            }

            // Unconditional branch: B label
            Opcode::B | Opcode::BL => {
                if operands.len() != 1 {
                    return Err(anyhow!("Expected 1 operand for {}", opcode));
                }
                let target = self.resolve_label_or_address(operands[0], line_num)?;
                // Calculate relative offset in instructions (words)
                let offset = (target as i32 - (self.current_address + 4) as i32) / 4;
                if offset < -32768 || offset > 32767 {
                    return Err(anyhow!("Branch offset out of range"));
                }
                instr.imm = Some(offset as u32 & 0xFFFF);
            }

            // Register branch: BX Rn
            Opcode::BX | Opcode::BLX => {
                if operands.len() != 1 {
                    return Err(anyhow!("Expected 1 operand for {}", opcode));
                }
                instr.rn = Some(self.parse_register(operands[0], line_num)?);
            }

            // Conditional branches: BEQ label
            Opcode::BEQ | Opcode::BNE | Opcode::BCS | Opcode::BCC |
            Opcode::BMI | Opcode::BPL | Opcode::BVS | Opcode::BVC |
            Opcode::BHI | Opcode::BLS | Opcode::BGE | Opcode::BLT |
            Opcode::BGT | Opcode::BLE => {
                if operands.len() != 1 {
                    return Err(anyhow!("Expected 1 operand for {}", opcode));
                }
                let target = self.resolve_label_or_address(operands[0], line_num)?;
                let offset = (target as i32 - (self.current_address + 4) as i32) / 4;
                if offset < -2048 || offset > 2047 {
                    return Err(anyhow!("Conditional branch offset out of range"));
                }
                instr.imm = Some(offset as u32 & 0xFFF);
            }

            // Syscalls
            Opcode::SVC | Opcode::SVC1 | Opcode::SVC2 | Opcode::SVC3 => {
                if operands.len() != 1 {
                    return Err(anyhow!("Expected 1 operand for {}", opcode));
                }
                instr.imm = Some(self.parse_immediate(operands[0], line_num)?);
            }

            // No operands
            Opcode::NOP | Opcode::HLT | Opcode::RET | Opcode::RETI | Opcode::YIELD | Opcode::BRK | Opcode::WFI => {
                if !operands.is_empty() {
                    return Err(anyhow!("{} takes no operands", opcode));
                }
            }

            _ => {
                return Err(anyhow!("Unhandled opcode: {:?}", opcode));
            }
        }

        Ok(())
    }

    fn parse_register(&self, reg_str: &str, line_num: usize) -> Result<Register> {
        let reg_str = reg_str.trim().to_lowercase();
        
        // Handle register aliases
        let reg_str = match reg_str.as_str() {
            "sp" => "r6",
            "fp" => "r7",
            "lr" => "r14",
            "pc" => "r15",
            _ => &reg_str,
        };

        // Parse R0-R15
        if reg_str.starts_with('r') {
            let num_str = &reg_str[1..];
            if let Ok(num) = num_str.parse::<u8>() {
                if num <= 15 {
                    return Ok(Register::from_u8(num).unwrap());
                }
            }
        }

        Err(anyhow!("Invalid register: {} (line {})", reg_str, line_num))
    }

    fn parse_immediate(&self, imm_str: &str, line_num: usize) -> Result<u32> {
        let imm_str = imm_str.trim();
        
        // Handle # prefix
        let imm_str = imm_str.strip_prefix('#').unwrap_or(imm_str);
        
        // Handle labels
        if let Some(&symbol) = self.symbols.get(imm_str) {
            return Ok(symbol.value);
        }

        // Parse numeric literals
        if imm_str.starts_with("0x") || imm_str.starts_with("0X") {
            u32::from_str_radix(&imm_str[2..], 16)
                .map_err(|_| anyhow!("Invalid hex immediate: {} (line {})", imm_str, line_num))
        } else if imm_str.starts_with("0b") || imm_str.starts_with("0B") {
            u32::from_str_radix(&imm_str[2..], 2)
                .map_err(|_| anyhow!("Invalid binary immediate: {} (line {})", imm_str, line_num))
        } else if imm_str.starts_with('0') && imm_str.len() > 1 {
            u32::from_str_radix(&imm_str[1..], 8)
                .map_err(|_| anyhow!("Invalid octal immediate: {} (line {})", imm_str, line_num))
        } else {
            imm_str.parse::<u32>()
                .map_err(|_| anyhow!("Invalid immediate: {} (line {})", imm_str, line_num))
        }
    }

    fn resolve_label_or_address(&self, operand: &str, line_num: usize) -> Result<u32> {
        let operand = operand.trim();
        
        // Check if it's a label
        if let Some(symbol) = self.symbols.get(operand) {
            return Ok(symbol.value);
        }
        
        // Parse as immediate address
        self.parse_immediate(operand, line_num)
    }

    pub fn get_errors(&self) -> &[AssemblerError] {
        &self.errors
    }

    pub fn get_warnings(&self) -> &[String] {
        &self.warnings
    }
}

// Example programs for testing
pub mod examples {
    use super::*;

    pub fn hello_world() -> &'static str {
        r#"
; ZEROPOINT Hello World
; Prints "Hello, ZEROPOINT!" to stdout

.section .text
.global _start

_start:
    ; Write syscall: write(1, msg, len)
    MOVI R0, #1          ; fd = 1 (stdout)
    MOVI R1, #msg        ; buffer address
    MOVI R2, #msg_len    ; length
    MOVI R7, #1          ; syscall number for write
    SVC                  ; syscall

    ; Exit syscall: exit(0)
    MOVI R0, #0          ; exit code
    MOVI R7, #0          ; syscall number for exit
    SVC                  ; syscall

.section .data
msg:
    .ascii "Hello, ZEROPOINT!\n"
msg_len = . - msg
"#
    }

    pub fn fibonacci() -> &'static str {
        r#"
; ZEROPOINT Fibonacci
; Calculates first 20 Fibonacci numbers

.section .text
.global _start

_start:
    MOVI R0, #0          ; fib(0) = 0
    MOVI R1, #1          ; fib(1) = 1
    MOVI R2, #20         ; count = 20
    MOVI R3, #0          ; loop counter

fib_loop:
    ; Print current Fibonacci number (R0)
    ; In real implementation, would call print routine
    
    ; Calculate next: R4 = R0 + R1
    ADD R4, R0, R1
    
    ; Shift: R0 = R1, R1 = R4
    MOV R0, R1
    MOV R1, R4
    
    ; Increment counter
    ADDI R3, R3, #1
    
    ; Check if done
    CMP R3, R2
    BLT fib_loop

    ; Exit
    MOVI R0, #0
    MOVI R7, #0
    SVC
"#
    }

    pub fn memory_test() -> &'static str {
        r#"
; ZEROPOINT Memory Test
; Tests memory read/write operations

.section .text
.global _start

_start:
    ; Allocate some memory (simulated)
    MOVI R0, #0x10000    ; base address
    MOVI R1, #0xDEADBEEF ; test pattern 1
    MOVI R2, #0xCAFEBABE ; test pattern 2
    
    ; Write test patterns
    STR R1, [R0, #0]
    STR R2, [R0, #4]
    
    ; Read back and verify
    LDR R3, [R0, #0]
    LDR R4, [R0, #4]
    
    ; Compare
    CMP R3, R1
    BNE mem_error
    
    CMP R4, R2
    BNE mem_error
    
    ; Success
    MOVI R0, #0
    B exit
    
mem_error:
    MOVI R0, #1
    
exit:
    MOVI R7, #0
    SVC
"#
    }
}