# ZEROPOINT Security

## Threat Model

ZEROPOINT is a virtual computer running as a user-space application. The security model assumes:

### Trusted
- The ZEROPOINT binary itself
- The host operating system kernel
- The Rust standard library and dependencies

### Untrusted
- Guest code running inside ZEROPOINT (assembly programs, shell commands)
- Virtual filesystem contents
- Network packets from virtual network
- Keyboard/mouse input
- Disk images

### Security Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    Host System                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                  ZEROPOINT Process                        │ │
│  │  ┌─────────────────────────────────────────────────────┐ │ │
│  │  │           Guest Virtual Computer                    │ │ │
│  │  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────────┐  │ │ │
│  │  │  │ CPU  │ │ Mem  │ │ VFS  │ │ Sched│ │ Devices  │  │ │ │
│  │  │  └──────┘ └──────┘ └──────┘ └──────┘ └──────────┘  │ │ │
│  │  └─────────────────────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Memory Isolation

### Process Isolation
- Each process has separate page tables
- No direct memory access between processes
- Kernel space (0x8000_0000+) inaccessible from user mode
- Guard pages around stack

### Memory Protection
- Page-level permissions (R/W/X)
- Execute-only pages for code (future)
- No-execute stack/heap (NX bit simulation)
- ASLR for process memory layout (future)

### Buffer Overflow Protection
- Bounds checking on all memory accesses
- Stack canaries (future)
- Fortify source (future)
- No unsafe pointer arithmetic in guest code

## Filesystem Security

### Access Control
- POSIX-style permissions (owner/group/other)
- Read/Write/Execute bits
- Setuid/setgid (future)
- Sticky bit on /tmp

### Path Safety
- Path resolution stays within VFS
- No host filesystem access
- Symlink traversal limits (max 40)
- No absolute paths outside VFS root

### Virtual Filesystems
- /proc: Read-only process info
- /sys: Read-only system info
- /dev: Controlled device access
- No procfs write access to kernel structures

## Network Security

### Virtual Network
- Isolated virtual network stack
- No raw socket access to host
- MAC address spoofing possible (guest-controlled)
- No promiscuous mode by default

### Host Network Access
- No direct host network access
- No host port binding
- No packet injection to host
- Future: Controlled bridge/tap with host permission

## Device Security

### Device Isolation
- Devices memory-mapped in kernel space only
- User processes access via syscalls
- No direct device register access from user mode
- DMA contained to guest physical memory

### Device Files (/dev/)
- Controlled access via permissions
- No raw device access without capability
- Null/zero/random safe by design
- Disk writes contained to virtual disk image

## System Call Security

### Syscall Validation
- All arguments validated
- Pointer bounds checked
- Integer overflow protection
- Capability checks (future)

### Dangerous Syscalls
| Syscall | Risk | Mitigation |
|---------|------|------------|
| execve | Code execution | Validate path, args |
| clone | Process creation | Resource limits |
| mmap | Memory mapping | Permission checks |
| ptrace | Process inspection | Restricted (future) |

## Input Validation

### Shell Input
- Command parsing with quote handling
- No eval/exec of arbitrary strings
- Path sanitization
- Argument validation

### Assembly Input
- Two-pass assembler with validation
- No arbitrary code execution at assemble time
- Instruction encoding verified
- Label/symbol resolution bounded

### Keyboard Input
- Scancode validation
- No direct memory writes from keyboard
- Repeat rate limiting

## Resource Limits

### Per-Process Limits (Future)
- Max memory: 64MB default
- Max CPU time: Unlimited (configurable)
- Max open files: 256
- Max processes: 1024
- Max stack: 8MB

### System-Wide Limits
- Total memory: 64MB (configurable)
- Total processes: 4096
- Total open files: 8192

## Denial of Service Protection

### CPU
- YIELD instruction for cooperative multitasking
- Preemptive scheduler with time slices
- No infinite loops in kernel code
- Watchdog timer (future)

### Memory
- Frame allocator with limits
- OOM killer (future)
- No memory leaks in kernel
- Statistics monitoring

### Disk
- Fixed virtual disk size
- No host disk access
- Quota support (future)

### Network
- Packet queue limits (128 RX/TX)
- Rate limiting (future)
- No amplification attacks

## Secure Boot (Future)

### Boot Process
1. Verify ZEROPOINT binary signature
2. Load trusted kernel modules
3. Initialize hardware with known state
4. Mount read-only root filesystem
5. Start init with minimal privileges

### Measured Boot
- Hash each component
- Store in TPM (simulated)
- Remote attestation (future)

## Cryptography (Future)

### Algorithms
- AES-256 for disk encryption
- ChaCha20-Poly1305 for network
- Ed25519 for signatures
- Argon2 for key derivation

### Key Management
- Keys never in guest memory
- Host-managed key store
- Hardware-backed (host TPM/SE)

## Auditing and Logging

### Event Logging
- All syscalls logged (optional)
- Failed access attempts
- Privilege escalation attempts
- Resource exhaustion events

### Log Integrity
- Append-only logs
- Cryptographic chaining (future)
- Remote log shipping (future)

## Vulnerability Response

### Reporting
- Security issues: security@zeropoint.local
- Public disclosure after fix
- CVE assignment for significant issues

### Patch Process
1. Assess severity (CVSS)
2. Develop fix
3. Test thoroughly
4. Release patch
5. Update documentation

### Supported Versions
- Latest release: Full support
- Previous release: Security fixes only
- Older: Best effort

## Secure Coding Guidelines

### For ZEROPOINT Developers
- No `unsafe` without justification
- All inputs validated
- Error handling on all paths
- No secret logging
- Constant-time crypto (future)
- Fuzzing for parsers

### For Guest Developers
- Validate all inputs
- Check syscall returns
- Use safe string functions
- Limit resource usage
- No self-modifying code

## Compliance

### Standards Alignment
- POSIX security model (subset)
- Common Criteria EAL1 (target)
- NIST 800-53 controls (subset)
- ISO 27001 principles

### Certifications (Future)
- FIPS 140-2 for crypto
- Common Criteria evaluation
- SOC 2 Type II

## Incident Response

### Detection
- Anomalous syscall patterns
- Resource usage spikes
- Failed login attempts
- Memory corruption detection

### Containment
- Process termination
- Network isolation
- Disk snapshot
- Memory dump

### Recovery
- Clean reboot
- State restoration
- Forensic analysis
- Patch deployment

## Future Security Enhancements

### Planned
- Mandatory Access Control (MAC)
- Seccomp-style syscall filtering
- Capability-based security
- Sandbox escape prevention
- Formal verification of critical paths
- Hardware-backed isolation (Intel VT-x, AMD-V)
- Encrypted guest memory
- Secure enclaves (SGX-like)

### Research
- CHERI capabilities
- WebAssembly sandboxing
- eBPF for policy enforcement
- Memory tagging (MTE)
- Shadow stacks
- Control-flow integrity (CFI)