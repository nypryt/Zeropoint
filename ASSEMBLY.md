# ZEROPOINT Assembly Language

## Overview

ZEROPOINT assembly language is a RISC-style assembly language that targets the ZEROPOINT CPU architecture. It features a two-pass assembler with label support, conditional execution, and a rich set of directives.

## Assembler Features

- Two-pass assembly (label resolution + code generation)
- Labels and symbols with local/global scope
- Conditional instruction suffixes
- Multiple number formats (decimal, hex, binary, octal)
- Section directives (.text, .data, .bss)
- Data directives (.word, .byte, .ascii, .space)
- Symbol definitions (.equ, .set)
- Macro support
- Conditional assembly (.if/.endif)
- Error reporting with line/column numbers

## Program Structure

```assembly
; Comments start with semicolon
; or double-slash //

.section .text        ; Code section (executable)
.global _start        ; Export entry point

_start:               ; Label at current address
    MOVI R0, #42      ; Instruction with immediate
    HLT               ; Halt

.section .data        ; Data section (read-write)
my_var:               ; Label = address
    .word 0x12345678  ; 32-bit value
    .byte 1, 2, 3     ; Bytes
    .ascii "Hello"    ; String without null
    .asciz "World"    ; Null-terminated string
    .space 100        ; Reserve 100 zero bytes

.section .bss         ; BSS section (zero-initialized)
buffer:               ; Label
    .space 4096       ; 4KB buffer
```

## Sections

| Section | Purpose | Permissions |
|---------|---------|-------------|
| `.text` | Executable code | Read + Execute |
| `.data` | Initialized data | Read + Write |
| `.bss` | Zero-initialized data | Read + Write |
| `.rodata` | Read-only data | Read only |

## Directives

### Section Control
```
.section name          ; Switch to section
.text                  ; Shortcut for .section .text
.data                  ; Shortcut for .section .data
.bss                   ; Shortcut for .section .bss
.rodata                ; Shortcut for .section .rodata
```

### Symbol Control
```
.global symbol         ; Export symbol (visible to linker)
.local symbol          ; Local symbol (not exported)
.weak symbol           ; Weak symbol (can be overridden)
.equ name, expression  ; Define constant (alias for .set)
.set name, expression  ; Define/set symbol value
```

### Data Definition
```
.word expression       ; 32-bit word (4 bytes)
.half expression       ; 16-bit halfword (2 bytes)
.byte expression       ; 8-bit byte (1 byte)
.quad expression       ; 64-bit quadword (8 bytes)
.float expression      ; 32-bit float
.double expression     ; 64-bit double

.ascii "string"        ; ASCII string (no null terminator)
.asciz "string"        ; Null-terminated string
.string "string"       ; Alias for .asciz

.space n               ; Reserve n zero bytes
.skip n                ; Alias for .space
.zero n                ; Alias for .space
```

### Alignment
```
.align n               ; Align to n-byte boundary
.p2align n             ; Align to 2^n byte boundary
.balign n              ; Alias for .align
```

### File Inclusion
```
.include "filename"    ; Include another assembly file
.incbin "filename"     ; Include binary file verbatim
```

### Conditional Assembly
```
.if expression         ; Assemble if expression != 0
.elseif expression     ; Alternative condition
.else                  ; Default
.endif                 ; End conditional block

.ifdef SYMBOL          ; Assemble if SYMBOL is defined
.ifndef SYMBOL         ; Assemble if SYMBOL is not defined
.endif
```

### Macros
```
.macro NAME arg1, arg2=default
    ; Macro body using \arg1, \arg2
.endm

NAME value1, value2    ; Invoke macro
```

## Instruction Syntax

### General Form
```
INSTRUCTION[CONDITION] DEST, SRC1, SRC2
```

### Operand Types

| Type | Syntax | Example |
|------|--------|---------|
| Register | `R0`-`R15`, `SP`, `LR`, `PC` | `R0`, `SP` |
| Immediate | `#value`, `#0xFF`, `#0b1010` | `#42`, `#0xFF` |
| Memory (indirect) | `[Rn]` | `[R1]` |
| Memory (offset) | `[Rn, #offset]` | `[R1, #8]` |
| Memory (indexed) | `[Rn, Rm]` | `[R1, R2]` |
| Label | `label_name` | `loop` |

### Register Names
```
R0  R1  R2  R3  R4  R5  R6  R7
R8  R9  R10 R11 R12 R13 R14 R15
SP  LR  PC  FP
```

## Arithmetic Instructions

### Three-Operand (Register)
```
ADD  Rd, Rn, Rm    ; Rd = Rn + Rm
SUB  Rd, Rn, Rm    ; Rd = Rn - Rm
MUL  Rd, Rn, Rm    ; Rd = Rn * Rm
DIV  Rd, Rn, Rm    ; Rd = Rn / Rm
MOD  Rd, Rn, Rm    ; Rd = Rn % Rm
```

### Three-Operand (Immediate)
```
ADDI Rd, Rn, #imm  ; Rd = Rn + imm
SUBI Rd, Rn, #imm  ; Rd = Rn - imm
MULI Rd, Rn, #imm  ; Rd = Rn * imm
```

### Two-Operand
```
MOV  Rd, Rm        ; Rd = Rm
MVN  Rd, Rm        ; Rd = ~Rm
NEG  Rd, Rm        ; Rd = -Rm
```

### Shift/Rotate (Register)
```
LSL  Rd, Rn, Rm    ; Rd = Rn << Rm[4:0]
LSR  Rd, Rn, Rm    ; Rd = Rn >> Rm[4:0] (logical)
ASR  Rd, Rn, Rm    ; Rd = Rn >>> Rm[4:0] (arithmetic)
ROR  Rd, Rn, Rm    ; Rd = Rn ROR Rm[4:0]
```

### Shift/Rotate (Immediate)
```
LSLI Rd, Rn, #imm  ; Rd = Rn << imm[4:0]
LSRI Rd, Rn, #imm  ; Rd = Rn >> imm[4:0]
ASRI Rd, Rn, #imm  ; Rd = Rn >>> imm[4:0]
RORI Rd, Rn, #imm  ; Rd = Rn ROR imm[4:0]
```

## Logic Instructions

### Three-Operand
```
AND  Rd, Rn, Rm    ; Rd = Rn & Rm
ORR  Rd, Rn, Rm    ; Rd = Rn | Rm
EOR  Rd, Rn, Rm    ; Rd = Rn ^ Rm
BIC  Rd, Rn, Rm    ; Rd = Rn & ~Rm (bit clear)
```

### Immediate
```
ANDI Rd, Rn, #imm  ; Rd = Rn & imm
ORI  Rd, Rn, #imm  ; Rd = Rn | imm
EORI Rd, Rn, #imm  ; Rd = Rn ^ imm
MOVI Rd, #imm      ; Rd = imm
```

## Comparison Instructions

```
CMP  Rn, Rm        ; Compare Rn - Rm (sets flags)
CMPI Rn, #imm      ; Compare Rn - imm (sets flags)
TST  Rn, Rm        ; Test Rn & Rm (sets flags)
TSTI Rn, #imm      ; Test Rn & imm (sets flags)
TEQ  Rn, Rm        ; Test Rn ^ Rm (sets flags)
```

## Memory Instructions

### Register-Register Addressing
```
STR  Rd, [Rn, Rm]     ; Mem[Rn+Rm] = Rd
LDR  Rd, [Rn, Rm]     ; Rd = Mem[Rn+Rm]
STRB Rd, [Rn, Rm]     ; Mem[Rn+Rm] = Rd[7:0]
LDRB Rd, [Rn, Rm]     ; Rd = Mem[Rn+Rm] (zero-extended)
```

### Immediate Offset
```
STRI Rd, [Rn, #imm]   ; Mem[Rn+imm] = Rd
LDRI Rd, [Rn, #imm]   ; Rd = Mem[Rn+imm]
STRIB Rd, [Rn, #imm]  ; Mem[Rn+imm] = Rd[7:0]
LDRIB Rd, [Rn, #imm]  ; Rd = Mem[Rn+imm] (zero-extended)
```

### Stack Operations
```
PUSH Rn               ; SP -= 4; Mem[SP] = Rn
POP  Rd               ; Rd = Mem[SP]; SP += 4
PUSHA                 ; Push R0-R13, LR
POPA                  ; Pop R0-R13, LR
```

## Control Flow

### Unconditional Branches
```
B    label      ; PC = PC + offset
BL   label      ; LR = PC; PC = PC + offset (call)
BX   Rn         ; PC = Rn (branch to register)
BLX  Rn         ; LR = PC; PC = Rn (call to register)
```

### Conditional Branches
```
BEQ  label      ; Branch if Equal (Z=1)
BNE  label      ; Branch if Not Equal (Z=0)
BCS  label      ; Branch if Carry Set (C=1)
BCC  label      ; Branch if Carry Clear (C=0)
BMI  label      ; Branch if Minus (N=1)
BPL  label      ; Branch if Plus (N=0)
BVS  label      ; Branch if Overflow (V=1)
BVC  label      ; Branch if No Overflow (V=0)
BHI  label      ; Branch if Higher (C=1, Z=0)
BLS  label      ; Branch if Lower or Same (C=0 or Z=1)
BGE  label      ; Branch if Greater or Equal (N=V)
BLT  label      ; Branch if Less Than (N≠V)
BGT  label      ; Branch if Greater Than (Z=0, N=V)
BLE  label      ; Branch if Less or Equal (Z=1 or N≠V)
```

### Conditional Execution (All Instructions)
```
ADDEQ  Rd, Rn, Rm   ; Add only if Z=1
MOVNE  Rd, Rm       ; Move only if Z=0
```

### Returns
```
RET    ; PC = LR
RETI   ; Return from interrupt
```

## System Calls

```
SVC                ; System call (R7 = number, R0-R3 = args)
SVC1               ; Syscall with 1 arg (R0)
SVC2               ; Syscall with 2 args (R0,R1)
SVC3               ; Syscall with 3 args (R0,R1,R2)
```

Common syscall numbers:
```
0   = exit
1   = write
2   = read
3   = open
4   = close
5   = stat
10  = brk
56  = clone (fork)
57  = execve
60  = exit_group
```

## Miscellaneous

```
NOP                ; No operation
HLT                ; Halt processor
YIELD              ; Yield to scheduler
BRK                ; Software breakpoint
WFI                ; Wait for interrupt
```

## Number Formats

| Format | Prefix | Example |
|--------|--------|---------|
| Decimal | (none) | `42`, `-10` |
| Hexadecimal | `0x` or `0X` | `0xFF`, `0x1000` |
| Binary | `0b` or `0B` | `0b1010`, `0B1111` |
| Octal | `0` | `0777` |
| Character | `'` | `'A'`, `'\n'` |

## Expressions

Supported in immediates and directives:
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `&&`, `||`, `!`
- Parentheses for grouping
- Symbol references: `label`, `symbol`

Example:
```
MOVI R0, #(0x1000 + 4 * 10)
ADDI R1, R2, #(buffer_end - buffer_start)
```

## Labels and Symbols

### Label Definition
```
label:              ; Label at current address
label = expression  ; Define symbol value
```

### Special Symbols
- `.` - Current address (location counter)
- `_start` - Default entry point
- `__LINE__` - Current line number
- `__FILE__` - Current file name

### Local Labels
```
1:                  ; Local label
    B 1b            ; Branch backward to local label 1
    B 1f            ; Branch forward to local label 1
```

## Complete Example: Hello World

```assembly
; ZEROPOINT Hello World
; Demonstrates basic I/O via syscalls

.section .text
.global _start

_start:
    ; write(1, msg, msg_len)
    ; R0 = fd (1 = stdout)
    ; R1 = buffer address
    ; R2 = length
    ; R7 = syscall number (1 = write)
    
    MOVI R0, #1              ; stdout
    MOVI R1, #msg            ; message address
    MOVI R2, #msg_len        ; message length
    MOVI R7, #1              ; SYS_write
    SVC                      ; system call
    
    ; exit(0)
    ; R0 = exit code
    ; R7 = syscall number (0 = exit)
    
    MOVI R0, #0              ; exit code 0
    MOVI R7, #0              ; SYS_exit
    SVC                      ; system call (never returns)

.section .data

msg:
    .ascii "Hello, ZEROPOINT!\n"
    ; Could also use: .asciz "Hello, ZEROPOINT!\n"
    
msg_len = . - msg            ; Calculate length at assembly time
```

## Example: Function with Stack Frame

```assembly
.section .text
.global _start
.global factorial

; int factorial(int n)
; Input: R0 = n
; Output: R0 = result

factorial:
    ; Prologue
    PUSH LR               ; Save return address
    PUSH FP               ; Save old frame pointer
    MOV FP, SP            ; Set new frame pointer
    
    ; Allocate local storage if needed
    ; SUBI SP, SP, #16    ; Allocate 16 bytes
    
    ; Base case: n <= 1
    CMPI R0, #1
    BLE  fact_base
    
    ; Recursive case: n * factorial(n-1)
    PUSH R0               ; Save n
    SUBI R0, R0, #1       ; n - 1
    BL factorial          ; factorial(n-1)
    POP R1                ; Restore n
    MUL R0, R0, R1        ; R0 = result * n
    
    B fact_done
    
fact_base:
    MOVI R0, #1           ; Return 1
    
fact_done:
    ; Epilogue
    MOV SP, FP            ; Restore stack pointer
    POP FP                ; Restore frame pointer
    POP LR                ; Restore return address
    RET                   ; Return to caller

_start:
    MOVI R0, #5           ; Calculate 5!
    BL factorial
    
    ; Result in R0, print it (simplified)
    MOVI R7, #0
    SVC
```

## Example: Memory Copy

```assembly
.section .text
.global _start

; void memcpy(void* dst, void* src, size_t n)
; R0 = dst, R1 = src, R2 = n

_start:
    ; Setup test
    MOVI R0, #dest
    MOVI R1, #src
    MOVI R2, #16
    BL memcpy
    
    MOVI R7, #0
    SVC

memcpy:
    CMP R2, #0
    BEQ memcpy_done
    
memcpy_loop:
    LDRB R3, [R1], #1      ; Load byte, post-increment src
    STRB R3, [R0], #1      ; Store byte, post-increment dst
    SUBI R2, R2, #1        ; n--
    CMP R2, #0
    BNE memcpy_loop
    
memcpy_done:
    RET

.section .data
src:
    .ascii "Hello, World!"
    .byte 0, 0, 0          ; Pad to 16 bytes

.section .bss
dest:
    .space 16
```

## Running Assembly Programs

### From Shell
```bash
# Assemble and run file
asm program.asm

# Assemble and run inline
asm -e "MOVI R0, #42; HLT"

# Run with debug
asm -d program.asm
```

### From Code
```rust
let mut assembler = Assembler::new();
let program = assembler.assemble(source_code)?;

// Load into memory at 0x10000
for (i, &word) in program.code.iter().enumerate() {
    memory.write32(pid, 0x10000 + (i * 4) as u64, word).await?;
}

// Set PC and run
cpu.set_pc(0x10000 + program.entry_point.unwrap_or(0));
cpu.halted = false;
```

## Error Handling

### Assembly Errors
- Unknown instruction
- Invalid register
- Invalid immediate
- Undefined label
- Duplicate label
- Syntax error
- Section error

### Error Format
```
error: Unknown instruction: INVALID
  --> program.asm:10:5
   |
10 | INVALID R0, R1
   |     ^^^^^^
```

## Best Practices

1. **Use meaningful labels**: `loop_start:` not `l1:`
2. **Comment complex logic**: Explain algorithms
3. **Preserve registers**: Follow calling convention
4. **Use constants**: `.equ MAX_SIZE, 1024`
5. **Align data**: `.align 4` for word access
6. **Check bounds**: Prevent buffer overflows
7. **Handle errors**: Check syscall return values

## Calling Convention

### Registers
| Register | Purpose | Preserved |
|----------|---------|-----------|
| R0-R3    | Arguments / Return values | No |
| R4-R11   | Callee-saved | Yes |
| R12      | Scratch (IP) | No |
| R13 (SP) | Stack pointer | Yes |
| R14 (LR) | Link register | No |
| R15 (PC) | Program counter | N/A |

### Stack Frame
```
Higher addresses
+------------------+
| Caller's frame   |
+------------------+
| Return address   | <- LR (saved by BL)
+------------------+
| Saved FP         | <- FP
+------------------+
| Local variables  |
| ...              |
+------------------+
| Outgoing args    | (for nested calls)
+------------------+
| Stack grows down |
Lower addresses
```

### Function Prologue
```
PUSH LR
PUSH FP
MOV FP, SP
SUBI SP, SP, #local_size
```

### Function Epilogue
```
MOV SP, FP
POP FP
POP LR
RET
```

## Differences from ARM Assembly

| Feature | ZEROPOINT | ARM |
|---------|-----------|-----|
| Instruction size | 32-bit fixed | 32/16-bit (Thumb) |
| Immediate range | 8-bit | 12-bit (with rotation) |
| Condition codes | Suffix on all | Suffix on most |
| Barrel shifter | Separate instructions | Integrated |
| Load/store multiple | PUSHA/POPA | LDM/STM |
| Coprocessors | None | CP15, VFP, NEON |
| SIMD | Extension opcodes | NEON/SVE |