# ZEROPOINT Development Roadmap

This file tracks the development status and future work for ZEROPOINT.

## Legend
- ✅ Done
- 🔄 In Progress
- ⏳ Planned
- 💡 Idea / Research
- ❌ Blocked / Won't Fix

---

## Core Subsystems

### CPU
- ✅ Basic instruction set (100+ instructions)
- ✅ Register file (R0-R15, SP, LR, PC, FP)
- ✅ Condition codes (N, Z, C, V)
- ✅ Conditional execution
- ✅ System calls (SVC)
- ✅ Interrupts (IRQ, FIQ, SVC, undefined)
- ✅ Breakpoints and tracing
- ⏳ Floating point instructions (FADD, FSUB, FMUL, FDIV, FCMP, FMOV, FCVT)
- ⏳ Vector/SIMD instructions (VADD, VSUB, VMUL, VLD, VST)
- ⏳ Cryptographic instructions (AES, SHA)
- ⏳ 64-bit mode (ZP64)
- ⏳ Instruction cache simulation
- ⏳ Branch prediction simulation
- ⏳ Out-of-order execution hints
- ⏳ Performance counters
- ⏳ JIT compilation for hot code paths

### Memory Management
- ✅ Physical memory allocator (64MB, 4KB pages)
- ✅ Virtual memory with page tables
- ✅ Page permissions (R/W/X)
- ✅ Per-process address spaces
- ✅ Memory region tracking
- ✅ Basic statistics
- ⏳ Demand paging with swap
- ⏳ Page replacement algorithms (Clock, LRU)
- ⏳ Memory-mapped files
- ⏳ Shared memory (shm)
- ⏳ Huge pages (2MB, 1GB)
- ⏳ Copy-on-write for fork()
- ⏳ ASLR
- ⏳ NX bit (no-execute pages)
- ⏳ Memory compression
- ⏳ Page deduplication
- ⏳ NUMA awareness

### Filesystem
- ✅ VFS layer with mount points
- ✅ In-memory filesystem
- ✅ Regular files, directories, symlinks
- ✅ POSIX permissions and metadata
- ✅ Standard directory layout
- ✅ File descriptor management
- ⏳ Persistent disk images
- ⏳ ext2/ext4 compatibility
- ⏳ FAT32 for disk images
- ⏳ tarfs (read-only tar mounting)
- ⏳ initramfs
- ⏳ hostfs (controlled host access)
- ⏳ File locking (flock, fcntl)
- ⏳ Extended attributes (xattr)
- ⏳ ACLs
- ⏳ Quotas
- ⏳ Snapshots
- ⏳ Union mounts
- ⏳ Directory entry caching
- ⏳ Read-ahead / write-back

### Assembler
- ✅ Two-pass assembly
- ✅ Labels and symbols
- ✅ Conditional suffixes
- ✅ Number formats (dec/hex/bin/oct)
- ✅ Directives (.section, .global, .word, .byte, .ascii, .space, .align, .equ)
- ✅ Macros
- ✅ Conditional assembly
- ✅ Error reporting
- ⏳ Include file support
- ⏳ Incbin (binary include)
- ⏳ Expression evaluation improvements
- ⏳ Relocation support
- ⏳ Object file output (ELF)
- ⏳ Linker integration
- ⏳ Debug info (DWARF)
- ⏳ Optimization passes

### Shell
- ✅ 40+ built-in commands
- ✅ Command history
- ✅ Aliases
- ✅ Variables with export
- ✅ Assembly integration
- ⏳ Tab completion
- ⏳ Command substitution `$(cmd)`
- ⏳ Arithmetic expansion `$((expr))`
- ⏳ Brace expansion `{a,b,c}`
- ⏳ Globbing `*.txt`
- ⏳ Here documents `<<EOF`
- ⏳ Redirection (`>`, `>>`, `<`, `2>`, `|`)
- ⏳ Shell functions
- ⏳ Local variables in functions
- ⏳ Script execution (.zsh files)
- ⏳ Profile/rc file loading (~/.zshrc)
- ⏳ Job control (bg, fg, jobs)
- ⏳ Signal handling (trap)
- ⏳ Colored output
- ⏳ Man pages
- ⏳ Custom key bindings

### Scheduler
- ✅ Process states
- ✅ Round Robin scheduling
- ✅ Priority scheduling
- ✅ FCFS scheduling
- ✅ SJF scheduling
- ✅ Time-sliced preemption
- ✅ Context switch
- ✅ Process hierarchy (fork, exec, exit)
- ✅ Idle process
- ⏳ Real-time scheduling (FIFO, RR)
- ⏳ CPU affinity
- ⏳ cgroups / resource limits
- ⏳ Fair scheduling (CFS-style)
- ⏳ Deadline scheduling
- ⏳ Power-aware scheduling
- ⏳ NUMA-aware scheduling

### Devices
- ✅ Keyboard (PS/2)
- ✅ Display (VGA, framebuffer)
- ✅ Disk (VirtIO Block)
- ✅ Network (VirtIO Net)
- ✅ Timer (HPET)
- ✅ RTC (CMOS)
- ✅ Interrupt controller
- ✅ DMA framework
- ⏳ Audio (PCM, mixer)
- ⏳ Mouse (PS/2, USB)
- ⏳ GPIO
- ⏳ I2C / SPI
- ⏳ USB host controller
- ⏳ PCI bus
- ⏳ SD/MMC
- ⏳ Watchdog timer
- ⏳ Hardware RNG
- ⏳ Crypto accelerator
- ⏳ Bluetooth
- ⏳ WiFi

### UI (Ratatui)
- ✅ 8 tabs (Shell, CPU, Memory, FS, Devices, Processes, Network, Logs)
- ✅ Tab navigation (Tab, F1-F8)
- ✅ Shell with history and input
- ✅ CPU register display
- ✅ Memory statistics and regions
- ✅ Filesystem browser
- ✅ Device list with registers
- ✅ Process table
- ✅ Network statistics
- ✅ System logs
- ✅ Status bar
- ⏳ Mouse support
- ⏳ Split panes
- ⏳ Configurable layout
- ⏳ Themes / color schemes
- ⏳ Custom key bindings
- ⏳ Plugin system
- ⏳ Remote UI (web)
- ⏳ Accessibility (screen readers)

---

## Integration & System

### Boot & Initialization
- ✅ Basic boot sequence
- ✅ Subsystem initialization order
- ⏳ Secure boot verification
- ⏳ Measured boot (TPM simulation)
- ⏳ Boot parameters / cmdline
- ⏳ Initramfs loading
- ⏳ Systemd-style service manager
- ⏳ Runlevels / targets

### Binary Formats
- ✅ Raw binary execution
- ⏳ ELF loader (32-bit)
- ⏳ ELF loader (64-bit)
- ⏳ Shared library support (ld.so)
- ⏳ DWARF debug info
- ⏳ WASM runtime
- ⏳ Java class file (future)

### Networking Stack
- ✅ VirtIO Net device
- ⏳ TCP/IP stack
- ⏳ UDP support
- ⏳ ICMP (ping)
- ⏳ DHCP client
- ⏳ DNS resolver
- ⏳ HTTP client
- ⏳ TLS/SSL
- ⏳ Firewall / packet filter
- ⏳ VLAN support
- ⏳ Bridge / tap (host networking)

### Graphics
- ✅ Framebuffer (32-bit BGRA)
- ✅ Double buffering
- ✅ Hardware cursor
- ⏳ 2D acceleration (blit, fill, copy)
- ⏳ Text rendering (fonts)
- ⏳ Window manager
- ⏳ Compositor
- ⏳ OpenGL / Vulkan (virtio-gpu)
- ⏳ Wayland protocol

### Audio
- ⏳ PCM playback
- ⏳ Mixer (volume, balance)
- ⏳ MIDI synthesis
- ⏳ Audio capture
- ⏳ AC97 / HDA emulation
- ⏳ VirtIO sound

### Storage
- ✅ VirtIO Block device
- ⏳ Disk image formats (qcow2, vmdk, raw)
- ⏳ Snapshots
- ⏳ Thin provisioning
- ⏳ RAID (software)
- ⏳ LVM
- ⏳ Encryption (LUKS)
- ⏳ NVMe emulation
- ⏳ SCSI emulation

### Input
- ✅ Keyboard
- ⏳ Mouse (absolute/relative)
- ⏳ Touchscreen
- ⏳ Gamepad / joystick
- ⏳ Tablet / stylus

---

## Developer Experience

### Tooling
- ✅ Cargo build system
- ✅ Unit tests (cargo test)
- ✅ Linting (cargo clippy)
- ✅ Formatting (cargo fmt)
- ⏳ Debugger (GDB stub)
- ⏳ Profiler (perf-like)
- ⏳ Tracing / flame graphs
- ⏳ Code coverage
- ⏳ Benchmark suite
- ⏳ Fuzzing infrastructure
- ⏳ Static analysis
- ⏳ Documentation generation
- ⏳ Release automation

### Testing
- ✅ Unit tests for core modules
- ⏳ Integration tests
- ⏳ End-to-end tests
- ⏳ Regression tests
- ⏳ Property-based testing
- ⏳ Chaos testing
- ⏳ Performance benchmarks
- ⏳ Compatibility tests (Linux, Windows, macOS)

### Documentation
- ✅ README.md
- ✅ ARCHITECTURE.md
- ✅ ISA.md
- ✅ MEMORY.md
- ✅ FILESYSTEM.md
- ✅ ASSEMBLY.md
- ✅ SHELL.md
- ✅ DEVICES.md
- ✅ SECURITY.md
- ✅ CHANGELOG.md
- ✅ TODO.md
- ⏳ API reference (rustdoc)
- ⏳ Tutorial series
- ⏳ Video demos
- ⏳ Man pages
- ⏳ Migration guides
- ⏳ FAQ

---

## Platform Support

### Tier 1 (Full Support)
- ✅ Linux x86_64
- ⏳ Windows x86_64
- ⏳ macOS x86_64 / ARM64

### Tier 2 (Best Effort)
- ⏳ Linux ARM64
- ⏳ Linux RISC-V
- ⏳ FreeBSD
- ⏳ Android/Termux

### Tier 3 (Experimental)
- ⏳ WebAssembly (browser)
- ⏳ Embedded (no_std)
- ⏳ UEFI application

---

## Security

### Implemented
- ✅ Process memory isolation
- ✅ Kernel/user space separation
- ✅ Validated syscall arguments
- ✅ No host filesystem access
- ✅ No host network access
- ✅ Device isolation

### Planned
- ⏳ Capability-based security
- ⏳ Mandatory Access Control (MAC)
- ⏳ Seccomp-style syscall filtering
- ⏳ Sandbox escape prevention
- ⏳ Encrypted guest memory
- ⏳ Secure enclaves
- ⏳ Formal verification of critical paths
- ⏳ CHERI capabilities
- ⏳ Memory tagging (MTE)
- ⏳ Shadow stacks
- ⏳ Control-flow integrity (CFI)

---

## Performance

### Targets
- ✅ >10 MIPS instruction throughput
- ⏳ <10μs context switch
- ⏳ <1μs memory allocation
- ⏳ <100μs FS operations
- ⏳ <16ms UI frame time (60 FPS)
- ⏳ <10MB host memory overhead

### Optimizations
- ⏳ TLB simulation
- ⏳ Page table caching
- ⏳ Instruction cache
- ⏳ JIT compilation
- ⏳ Zero-copy I/O
- ⏳ Async I/O (io_uring)
- ⏳ SIMD acceleration (host)
- ⏳ Multi-threaded CPU cores

---

## Research & Experimental

### Languages
- 💡 Rust for kernel (current)
- 💡 Zig for device drivers
- 💡 C for legacy compatibility
- 💡 Assembly for boot code
- 💡 WebAssembly for guest apps

### Architectures
- 💡 RISC-V backend
- 💡 ARM64 backend
- 💡 x86_64 backend
- 💡 LoongArch backend

### Novel Ideas
- 💡 Distributed ZEROPOINT clusters
- 💡 Checkpoint/restore migration
- 💡 Time-travel debugging
- 💡 Deterministic replay
- 💡 Speculative execution
- 💡 Neural network acceleration
- 💡 Quantum computing simulation
- 💡 Blockchain integration

---

## Milestones

### v0.2.0 - "Persistence" (Q1 2027)
- [ ] Disk image persistence
- [ ] ELF binary loading
- [ ] Basic TCP/IP stack
- [ ] Shell scripting
- [ ] Mouse support
- [ ] Audio device

### v0.3.0 - "Networking" (Q2 2027)
- [ ] Full TCP/IP stack
- [ ] DHCP/DNS
- [ ] HTTP/HTTPS
- [ ] Package manager
- [ ] SSH server
- [ ] Web server

### v0.4.0 - "Graphics" (Q3 2027)
- [ ] Window manager
- [ ] Compositor
- [ ] Font rendering
- [ ] 2D acceleration
- [ ] Wayland support
- [ ] GPU virtualization

### v0.5.0 - "Multi-core" (Q4 2027)
- [ ] SMP CPU simulation
- [ ] Thread support
- [ ] Locking primitives
- [ ] NUMA support
- [ ] Scalable scheduler

### v1.0.0 - "Stable" (2028)
- [ ] API stability guarantee
- [ ] Comprehensive test suite
- [ ] Performance benchmarks
- [ ] Security audit
- [ ] Documentation complete
- [ ] Package distribution

---

## Contributing

### Priority Labels
- `priority: critical` - Blocks release
- `priority: high` - Important for next milestone
- `priority: medium` - Nice to have
- `priority: low` - Future consideration

### Good First Issues
- Add missing syscall implementations
- Improve shell command help text
- Add UI keyboard shortcuts
- Write unit tests
- Improve documentation
- Fix clippy warnings
- Add benchmark benchmarks

### Architecture Decisions
Record major decisions in `docs/adr/` (Architecture Decision Records)

---

## Notes

- This roadmap is living document
- Priorities shift based on user feedback
- Contributions welcome in any area
- See CONTRIBUTING.md for guidelines
- Join discussion on GitHub Discussions

*Last updated: 2026-11-15*