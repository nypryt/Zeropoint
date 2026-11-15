# ZEROPOINT Instruction Set Architecture (ISA)

## Overview

ZEROPOINT implements a 32-bit RISC-style instruction set architecture with:
- 16 general-purpose 32-bit registers
- Fixed 32-bit instruction encoding
- Condition codes for conditional execution
- Memory-mapped I/O
- Supervisor calls for system services

## Register File

| Register | Name | Alias | Description |
|----------|------|-------|-------------|
| R0 | r0 | - | General purpose / syscall return |
| R1 | r1 | - | General purpose / syscall arg 1 |
| R2 | r2 | - | General purpose / syscall arg 2 |
| R3 | r3 | - | General purpose / syscall arg 3 |
| R4 | r4 | - | General purpose |
| R5 | r5 | - | General purpose |
| R6 | r6 | SP | Stack pointer |
| R7 | r7 | FP | Frame pointer |
| R8 | r8 | - | General purpose |
| R9 | r9 | - | General purpose |
| R10 | r10 | - | General purpose |
| R11 | r11 | - | General purpose |
| R12 | r12 | - | General purpose |
| R13 | r13 | - | General purpose |
| R14 | r14 | LR | Link register |
| R15 | r15 | PC | Program counter |

## Condition Codes

The processor maintains four condition flags in the CPSR:

| Flag | Bit | Name | Description |
|------|-----|------|-------------|
| N | 31 | Negative | Result is negative (bit 31 = 1) |
| Z | 30 | Zero | Result is zero |
| C | 29 | Carry | Unsigned overflow / borrow |
| V | 28 | Overflow | Signed overflow |

### Condition Suffixes

Instructions can be conditionally executed by appending a condition code:

| Suffix | Code | Flags | Meaning |
|--------|------|-------|---------|
| EQ | 0000 | Z=1 | Equal |
| NE | 0001 | Z=0 | Not equal |
| CS/HS | 0010 | C=1 | Carry set / Unsigned higher or same |
| CC/LO | 0011 | C=0 | Carry clear / Unsigned lower |
| MI | 0100 | N=1 | Minus / Negative |
| PL | 0101 | N=0 | Plus / Positive or zero |
| VS | 0110 | V=1 | Overflow set |
| VC | 0111 | V=0 | Overflow clear |
| HI | 1000 | C=1, Z=0 | Unsigned higher |
| LS | 1001 | C=0 or Z=1 | Unsigned lower or same |
| GE | 1010 | N=V | Signed greater or equal |
| LT | 1011 | N≠V | Signed less than |
| GT | 1100 | Z=0, N=V | Signed greater than |
| LE | 1101 | Z=1 or N≠V | Signed less or equal |
| AL | 1110 | Any | Always (default) |
| NV | 1111 | Never | Never execute |

Example: `ADDEQ R0, R1, R2` - Add only if Z flag is set.

## Instruction Encoding

All instructions are 32 bits:

```
31-24: Opcode (8 bits)
23-20: Rd (destination register, 4 bits)
19-16: Rn (first source register, 4 bits)
15-12: Rm (second source register, 4 bits)
11-8:  Condition (4 bits)
7-0:   Immediate value (8 bits)
```

For immediate instructions, Rm field is repurposed as part of immediate.

## Instruction Set

### Arithmetic & Logic (Register-Register)

| Mnemonic | Opcode | Operation | Flags |
|----------|--------|-----------|-------|
| ADD Rd, Rn, Rm | 0x00 | Rd = Rn + Rm | N,Z,C,V |
| SUB Rd, Rn, Rm | 0x01 | Rd = Rn - Rm | N,Z,C,V |
| MUL Rd, Rn, Rm | 0x02 | Rd = Rn * Rm | N,Z |
| DIV Rd, Rn, Rm | 0x03 | Rd = Rn / Rm | N,Z |
| MOD Rd, Rn, Rm | 0x04 | Rd = Rn % Rm | N,Z |
| AND Rd, Rn, Rm | 0x05 | Rd = Rn & Rm | N,Z |
| ORR Rd, Rn, Rm | 0x06 | Rd = Rn \| Rm | N,Z |
| EOR Rd, Rn, Rm | 0x07 | Rd = Rn ^ Rm | N,Z |
| BIC Rd, Rn, Rm | 0x08 | Rd = Rn & ~Rm | N,Z |
| MOV Rd, Rm | 0x09 | Rd = Rm | N,Z |
| MVN Rd, Rm | 0x0A | Rd = ~Rm | N,Z |
| NEG Rd, Rm | 0x0B | Rd = -Rm | N,Z,C,V |
| LSL Rd, Rn, Rm | 0x0C | Rd = Rn << Rm[4:0] | N,Z,C |
| LSR Rd, Rn, Rm | 0x0D | Rd = Rn >> Rm[4:0] | N,Z,C |
| ASR Rd, Rn, Rm | 0x0E | Rd = Rn >>> Rm[4:0] | N,Z,C |
| ROR Rd, Rn, Rm | 0x0F | Rd = Rn ROR Rm[4:0] | N,Z,C |

### Arithmetic & Logic (Immediate)

| Mnemonic | Opcode | Operation | Flags |
|----------|--------|-----------|-------|
| ADDI Rd, Rn, #imm | 0x10 | Rd = Rn + imm | N,Z,C,V |
| SUBI Rd, Rn, #imm | 0x11 | Rd = Rn - imm | N,Z,C,V |
| MULI Rd, Rn, #imm | 0x12 | Rd = Rn * imm | N,Z |
| ANDI Rd, Rn, #imm | 0x13 | Rd = Rn & imm | N,Z |
| ORI Rd, Rn, #imm | 0x14 | Rd = Rn \| imm | N,Z |
| EORI Rd, Rn, #imm | 0x15 | Rd = Rn ^ imm | N,Z |
| MOVI Rd, #imm | 0x16 | Rd = imm | N,Z |
| LSLI Rd, Rn, #imm | 0x17 | Rd = Rn << imm[4:0] | N,Z,C |
| LSRI Rd, Rn, #imm | 0x18 | Rd = Rn >> imm[4:0] | N,Z,C |
| ASRI Rd, Rn, #imm | 0x19 | Rd = Rn >>> imm[4:0] | N,Z,C |
| RORI Rd, Rn, #imm | 0x1A | Rd = Rn ROR imm[4:0] | N,Z,C |

### Comparison Instructions

| Mnemonic | Opcode | Operation | Flags |
|----------|--------|-----------|-------|
| CMP Rn, Rm | 0x20 | Rn - Rm | N,Z,C,V |
| CMPI Rn, #imm | 0x21 | Rn - imm | N,Z,C,V |
| TST Rn, Rm | 0x22 | Rn & Rm | N,Z |
| TSTI Rn, #imm | 0x23 | Rn & imm | N,Z |
| TEQ Rn, Rm | 0x24 | Rn ^ Rm | N,Z |

### Memory Instructions

| Mnemonic | Opcode | Operation |
|----------|--------|-----------|
| STR Rd, [Rn, Rm] | 0x30 | Mem[Rn+Rm] = Rd |
| STRI Rd, [Rn, #imm] | 0x31 | Mem[Rn+imm] = Rd |
| LDR Rd, [Rn, Rm] | 0x32 | Rd = Mem[Rn+Rm] |
| LDRI Rd, [Rn, #imm] | 0x33 | Rd = Mem[Rn+imm] |
| STRB Rd, [Rn, Rm] | 0x34 | Mem[Rn+Rm] = Rd[7:0] |
| STRIB Rd, [Rn, #imm] | 0x35 | Mem[Rn+imm] = Rd[7:0] |
| LDRB Rd, [Rn, Rm] | 0x36 | Rd = Mem[Rn+Rm] (zero-extended) |
| LDRIB Rd, [Rn, #imm] | 0x37 | Rd = Mem[Rn+imm] (zero-extended) |
| PUSH Rn | 0x38 | SP -= 4; Mem[SP] = Rn |
| POP Rd | 0x39 | Rd = Mem[SP]; SP += 4 |
| PUSHA | 0x3A | Push R0-R13, LR |
| POPA | 0x3B | Pop R0-R13, LR |

### Control Flow

| Mnemonic | Opcode | Operation |
|----------|--------|-----------|
| B label | 0x40 | PC = PC + offset*4 |
| BL label | 0x41 | LR = PC; PC = PC + offset*4 |
| BX Rn | 0x42 | PC = Rn |
| BLX Rn | 0x43 | LR = PC; PC = Rn |
| BEQ label | 0x44 | If Z: PC = PC + offset*4 |
| BNE label | 0x45 | If !Z: PC = PC + offset*4 |
| BCS label | 0x46 | If C: PC = PC + offset*4 |
| BCC label | 0x47 | If !C: PC = PC + offset*4 |
| BMI label | 0x48 | If N: PC = PC + offset*4 |
| BPL label | 0x49 | If !N: PC = PC + offset*4 |
| BVS label | 0x4A | If V: PC = PC + offset*4 |
| BVC label | 0x4B | If !V: PC = PC + offset*4 |
| BHI label | 0x4C | If C && !Z: PC = PC + offset*4 |
| BLS label | 0x4D | If !C || Z: PC = PC + offset*4 |
| BGE label | 0x4E | If N==V: PC = PC + offset*4 |
| BLT label | 0x4F | If N!=V: PC = PC + offset*4 |
| BGT label | 0x50 | If !Z && N==V: PC = PC + offset*4 |
| BLE label | 0x51 | If Z || N!=V: PC = PC + offset*4 |

Branch offsets are signed 12-bit (conditional) or 16-bit (unconditional), scaled by 4.

### System Calls

| Mnemonic | Opcode | Description |
|----------|--------|-------------|
| SVC | 0x60 | System call (R7 = syscall number) |
| SVC1 | 0x61 | Syscall with 1 arg (R0) |
| SVC2 | 0x62 | Syscall with 2 args (R0,R1) |
| SVC3 | 0x63 | Syscall with 3 args (R0,R1,R2) |

System call convention:
- R7 = syscall number
- R0-R3 = arguments
- R0 = return value
- Other registers preserved

### Control & Exception

| Mnemonic | Opcode | Description |
|----------|--------|-------------|
| NOP | 0x70 | No operation |
| HLT | 0x71 | Halt processor |
| RET | 0x72 | Return (PC = LR) |
| RETI | 0x73 | Return from interrupt |
| YIELD | 0x74 | Yield to scheduler |
| BRK | 0x75 | Breakpoint |
| WFI | 0x76 | Wait for interrupt |

### Floating Point (Extension)

| Mnemonic | Opcode | Description |
|----------|--------|-------------|
| FADD | 0x80 | Floating-point add |
| FSUB | 0x81 | Floating-point subtract |
| FMUL | 0x82 | Floating-point multiply |
| FDIV | 0x83 | Floating-point divide |
| FCMP | 0x84 | Floating-point compare |
| FMOV | 0x85 | Floating-point move |
| FCVT | 0x86 | Float/int conversion |

### Vector/SIMD (Extension)

| Mnemonic | Opcode | Description |
|----------|--------|-------------|
| VADD | 0x90 | Vector add |
| VSUB | 0x91 | Vector subtract |
| VMUL | 0x92 | Vector multiply |
| VLD | 0x93 | Vector load |
| VST | 0x94 | Vector store |

### Cryptographic (Extension)

| Mnemonic | Opcode | Description |
|----------|--------|-------------|
| AES | 0xA0 | AES encryption/decryption |
| SHA | 0xA1 | SHA hashing |

## Addressing Modes

### Register Direct
```
ADD R0, R1, R2    ; R0 = R1 + R2
```

### Immediate
```
ADDI R0, R1, #42  ; R0 = R1 + 42
MOVI R0, #0xFF    ; R0 = 255
```

### Register Indirect
```
LDR R0, [R1]      ; R0 = Mem[R1]
STR R0, [R1]      ; Mem[R1] = R0
```

### Register Indirect with Offset
```
LDRI R0, [R1, #8] ; R0 = Mem[R1 + 8]
STRI R0, [R1, #8] ; Mem[R1 + 8] = R0
```

### Base + Index
```
LDR R0, [R1, R2]  ; R0 = Mem[R1 + R2]
STR R0, [R1, R2]  ; Mem[R1 + R2] = R0
```

### Stack Relative
```
PUSH R0           ; SP -= 4; Mem[SP] = R0
POP R0            ; R0 = Mem[SP]; SP += 4
```

### PC-Relative (Branches)
```
B label           ; PC = PC + offset*4
BL func           ; LR = PC; PC = PC + offset*4
BEQ label         ; If Z: PC = PC + offset*4
```

## System Calls

| Number | Name | Arguments | Return |
|--------|------|-----------|--------|
| 0 | exit | R0 = code | - |
| 1 | write | R0=fd, R1=buf, R2=len | bytes written |
| 2 | read | R0=fd, R1=buf, R2=len | bytes read |
| 3 | open | R0=path, R1=flags, R2=mode | fd |
| 4 | close | R0=fd | 0/-1 |
| 5 | stat | R0=path, R1=buf | 0/-1 |
| 6 | fstat | R0=fd, R1=buf | 0/-1 |
| 7 | lseek | R0=fd, R1=offset, R2=whence | new offset |
| 10 | brk | R0=new_end | old_end |
| 56 | clone | R0=flags, R1=stack, R2=ptid, R3=tls | child PID |
| 57 | execve | R0=path, R1=argv, R2=envp | -1 on error |
| 60 | exit_group | R0=code | - |
| 72 | fdatasync | R0=fd | 0/-1 |
| 79 | getcwd | R0=buf, R1=size | 0/-1 |
| 80 | chdir | R0=path | 0/-1 |
| 83 | mkdir | R0=path, R1=mode | 0/-1 |
| 84 | rmdir | R0=path | 0/-1 |
| 85 | creat | R0=path, R1=mode | fd |
| 87 | unlink | R0=path | 0/-1 |
| 88 | symlink | R0=target, R1=linkpath | 0/-1 |
| 89 | readlink | R0=path, R1=buf, R2=size | bytes read |
| 90 | chmod | R0=path, R1=mode | 0/-1 |
| 93 | chown | R0=path, R1=uid, R2=gid | 0/-1 |
| 102 | getuid | - | uid |
| 103 | getgid | - | gid |
| 104 | geteuid | - | euid |
| 105 | getegid | - | egid |
| 120 | getpid | - | pid |
| 124 | getppid | - | ppid |
| 169 | time | R0=buf | time_t |
| 186 | gettimeofday | R0=tv, R1=tz | 0/-1 |
| 201 | getuid32 | - | uid |
| 202 | getgid32 | - | gid |

## Assembly Language Syntax

### Program Structure
```assembly
.section .text        ; Code section
.global _start        ; Export symbol

_start:               ; Label (entry point)
    MOVI R0, #0
    HLT               ; Halt

.section .data        ; Data section
my_var:               ; Label
    .word 42          ; 32-bit value
    .byte 1,2,3       ; Bytes
    .ascii "hello"    ; ASCII string (no null)
    .asciz "world"    ; Null-terminated string
    .space 100        ; Reserve 100 bytes
    .align 4          ; Align to 4-byte boundary
```

### Directives

| Directive | Description |
|-----------|-------------|
| `.section name` | Switch section (.text, .data, .bss) |
| `.global symbol` | Export symbol for linking |
| `.word value` | 32-bit word |
| `.half value` | 16-bit halfword |
| `.byte value` | 8-bit byte |
| `.ascii "str"` | ASCII string (no null) |
| `.asciz "str"` | Null-terminated string |
| `.space n` | Reserve n zero bytes |
| `.align n` | Align to n-byte boundary |
| `.equ name, value` | Define constant |
| `.set name, value` | Define/set symbol |
| `.include "file"` | Include another file |

### Symbols and Labels

```
label:              ; Label at current address
symbol = expression ; Define symbol value
.local symbol       ; Local symbol (not exported)
.weak symbol        ; Weak symbol
```

### Expressions

Supported in immediates and directives:
- Decimal: `42`, `-10`
- Hexadecimal: `0xFF`, `0x1000`
- Binary: `0b1010`, `0B1111`
- Octal: `0777`
- Character: `'A'`, `'\n'`
- Symbol reference: `label`, `symbol`
- Arithmetic: `label + 4`, `symbol * 2`
- Parentheses: `(a + b) * 2`

### Conditional Assembly

```
.if condition
    ; code
.elseif condition
    ; code
.else
    ; code
.endif

.ifdef SYMBOL
    ; code if SYMBOL defined
.endif

.ifndef SYMBOL
    ; code if SYMBOL not defined
.endif
```

### Macros

```
.macro NAME arg1, arg2
    MOVI \arg1, \arg2
.endm

NAME R0, #42    ; Expands to: MOVI R0, #42
```

## Example Programs

### Hello World
```assembly
.section .text
.global _start

_start:
    MOVI R0, #1          ; stdout
    MOVI R1, #msg
    MOVI R2, #msg_len
    MOVI R7, #1          ; write
    SVC

    MOVI R0, #0
    MOVI R7, #0          ; exit
    SVC

.section .data
msg:
    .ascii "Hello, ZEROPOINT!\n"
msg_len = . - msg
```

### Fibonacci
```assembly
.section .text
.global _start

_start:
    MOVI R0, #0          ; fib(0)
    MOVI R1, #1          ; fib(1)
    MOVI R2, #20         ; count

loop:
    ; Print R0 (simplified - would call print routine)
    
    ADD R3, R0, R1       ; next = a + b
    MOV R0, R1           ; a = b
    MOV R1, R3           ; b = next
    
    SUBI R2, R2, #1      ; count--
    CMP R2, #0
    BGT loop             ; if count > 0, continue
    
    MOVI R7, #0
    SVC                  ; exit
```

### Memory Copy
```assembly
.section .text
.global _start

; void memcpy(void* dst, void* src, size_t n)
; R0 = dst, R1 = src, R2 = n

_start:
    CMP R2, #0
    BEQ done

copy_loop:
    LDRB R3, [R1], #1    ; Load byte, post-increment src
    STRB R3, [R0], #1    ; Store byte, post-increment dst
    SUBI R2, R2, #1      ; n--
    CMP R2, #0
    BNE copy_loop

done:
    MOVI R7, #0
    SVC
```

## Instruction Timing

| Category | Cycles |
|----------|--------|
| ALU (reg-reg) | 1 |
| ALU (immediate) | 1 |
| Load/Store | 2-3 |
| Branch (taken) | 2 |
| Branch (not taken) | 1 |
| System call | 10+ |
| Multiply | 3-5 |
| Divide | 10-20 |

## Exceptions and Interrupts

### Exception Vector Table
```
0x0000_0000  Reset
0x0000_0004  Undefined Instruction
0x0000_0008  Supervisor Call (SVC)
0x0000_000C  Prefetch Abort
0x0000_0010  Data Abort
0x0000_0014  Reserved
0x0000_0018  IRQ
0x0000_001C  FIQ
```

### Interrupt Handling
1. Save PC to LR_irq, CPSR to SPSR_irq
2. Set PC to vector address
3. Enter IRQ mode, disable IRQs
4. Execute handler
5. Restore CPSR from SPSR_irq
6. Return with `SUBS PC, LR, #4`

### Device IRQ Assignments
| IRQ | Device |
|-----|--------|
| 1 | Keyboard |
| 2 | Display (VSync) |
| 3 | Disk |
| 4 | Network |
| 5 | Timer |
| 6 | RTC |

## Binary Format

### Executable Header (Future)
```
Magic: "ZPXE" (4 bytes)
Version: 1 (2 bytes)
Flags: (2 bytes)
Entry Point: (4 bytes)
Program Header Offset: (4 bytes)
Section Header Offset: (4 bytes)
```

### Raw Binary
Current assembler output is raw 32-bit words loaded at 0x10000.

## Compatibility

### ARM Similarities
- Register naming (R0-R15, SP, LR, PC)
- Condition codes
- Instruction mnemonics
- Barrel shifter
- Load/store architecture

### ARM Differences
- Fixed 32-bit encoding only (no Thumb)
- 8-bit immediate field
- Different opcode assignments
- Simplified addressing modes
- No coprocessor interface
- No NEON/SVE

### x86 Differences
- RISC vs CISC
- Fixed vs variable length
- Load/store vs register-memory
- Condition codes vs flags register
- Explicit vs implicit operands

## Future Extensions

### Planned
- 64-bit mode (ZP64)
- SIMD/Vector extensions
- Hardware virtualization
- Memory protection keys
- Transactional memory
- Cryptographic instructions

### Considered
- Compressed instructions (16-bit)
- Predicated execution (all instructions)
- Register windows
- Stack caching
- Out-of-order execution hints