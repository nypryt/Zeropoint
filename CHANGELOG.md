# ZEROPOINT Changelog

All notable changes to ZEROPOINT will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure
- Custom 32-bit RISC CPU with 100+ instructions
- Virtual memory manager with paging
- In-memory virtual filesystem (VFS)
- Two-pass assembler with label support
- Interactive shell with 40+ built-in commands
- Preemptive process scheduler (RR, Priority, FCFS, SJF)
- Device emulation: Keyboard, Display, Disk, Network, Timer, RTC
- Ratatui-based terminal UI with 8 tabs
- Comprehensive documentation (README, ARCHITECTURE, ISA, MEMORY, FILESYSTEM, ASSEMBLY, SHELL, DEVICES, SECURITY)

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A

## [0.1.0] - 2026-11-15

### Added
- **CPU Subsystem**
  - 16 general-purpose 32-bit registers (R0-R15, SP, LR, PC, FP)
  - 100+ instructions: arithmetic, logic, shifts, memory, branches, syscalls
  - Condition codes (N, Z, C, V) with conditional execution
  - System call interface (SVC) with 200+ syscall numbers
  - Interrupt handling (IRQ, FIQ, SVC, undefined instruction)
  - Breakpoint and trace support
  - Cycle-accurate execution model

- **Memory Subsystem**
  - 64MB physical memory (configurable)
  - 4KB page size
  - Per-process page tables (2GB user space)
  - Page permissions (R/W/X)
  - Frame allocator with free list
  - Memory region tracking per process
  - Statistics collection

- **Filesystem Subsystem**
  - VFS layer with mount point support
  - In-memory filesystem with inodes
  - Regular files, directories, symlinks
  - POSIX-style permissions and metadata
  - Standard directory layout (/bin, /etc, /home, /tmp, /dev, /proc, /sys, /mnt, /root, /usr, /var)
  - File descriptor management

- **Assembler**
  - Two-pass assembly (labels + code gen)
  - Conditional instruction suffixes
  - Multiple number formats (dec/hex/bin/oct)
  - Directives: .section, .global, .word, .byte, .ascii, .space, .align, .equ
  - Macros and conditional assembly
  - Error reporting with line/column

- **Shell**
  - 40+ built-in commands
  - Command history (1000 entries)
  - Aliases with argument support
  - Shell variables with export
  - Tab completion (future)
  - Assembly integration (asm command)

- **Scheduler**
  - Process states: Created, Ready, Running, Blocked, Terminated, Zombie
  - Scheduling policies: Round Robin, Priority, FCFS, SJF
  - Time-sliced preemptive multitasking
  - Context switch with full register save/restore
  - Process hierarchy (fork, exec, exit)
  - Idle process when no work

- **Device Emulation**
  - Keyboard: PS/2 style, scancodes, LEDs, repeat
  - Display: 1024x768x32, double-buffered, VSync IRQ
  - Disk: 64MB, 512-byte sectors, VirtIO Block compatible
  - Network: VirtIO Net compatible, MAC/IP config
  - Timer: HPET style, 1MHz, one-shot/periodic
  - RTC: CMOS style, host-time based

- **UI (Ratatui)**
  - 8 tabs: Shell, CPU, Memory, FS, Devices, Processes, Network, Logs
  - Keyboard navigation (Tab, F1-F8)
  - Real-time updates (100ms interval)
  - 60 FPS rendering target
  - Status bar with system info

- **Documentation**
  - README.md - Project overview and quick start
  - ARCHITECTURE.md - System architecture and design
  - ISA.md - Complete instruction set reference
  - MEMORY.md - Memory management details
  - FILESYSTEM.md - VFS and filesystem implementation
  - ASSEMBLY.md - Assembly language guide
  - SHELL.md - Shell command reference
  - DEVICES.md - Device emulation details
  - SECURITY.md - Security model and practices
  - CHANGELOG.md - This file
  - TODO.md - Development roadmap

### Changed
- N/A (initial release)

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- Process memory isolation via page tables
- Kernel/user space separation
- No host filesystem access from guest
- No host network access from guest
- Validated syscall arguments
- Resource limits (configurable)

---

## Release Notes

### Version 0.1.0 - "Foundation"

This is the initial release of ZEROPOINT, establishing the core virtual computer architecture. All major subsystems are implemented and integrated:

**What Works:**
- Boot to interactive shell
- Run assembly programs
- Navigate filesystem
- Monitor CPU, memory, processes, devices
- Network packet simulation
- Timer interrupts

**Known Limitations:**
- No demand paging / swap
- No ELF binary loading
- No TCP/IP stack
- No graphics beyond framebuffer
- No audio
- No persistent storage (all in-memory)
- Limited shell scripting
- No multi-core support
- No hardware virtualization

**Next Priorities:**
1. Demand paging with swap to virtual disk
2. ELF binary loader
3. TCP/IP stack in network device
4. Persistent disk images
5. Shell scripting language
6. Multi-core CPU simulation
7. Audio device
8. GPIO/I2C/SPI devices
9. USB host controller
10. Hardware virtualization support

---

## Upgrade Guide

### From 0.0.x (Development)
- Complete rewrite - no upgrade path
- Fresh installation required

### Breaking Changes
- N/A (initial release)

---

## Contributors

- Initial development by ZEROPOINT team

## License

MIT License - see LICENSE file for details.