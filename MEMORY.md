# ZEROPOINT Memory Management

## Overview

ZEROPOINT implements a virtual memory system with paging, per-process address spaces, and memory protection. The design separates physical memory management from virtual address translation.

## Physical Memory

### Layout
```
Total: 64MB (configurable)
Page Size: 4KB (4096 bytes)
Total Frames: 16,384
```

### Frame Allocator
- Bitmap-free list hybrid
- First-fit allocation
- O(1) allocation/deallocation
- Frame tracking for statistics

### Memory Map
```
0x0000_0000 - 0x00FF_FFFF   Frame 0 - 16383 (16MB user-accessible)
0x0100_0000 - 0x03FF_FFFF   Available for expansion
...
0xFF00_0000 - 0xFFFF_FFFF   Device memory (not managed by frame allocator)
```

## Virtual Memory

### Address Space Layout (32-bit)

```
0x0000_0000 - 0x7FFF_FFFF   User Space (2GB)
    ├── 0x0000_0000 - 0x0000_0FFF  NULL page (guard page)
    ├── 0x0000_1000 - 0x000F_FFFF  Low memory (text, data, bss)
    ├── 0x0010_0000 - 0x00FF_FFFF  Heap (grows up)
    ├── 0x0100_0000 - 0x3FFF_FFFF  Memory-mapped files / shared
    ├── 0x4000_0000 - 0x7FFD_FFFF  Available
    ├── 0x7FFE_0000 - 0x7FFE_FFFF  Stack guard page
    └── 0x7FFF_0000 - 0x7FFF_FFFF  User stack (grows down)

0x8000_0000 - 0xFFFF_FFFF   Kernel Space (2GB)
    ├── 0x8000_0000 - 0xFEFF_FFFF  Kernel text, data, heap
    ├── 0xFF00_0000 - 0xFF00_0FFF  Keyboard controller
    ├── 0xFF01_0000 - 0xFF01_FFFF  Display controller
    ├── 0xFF02_0000 - 0xFF02_0FFF  Disk controller
    ├── 0xFF03_0000 - 0xFF03_0FFF  Network controller
    ├── 0xFF04_0000 - 0xFF04_0FFF  Timer
    ├── 0xFF05_0000 - 0xFF05_00FF  RTC
    ├── 0xFF80_0000 - 0xFFFF_FFFF  Framebuffer / device memory
```

### Page Table Structure

Each process has its own page table covering user space:

```
PageTable {
    entries: [PageTableEntry; 524288]  // 2GB / 4KB
}
```

### Page Table Entry

```rust
struct PageTableEntry {
    physical_addr: u64,      // Physical frame address
    permissions: PagePermission,
    present: bool,           // Page is in memory
    user_accessible: bool,   // Accessible from user mode
    dirty: bool,             // Page has been written
    accessed: bool,          // Page has been accessed
}
```

### Page Permissions

| Permission | Value | Description |
|------------|-------|-------------|
| None | 0 | No access |
| Read | 1 | Read-only |
| Write | 2 | Write-only |
| Execute | 4 | Execute-only |
| ReadWrite | 3 | Read + Write |
| ReadExecute | 5 | Read + Execute |
| WriteExecute | 6 | Write + Execute |
| ReadWriteExecute | 7 | Full access |

## Memory Manager API

### Process Memory Management

```rust
// Create new process address space
fn create_process(&mut self) -> Result<u32>

// Destroy process and free all memory
fn destroy_process(&mut self, pid: u32) -> Result<()>

// Map virtual memory region
fn map_memory(&mut self, pid: u32, virt_addr: u64, size: usize, 
              permissions: PagePermission, name: String) -> Result<u64>

// Unmap memory region
fn unmap_memory(&mut self, pid: u32, virt_addr: u64) -> Result<()>
```

### Memory Access

```rust
// Read operations (with bounds checking)
async fn read8(&self, pid: u32, addr: u64) -> Result<u8>
async fn read16(&self, pid: u32, addr: u64) -> Result<u16>
async fn read32(&self, pid: u32, addr: u64) -> Result<u32>
async fn read64(&self, pid: u32, addr: u64) -> Result<u64>

// Write operations (with permission checking)
async fn write8(&mut self, pid: u32, addr: u64, value: u8) -> Result<()>
async fn write16(&mut self, pid: u32, addr: u64, value: u16) -> Result<()>
async fn write32(&mut self, pid: u32, addr: u64, value: u32) -> Result<()>
async fn write64(&mut self, pid: u32, addr: u64, value: u64) -> Result<()>

// Bulk operations
async fn read_bytes(&self, pid: u32, addr: u64, buf: &mut [u8]) -> Result<()>
async fn write_bytes(&mut self, pid: u32, addr: u64, buf: &[u8]) -> Result<()>
```

### Statistics

```rust
struct MemoryStats {
    total_memory: usize,
    used_memory: usize,
    free_memory: usize,
    total_frames: usize,
    used_frames: usize,
    free_frames: usize,
    num_processes: usize,
}
```

## Address Translation

### Translation Process

```
Virtual Address (32-bit)
    │
    ├── VPN (20 bits) ──────► Page Table Lookup
    │                           │
    │                           ▼
    │                    PageTableEntry
    │                           │
    │                           ├── present?
    │                           ├── permissions OK?
    │                           └── user_accessible?
    │
    └── Offset (12 bits) ─────► Physical Address = Frame + Offset
```

### Page Fault Handling

1. **Not Present**: Page fault - would trigger swap-in (future)
2. **Permission Violation**: Protection fault - SIGSEGV to process
3. **User Access to Kernel**: Protection fault - SIGSEGV

Current implementation returns errors; no demand paging yet.

## Memory Regions

### Region Tracking

Each process maintains a list of memory regions:

```rust
struct MemoryRegion {
    base: u64,
    size: usize,
    permissions: PagePermission,
    name: String,
    is_mapped: bool,
    file_backed: Option<String>,
    offset: u64,
}
```

### Standard Regions

| Region | Address Range | Permissions | Purpose |
|--------|---------------|-------------|---------|
| Text | 0x0000_1000+ | RX | Program code |
| Data | After text | RW | Initialized data |
| BSS | After data | RW | Zero-initialized data |
| Heap | 0x0010_0000+ | RW | Dynamic allocation |
| Stack | 0x7FFF_F000- | RW | Call stack |
| MMap | 0x4000_0000+ | RWX | Memory-mapped files |

### Region Operations

- **Create**: `map_memory()` - Allocates frames, creates page table entries
- **Destroy**: `unmap_memory()` - Frees frames, clears page table entries
- **Resize**: Unmap + map (future: in-place expansion)
- **Permissions**: `mprotect()` equivalent (future)

## Stack Management

### Initial Stack
- Located at top of user space: 0x7FFF_F000
- Grows downward
- Guard page at 0x7FFE_F000
- Initial size: 64KB (16 pages)

### Stack Operations
- `PUSH`/`POP` instructions manipulate SP
- `CALL`/`RET` use LR for return address
- Stack overflow detected via guard page
- No automatic stack growth (future)

## Heap Management

### Current Implementation
- Simple `brk`/`sbrk` style
- Heap starts at 0x0010_0000
- Grows upward toward mmap region
- No free list or malloc implementation yet

### Future: Allocator
- Buddy allocator for kernel
- dlmalloc/jemalloc style for user
- Per-thread arenas
- Guard pages between allocations

## Shared Memory

### Current
- No shared memory between processes
- Each process has isolated page tables
- Kernel space shared (identity mapped)

### Future
- `shmget`/`shmat` syscalls
- Copy-on-write for fork()
- Memory-mapped file sharing
- POSIX shared memory API

## Memory Protection

### User/Kernel Separation
- User processes cannot access kernel space (0x8000_0000+)
- Page tables enforce user_accessible flag
- System calls transition via SVC instruction

### Process Isolation
- Separate page tables per PID
- No direct access to other process memory
- Shared memory requires explicit setup

### Permission Enforcement
- Read/Write/Execute checked on every access
- Self-modifying code requires RWX permissions
- Execute-only pages for code (future)

## Page Replacement (Future)

### Algorithms Considered
- **Clock/Second Chance**: Simple, effective
- **LRU**: Optimal but expensive
- **LFU**: Good for some workloads
- **ARC**: Adaptive, complex

### Implementation Plan
- Page daemon thread
- Reference/accessed bits in PTE
- Dirty bit for writeback
- Swap space on virtual disk

## DMA and Device Memory

### Device Memory Mapping
- Devices mapped at fixed addresses in kernel space
- Direct access via memory-mapped I/O
- No page table entries for device regions in user page tables

### DMA (Future)
- DMA controller manages transfers
- Double-buffering for device-to-memory
- Scatter-gather support
- Interrupt on completion

## Statistics and Monitoring

### Available Metrics
- Total/used/free frames
- Per-process memory usage
- Page fault counts (future)
- Swap usage (future)
- Allocation latency

### Shell Commands
```
mem        - Show memory statistics
free       - Alias for mem
```

### UI Display
- Memory tab shows stats and regions
- Real-time updates
- Per-process breakdown

## Error Handling

### Common Errors
| Error | Cause | Handling |
|-------|-------|----------|
| Out of memory | No free frames | Return error, OOM killer (future) |
| Invalid address | Unmapped page | Page fault / SIGSEGV |
| Permission denied | Wrong perms | Protection fault / SIGSEGV |
| Alignment fault | Unaligned access | Emulate or SIGBUS |
| Double free | Double unmap | Detect and panic |

## Performance Considerations

### Optimizations
- Page table caching (TLB simulation)
- Large page support (2MB, 1GB) - future
- Zero-page sharing for BSS
- Copy-on-write for fork()
- Lazy allocation for heap

### Overhead
- Page table walk: ~3 memory accesses
- Context switch: page table pointer swap
- System call: minimal (register save)

## Future Enhancements

### Planned
- Demand paging with swap
- Memory-mapped files
- Shared memory (shm)
- Huge pages (2MB/1GB)
- NUMA awareness
- Memory compression
- Page deduplication

### Research
- Speculative page allocation
- Machine learning for prefetching
- Persistent memory support
- CXL memory expansion