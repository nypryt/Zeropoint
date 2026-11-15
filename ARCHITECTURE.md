# ZEROPOINT Architecture

## System Overview

ZEROPOINT is a modular virtual computer system with clearly defined subsystem boundaries. Each subsystem communicates through well-defined interfaces, enabling independent development and testing.

## Subsystem Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            ZEROPOINT System                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │   Shell     │◄───│    CPU      │◄───│   Memory    │◄───│    VFS      │  │
│  │  (UI/Front) │    │  (Execute)  │    │  (Physical) │    │  (Storage)  │  │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘    └──────┬──────┘  │
│         │                  │                  │                  │          │
│         │                  ▼                  │                  │          │
│         │    ┌─────────────────────┐         │                  │          │
│         │    │    Scheduler        │         │                  │          │
│         │    │  (Process Mgmt)     │         │                  │          │
│         │    └──────────┬──────────┘         │                  │          │
│         │               │                    │                  │          │
│         ▼               ▼                    ▼                  ▼          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      Device Manager                                  │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │   │
│  │  │ Keyboard │ │ Display  │ │  Disk    │ │ Network  │ │ Timer/RTC│  │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## CPU Subsystem (`cpu.rs`)

### Design Principles
- Single-cycle instruction execution model
- 16 general-purpose 32-bit registers (R0-R15)
- ARM-inspired condition codes (N, Z, C, V)
- Fixed 32-bit instruction encoding
- Memory-mapped I/O for device access

### Register File
| Register | Alias | Purpose |
|----------|-------|---------|
| R0-R5    | -     | General purpose |
| R6       | SP    | Stack pointer |
| R7       | FP    | Frame pointer |
| R8-R13   | -     | General purpose |
| R14      | LR    | Link register |
| R15      | PC    | Program counter |

### Instruction Format
```
31-24: Opcode (8 bits)
23-20: Rd (4 bits)
19-16: Rn (4 bits)
15-12: Rm (4 bits)
11-8:  Condition (4 bits)
7-0:   Immediate (8 bits)
```

### Pipeline
```
Fetch → Decode → Execute → Memory → Writeback
  ▲                                              │
  └──────────────────────────────────────────────┘
```

### Exception Handling
- **SVC** (Supervisor Call) - System calls
- **IRQ** - Device interrupts
- **HLT** - Halt instruction
- **BRK** - Software breakpoint
- **Undefined Instruction** - Invalid opcode

## Memory Subsystem (`mem.rs`)

### Physical Memory
- Configurable size (default 64MB)
- Page size: 4KB
- Frame allocation with free list
- Statistics tracking

### Virtual Memory
- Per-process page tables
- 32-bit virtual address space
- User space: 0x0000_0000 - 0x7FFF_FFFF
- Kernel space: 0x8000_0000 - 0xFFFF_FFFF
- Page permissions: Read, Write, Execute

### Page Table Structure
```
PageTableEntry {
    physical_addr: u64,
    permissions: PagePermission,
    present: bool,
    user_accessible: bool,
    dirty: bool,
    accessed: bool,
}
```

### Memory Regions
- Tracked per-process
- Support for file-backed mappings (future)
- Overlap detection
- Automatic frame management

## Filesystem Subsystem (`vfs.rs`)

### VFS Layer
- Abstract filesystem interface
- Multiple filesystem support
- Mount point management
- Path resolution with longest-prefix matching
- File descriptor table per process

### In-Memory Filesystem
- Tree structure with inodes
- Support for: Regular files, Directories, Symlinks
- Metadata: permissions, timestamps, ownership
- Atomic operations

### Directory Structure
```
/
├── bin/        # Executables
├── etc/        # Configuration
├── home/       # User directories
├── tmp/        # Temporary files
├── dev/        # Device files
├── proc/       # Process info (virtual)
├── sys/        # System info (virtual)
├── mnt/        # Mount points
├── root/       # Root home
├── usr/        # User programs
└── var/        # Variable data
```

## Assembler Subsystem (`asm.rs`)

### Two-Pass Design
1. **Pass 1**: Label collection, address calculation
2. **Pass 2**: Instruction encoding, relocation

### Features
- Labels and symbols
- Conditional instruction suffixes
- Immediate values (decimal, hex, binary, octal)
- Directives: `.section`, `.global`, `.ascii`, `.equ`
- Error reporting with line/column numbers

### Instruction Encoding
Matches CPU decoder exactly:
- Register-register: `OP Rd, Rn, Rm`
- Register-immediate: `OP Rd, Rn, #imm`
- Memory: `OP Rd, [Rn, Rm]` or `OP Rd, [Rn, #imm]`
- Branches: `OP label` (PC-relative)
- System: `SVC #num`

## Shell Subsystem (`shell.rs`)

### Architecture
- Command parser with quote/escape handling
- Built-in command registry
- Alias expansion
- Variable substitution
- History management
- Pipeline support (future)

### Built-in Commands (40+)
Categorized by function:
- **Filesystem**: ls, cd, pwd, cat, mkdir, rmdir, rm, touch, cp, mv, stat
- **Process**: ps, kill
- **Memory**: mem, cpu, regs, free
- **Devices**: devices, mount
- **Network**: netstat (future)
- **Assembly**: asm
- **System**: reboot, shutdown, version, uptime, date, whoami
- **Environment**: set, unset, export, env, alias, unalias, history
- **Help**: help, clear

## Scheduler Subsystem (`scheduler.rs`)

### Process Model
```
Process States:
  Created → Ready ↔ Running → Blocked → Terminated → Zombie
                    ↖_________↗
```

### Scheduling Policies
1. **Round Robin** (default) - Time-sliced, priority queues
2. **Priority** - Highest priority first
3. **FCFS** - First come, first served
4. **SJF** - Shortest job first (by CPU time)

### Context Switch
- Save/restore full register set
- Update CPU time accounting
- Ready queue management
- Idle process when no work

### Process Hierarchy
- Parent-child relationships
- Orphan reaping to init (PID 1)
- Exit code propagation
- Process groups (future)

## Device Subsystem (`device.rs`)

### Device Model
- Memory-mapped I/O at fixed addresses
- Register-based interface
- Interrupt generation
- DMA support (framework)

### Standard Devices

| Device | Base Address | Size | IRQ |
|--------|--------------|------|-----|
| Keyboard | 0xFF00_0000 | 4KB | 1 |
| Display | 0xFF01_0000 | 64KB | 2 |
| Disk | 0xFF02_0000 | 4KB | 3 |
| Network | 0xFF03_0000 | 4KB | 4 |
| Timer | 0xFF04_0000 | 4KB | 5 |
| RTC | 0xFF05_0000 | 256B | 6 |

### Keyboard (PS/2 style)
- Scancode input buffer
- LED control (caps, num, scroll)
- Repeat delay/rate
- Interrupt on key press/release

### Display (VGA style)
- 1024x768 @ 32bpp
- Double-buffered framebuffer
- Hardware cursor
- VSync interrupt at 60Hz
- Palette support

### Disk (VirtIO Block)
- 64MB virtual disk
- 512-byte sectors
- PIO and DMA modes
- Identify command
- Read/write sector commands

### Network (VirtIO Net)
- MAC address assignment
- IP configuration
- RX/TX queues
- Packet statistics
- Promiscuous/multicast modes

### Timer (HPET style)
- Configurable frequency
- One-shot and periodic modes
- 64-bit counter
- Compare match interrupt

### RTC (CMOS)
- Real-time clock
- Date/time registers
- Battery-backed (simulated)

## UI Subsystem (`ui.rs`)

### Tabbed Interface
- 8 tabs: Shell, CPU, Memory, FS, Devices, Processes, Network, Logs
- Keyboard navigation (Tab, F1-F8)
- Real-time updates (100ms interval)
- Responsive rendering (60 FPS target)

### Rendering Architecture
```
Frame Loop:
  1. Handle input events
  2. Update subsystem state
  3. Render current tab
  4. Render status bar
  5. Present frame
```

### Data Flow
- Read-only access to subsystems via `blocking_read()`
- No direct mutation from UI thread
- Commands executed through shell subsystem

## Concurrency Model

### Threads
1. **Main/UI Thread** - Terminal rendering, input handling
2. **CPU Thread** - Instruction execution loop
3. **Scheduler Thread** - Context switching, time slicing
4. **Device Thread** - Device polling, interrupt generation
5. **Async Runtime** - Tokio for async I/O (future)

### Synchronization
- `Arc<RwLock<T>>` for shared subsystem state
- `tokio::sync::RwLock` for async coordination
- `parking_lot::RwLock` for synchronous hot paths
- Message passing for cross-thread communication

### Memory Safety
- No `unsafe` in subsystem cores
- `unsafe` only for FFI (none currently)
- Borrow checking enforced at compile time
- No data races possible with lock-based design

## Boot Sequence

```
1. ZeroPoint::new()
   ├── MemoryManager::new(64MB)
   ├── VFS::new() → mount inmem at /
   ├── CPU::new(memory, vfs)
   ├── Scheduler::new() → create idle process
   ├── DeviceManager::new() → register 6 devices
   ├── Shell::new(cpu, mem, vfs, sched, devices)
   └── UI::new(all subsystems)

2. ZeroPoint::run()
   ├── Spawn CPU execution loop
   ├── Spawn scheduler loop
   ├── Spawn device polling loop
   └── Run UI main loop (blocking)

3. Shell initialization
   ├── Create standard directories
   ├── Set prompt
   └── Display welcome message

4. User interaction begins
```

## Shutdown Sequence

```
1. User presses Ctrl+C or types 'shutdown'
2. UI sets running = false
3. All background loops exit on next poll
4. Terminal restored to normal mode
5. Process exits cleanly
```

## Extension Points

### Adding Instructions
1. Add opcode to `Opcode` enum in `cpu.rs`
2. Add decoding in `Instruction::decode()`
3. Add execution in `CPU::execute()`
4. Add assembler support in `asm.rs`
5. Add tests

### Adding Devices
1. Implement `Device` trait
2. Assign base address and IRQ
3. Register in `DeviceManager::new()`
4. Add UI panel in `ui.rs`
5. Add shell commands if needed

### Adding Filesystems
1. Implement `FileSystem` trait
2. Register with `VFS::register_filesystem()`
3. Mount at desired path
4. Add shell integration

### Adding Scheduling Policies
1. Add variant to `SchedulingPolicy` enum
2. Implement selection in `Scheduler::schedule()`
3. Add configuration in shell

## Performance Characteristics

| Metric | Target |
|--------|--------|
| Instruction throughput | >10 MIPS |
| Context switch latency | <10μs |
| Memory allocation | <1μs |
| FS operation | <100μs |
| UI frame time | <16ms (60 FPS) |
| Memory overhead | <10MB host |

## Security Boundaries

- No host filesystem access from guest
- No host network access from guest
- Memory isolation between processes
- No code execution on host
- All I/O through emulated devices

## Future Architecture Changes

### Planned
- Multi-core CPU simulation
- MMU with TLB
- Demand paging with swap
- ELF binary loading
- TCP/IP stack in network device
- GPU device with 3D acceleration
- Audio device
- USB device emulation

### Under Consideration
- JIT compilation for CPU
- WASM as guest binary format
- Distributed ZEROPOINT clusters
- Checkpoint/restore for VM migration