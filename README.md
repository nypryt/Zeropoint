# ZEROPOINT

A complete virtual computer inside a terminal, featuring a custom CPU architecture, memory management, virtual filesystem, assembly language, shell, process scheduler, and device emulation.

## Overview

ZEROPOINT is a from-scratch virtual computer system implemented in Rust. It runs entirely in a terminal using Ratatui for the UI, providing a complete computing environment with:

- **Custom 32-bit RISC CPU** with 16 registers, condition codes, and 100+ instructions
- **Virtual Memory Manager** with paging, protection, and process isolation
- **Virtual Filesystem (VFS)** with in-memory filesystem, mount points, and standard directories
- **Assembly Language** with a full assembler supporting labels, symbols, and directives
- **Interactive Shell** with 40+ built-in commands, history, aliases, and variables
- **Preemptive Multitasking Scheduler** with multiple scheduling policies
- **Device Emulation** including keyboard, display, disk, network, timer, and RTC
- **Terminal UI** with tabbed interface showing all subsystems in real-time

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ZEROPOINT Terminal UI                    │
│  ┌──────┬──────┬────────┬─────┬────────┬────────┬────────┐  │
│  │ Shell│ CPU  │ Memory │ FS  │Devices │Procs   │ Net    │  │
│  └──────┴──────┴────────┴─────┴────────┴────────┴────────┘  │
├─────────────────────────────────────────────────────────────┤
│  Shell  │  CPU    │  Memory  │  VFS   │  Scheduler │ Devices│
├─────────┼─────────┼──────────┼────────┼────────────┼────────┤
│         │         │          │        │            │        │
│  Builtin│ Register│  64MB    │ In-Mem │  Round     │Keyboard│
│  Cmds   │  File   │  Physical│  FS    │  Robin     │Display │
│         │         │          │        │            │        │
│ History │ Pipeline│ Virtual  │ Mounts │ Priority   │Disk    │
│ Aliases │  ISA    │  Memory  │        │ FCFS/SJF   │Network │
│         │         │          │        │            │        │
│ Variables│ Syscall│ Paging   │ Perms  │ Context Sw │Timer   │
│         │         │          │        │            │        │
└─────────┴─────────┴──────────┴────────┴────────────┴────────┘
```

## Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- Terminal with 256-color support

### Building

```bash
git clone <repository>
cd zeropoint
cargo build --release
```

### Running

```bash
cargo run --release
```

Or run the built binary directly:

```bash
./target/release/zeropoint
```

## Usage

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle tabs forward/backward |
| `F1`-`F8` | Jump to specific tab |
| `Ctrl+C` | Exit ZEROPOINT |
| `Up`/`Down` | Navigate lists (FS, logs, etc.) |
| `Enter` | Execute command / Enter directory |
| `Backspace` | Go up one directory (FS tab) |

### Shell Commands

**Filesystem:**
- `ls [path]` - List directory contents
- `cd [path]` - Change directory
- `pwd` - Print working directory
- `cat <file>` - Display file contents
- `mkdir <path>` - Create directory
- `rmdir <path>` - Remove empty directory
- `rm <path>` - Remove file
- `touch <path>` - Create empty file
- `cp <src> <dst>` - Copy file
- `mv <src> <dst>` - Move/rename file
- `stat <path>` - Show file status
- `find <name> [path]` - Find file by name

**System:**
- `ps` - List processes
- `kill <pid>` - Kill process
- `mem` - Show memory statistics
- `cpu` - Show CPU state
- `regs` - Show CPU registers
- `devices` - List virtual devices
- `mount` - Show mount points
- `free` - Show free memory
- `df [path]` - Show disk usage
- `uptime` - Show system uptime
- `version` - Show ZEROPOINT version
- `date` - Show current date/time
- `whoami` - Show current user
- `env` - Show environment variables
- `set/unset/export` - Variable management
- `alias/unalias` - Command aliases
- `history` - Command history

**Assembly:**
- `asm <file.asm>` - Assemble and run assembly file
- `asm -e "code"` - Assemble and run inline assembly

**Control:**
- `reboot` - Reboot virtual system
- `shutdown` - Shutdown virtual system
- `clear` / `cls` - Clear screen
- `help [cmd]` - Show help

### Assembly Programming

ZEROPOINT assembly uses a RISC-like syntax:

```assembly
; Hello World
.section .text
.global _start

_start:
    ; write(1, msg, len)
    MOVI R0, #1          ; fd = 1 (stdout)
    MOVI R1, #msg        ; buffer address
    MOVI R2, #msg_len    ; length
    MOVI R7, #1          ; syscall: write
    SVC                  ; system call

    ; exit(0)
    MOVI R0, #0
    MOVI R7, #0          ; syscall: exit
    SVC

.section .data
msg:
    .ascii "Hello, ZEROPOINT!\n"
msg_len = . - msg
```

**Key Instructions:**
- Arithmetic: `ADD`, `SUB`, `MUL`, `DIV`, `MOD`
- Logic: `AND`, `ORR`, `EOR`, `BIC`, `MOV`, `MVN`, `NEG`
- Shifts: `LSL`, `LSR`, `ASR`, `ROR` (register and immediate)
- Memory: `LDR`/`STR`, `LDRI`/`STRI`, `LDRB`/`STRB`, `PUSH`/`POP`
- Branches: `B`, `BL`, `BX`, `BLX`, conditional branches (`BEQ`, `BNE`, etc.)
- Compare: `CMP`, `CMPI`, `TST`, `TSTI`, `TEQ`
- System: `SVC`, `HLT`, `RET`, `YIELD`, `NOP`, `BRK`

**Registers:**
- `R0`-`R15` (general purpose)
- `SP` = `R6` (stack pointer)
- `FP` = `R7` (frame pointer)
- `LR` = `R14` (link register)
- `PC` = `R15` (program counter)

### Tabs Overview

1. **Shell** - Interactive command line with history and output
2. **CPU** - Register display, flags, PC/SP/LR, cycle count
3. **Memory** - Statistics, page frames, process memory regions
4. **FS** - File browser with directory navigation
5. **Devices** - Virtual device status and registers
6. **Processes** - Process table, scheduler stats, context switches
7. **Network** - Network device stats, MAC/IP, packet counters
8. **Logs** - System event log with timestamps

## Development

### Project Structure

```
src/
├── main.rs       # Entry point
├── lib.rs        # Library exports
├── cpu.rs        # CPU core, ISA, instruction execution
├── mem.rs        # Memory manager, virtual memory, paging
├── vfs.rs        # Virtual filesystem, in-memory FS
├── asm.rs        # Assembler, instruction encoding
├── shell.rs      # Shell, builtins, command parsing
├── scheduler.rs  # Process scheduler, context switching
├── device.rs     # Device emulation (KB, display, disk, net, timer, RTC)
├── ui.rs         # Ratatui terminal UI
└── fs.rs         # Filesystem utilities
```

### Running Tests

```bash
cargo test
```

### Code Style

```bash
cargo fmt
cargo clippy
```

## ISA Specification

See [ISA.md](ISA.md) for complete instruction set architecture documentation.

## Memory Layout

```
0x0000_0000 - 0x7FFF_FFFF  User Space (2GB)
0x8000_0000 - 0xFF00_0000  Kernel Space (reserved)
0xFF00_0000 - 0xFF00_0FFF  Keyboard Controller
0xFF01_0000 - 0xFF01_FFFF  Display Controller
0xFF02_0000 - 0xFF02_0FFF  Disk Controller
0xFF03_0000 - 0xFF03_0FFF  Network Controller
0xFF04_0000 - 0xFF04_0FFF  Timer
0xFF05_0000 - 0xFF05_00FF  RTC
0xFF80_0000 - 0xFFFF_FFFF  Framebuffer / Device Memory
```

## License

Apache 2.0 License - see LICENSE file for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## Acknowledgments

- Inspired by classic home computer architectures
- Built with Rust and Ratatui
- Terminal UI patterns from various open-source projects