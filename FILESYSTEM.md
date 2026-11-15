# ZEROPOINT Virtual Filesystem

## Overview

ZEROPOINT implements a Virtual Filesystem (VFS) layer that provides a unified interface to multiple filesystem implementations. The default filesystem is an in-memory filesystem suitable for a virtual computer environment.

## VFS Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        VFS Layer                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Path Resolution ──► Mount Point Lookup ──► FS Driver       │
│       │                    │                    │             │
│       ▼                    ▼                    ▼             │
│  /home/user        /home (mount)         inmem FS           │
│  file.txt                                     │             │
│                                              │             │
│  File Descriptor Table (per-process)         │             │
│       │                                      │             │
│       ▼                                      ▼             │
│  fd 0,1,2 (stdin,stdout,stderr)          Inode Ops       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Filesystem Interface

All filesystems implement the `FileSystem` trait:

```rust
trait FileSystem: Send + Sync {
    fn name(&self) -> &str;
    
    // File operations
    fn read(&self, inode: u64, offset: u64, buf: &mut [u8]) -> Result<usize>;
    fn write(&self, inode: u64, offset: u64, buf: &[u8]) -> Result<usize>;
    
    // Directory operations
    fn create(&self, parent: u64, name: &str, file_type: FileType, mode: FileMode) -> Result<u64>;
    fn unlink(&self, parent: u64, name: &str) -> Result<()>;
    fn mkdir(&self, parent: u64, name: &str, mode: FileMode) -> Result<u64>;
    fn rmdir(&self, parent: u64, name: &str) -> Result<()>;
    fn lookup(&self, parent: u64, name: &str) -> Result<Option<u64>>;
    fn readdir(&self, inode: u64, offset: u64, entries: &mut Vec<DirEntry>) -> Result<usize>;
    
    // Metadata
    fn stat(&self, inode: u64) -> Result<FileMetadata>;
    fn setattr(&self, inode: u64, metadata: &FileMetadata) -> Result<()>;
    
    // Symlinks
    fn symlink(&self, parent: u64, name: &str, target: &str) -> Result<u64>;
    fn readlink(&self, inode: u64) -> Result<String>;
}
```

## In-Memory Filesystem

### Data Structures

```rust
struct InMemoryFile {
    metadata: FileMetadata,
    data: Vec<u8>,                    // File contents
    children: HashMap<String, u64>,   // Directory entries
    symlink_target: Option<String>,   // For symlinks
}
```

### Inode Allocation
- Sequential allocation starting from 1
- Inode 1 = root directory
- No inode recycling (simple, sufficient for virtual system)

### Features
- **Regular files**: Byte arrays with dynamic resizing
- **Directories**: HashMap of name → inode
- **Symlinks**: Store target path string
- **Metadata**: Full POSIX-style stat structure

## VFS Operations

### Path Resolution

```
resolve_path("/home/user/file.txt")
    │
    ├── Find longest matching mount point
    │   └── "/" matches (root mount)
    │
    ├── Resolve relative to mount: "home/user/file.txt"
    │
    ├── Split components: ["home", "user", "file.txt"]
    │
    ├── Walk directory tree:
    │   ├── lookup(root, "home") → inode 2
    │   ├── lookup(inode 2, "user") → inode 5
    │   └── lookup(inode 5, "file.txt") → inode 10
    │
    └── Return (filesystem, inode 10, "file.txt")
```

### Mount Points

```rust
struct MountPoint {
    fs: Arc<dyn FileSystem>,
    mount_point: String,    // e.g., "/mnt/usb"
    root_inode: u64,        // Root inode of mounted FS
}
```

Multiple filesystems can be mounted at different paths.

### File Descriptors

Per-process file descriptor table:
```
fd 0: stdin  (keyboard input)
fd 1: stdout (display output)
fd 2: stderr (display output)
fd 3+: User files
```

Each FD tracks:
- Filesystem reference
- Inode number
- Current offset
- Open flags (read/write/append)
- Original path (for debugging)

## Standard Directory Layout

```
/
├── bin/        (drwxr-xr-x)  Executables
├── etc/        (drwxr-xr-x)  Configuration
├── home/       (drwxr-xr-x)  User directories
│   └── user/   (drwxr-xr-x)  Default user home
├── tmp/        (drwxrwxrwt)  Temporary files (sticky bit)
├── dev/        (drwxr-xr-x)  Device files
├── proc/       (dr-xr-xr-x)  Process info (virtual)
├── sys/        (dr-xr-xr-x)  System info (virtual)
├── mnt/        (drwxr-xr-x)  Mount points
├── root/       (drwx------)  Root home
├── usr/        (drwxr-xr-x)  User programs
│   ├── bin/    (drwxr-xr-x)
│   ├── lib/    (drwxr-xr-x)
│   └── share/  (drwxr-xr-x)
└── var/        (drwxr-xr-x)  Variable data
    ├── log/    (drwxr-xr-x)
    └── spool/  (drwxr-xr-x)
```

## File Types

| Type | Constant | Description |
|------|----------|-------------|
| Regular | `FileType::Regular` | Normal file |
| Directory | `FileType::Directory` | Directory |
| Symlink | `FileType::Symlink` | Symbolic link |
| Char Device | `FileType::CharacterDevice` | Character device |
| Block Device | `FileType::BlockDevice` | Block device |
| FIFO | `FileType::Fifo` | Named pipe |
| Socket | `FileType::Socket` | Unix domain socket |

## File Modes / Permissions

```
Mode bits (octal):
  0400 - Owner read
  0200 - Owner write
  0100 - Owner execute
  0040 - Group read
  0020 - Group write
  0010 - Group execute
  0004 - Other read
  0002 - Other write
  0001 - Other execute

Special bits:
  04000 - Setuid
  02000 - Setgid
  01000 - Sticky bit
```

Common modes:
- `0755` - rwxr-xr-x (executable)
- `0644` - rw-r--r-- (regular file)
- `0755` - rwxr-xr-x (directory)
- `0777` - rwxrwxrwt (tmp with sticky)

## File Metadata

```rust
struct FileMetadata {
    inode: u64,           // Inode number
    file_type: FileType,  // Type of file
    mode: FileMode,       // Permissions
    uid: u32,             // Owner user ID
    gid: u32,             // Owner group ID
    size: u64,            // File size in bytes
    atime: u64,           // Last access time (Unix timestamp)
    mtime: u64,           // Last modification time
    ctime: u64,           // Last status change time
    links: u32,           // Hard link count
    block_size: u32,      // Preferred block size
    blocks: u64,          // Blocks allocated
}
```

## VFS API

### Filesystem Operations

```rust
// Mount a filesystem
fn mount(&mut self, fs_name: &str, mount_point: &str, 
         fs: Arc<dyn FileSystem>, root_inode: u64) -> Result<()>

// Open file
fn open(&mut self, path: &str, flags: u32) -> Result<u32>

// Close file descriptor
fn close(&mut self, fd: u32) -> Result<()>

// Read/write
fn read(&mut self, fd: u32, buf: &mut [u8]) -> Result<usize>
fn write(&mut self, fd: u32, buf: &[u8]) -> Result<usize>

// Seek
fn seek(&mut self, fd: u32, offset: i64, whence: u32) -> Result<u64>

// Directory operations
fn mkdir(&mut self, path: &str, mode: FileMode) -> Result<()>
fn rmdir(&mut self, path: &str) -> Result<()>
fn unlink(&mut self, path: &str) -> Result<()>

// Metadata
fn stat(&self, path: &str) -> Result<FileMetadata>
fn readdir(&self, path: &str) -> Result<Vec<DirEntry>>

// Symlinks
fn symlink(&mut self, target: &str, link_path: &str) -> Result<()>
fn readlink(&self, path: &str) -> Result<String>

// Working directory
fn chdir(&mut self, path: &str) -> Result<()>
fn getcwd(&self) -> &str

// Information
fn list_mounts(&self) -> Vec<&MountPoint>
fn list_open_files(&self) -> Vec<(u32, &FileHandle)>
```

## Shell Integration

### Filesystem Commands

| Command | Description |
|---------|-------------|
| `ls [path]` | List directory contents |
| `cd [path]` | Change directory |
| `pwd` | Print working directory |
| `cat <file>` | Display file contents |
| `mkdir <path>` | Create directory |
| `rmdir <path>` | Remove empty directory |
| `rm <path>` | Remove file |
| `touch <path>` | Create empty file |
| `cp <src> <dst>` | Copy file |
| `mv <src> <dst>` | Move/rename file |
| `stat <path>` | Show file metadata |
| `find <name> [path]` | Find file by name |

### Path Handling

- Absolute paths: `/home/user/file.txt`
- Relative paths: `../file.txt`, `./dir/file.txt`
- Home expansion: `~/file.txt` → `/home/user/file.txt` (future)
- Tab completion (future)

## Device Files

Device files in `/dev/` provide access to virtual devices:

| Device File | Major | Minor | Description |
|-------------|-------|-------|-------------|
| `/dev/null` | 1 | 3 | Discards all writes |
| `/dev/zero` | 1 | 5 | Infinite zeros |
| `/dev/random` | 1 | 8 | Random bytes |
| `/dev/urandom` | 1 | 9 | Non-blocking random |
| `/dev/tty` | 5 | 0 | Controlling terminal |
| `/dev/console` | 5 | 1 | System console |
| `/dev/kbd` | 10 | 0 | Keyboard input |
| `/dev/fb0` | 29 | 0 | Framebuffer |

Currently implemented as special inodes with custom read/write handlers.

## Virtual Filesystems

### /proc - Process Information

```
PID      TTY          TIME CMD
1        ?        00:00:00 init
2        ?        00:00:00 scheduler
3        ?        00:00:00 cpu
4        pts/0    00:00:00 shell
```

Files:
- `/proc/[pid]/status` - Process status
- `/proc/[pid]/mem` - Process memory
- `/proc/[pid]/fd/` - Open file descriptors
- `/proc/cpuinfo` - CPU information
- `/proc/meminfo` - Memory information
- `/proc/devices` - Device list
- `/proc/interrupts` - Interrupt counts

### /sys - System Information

```
/sys/
├── devices/        # Device tree
├── bus/            # Bus types
├── class/          # Device classes
├── block/          # Block devices
├── module/         # Kernel modules
└── kernel/         # Kernel parameters
```

## Filesystem Utilities (fs.rs)

### FSUtils

High-level operations built on VFS:

```rust
// Recursive operations
fn copy_tree(&self, src: &str, dst: &str) -> Result<()>
fn remove_tree(&self, path: &str) -> Result<()>

// Search
fn find_file(&self, name: &str, start_path: &str) -> Result<Option<String>>
fn find_by_pattern(&self, pattern: &str, path: &str) -> Result<Vec<String>>

// Information
fn get_size(&self, path: &str) -> Result<u64>
fn list_all_files(&self, path: &str) -> Result<Vec<String>>
fn get_tree(&self, path: &str, max_depth: usize) -> Result<String>

// Symlinks
fn create_symlink(&self, target: &str, link_path: &str) -> Result<()>
fn read_symlink(&self, path: &str) -> Result<String>
```

### TempFS

Temporary file management:

```rust
fn create_temp_file(&self, prefix: &str) -> Result<String>
fn create_temp_dir(&self, prefix: &str) -> Result<String>
fn cleanup(&self) -> Result<()>
```

## Performance

### In-Memory FS Characteristics
- **Read/Write**: O(1) for small files, O(n) for large
- **Lookup**: O(1) average (HashMap)
- **Readdir**: O(n) where n = directory entries
- **Memory**: Data stored in Vec<u8>, no compression

### Scalability
- Suitable for virtual system workloads
- Not designed for large-scale persistent storage
- Inode limit: 2^64 (practically unlimited)

## Future Enhancements

### Planned Filesystems
- **tarfs**: Read-only tar archive mounting
- **initramfs**: Boot filesystem
- **fatfs**: FAT32 for disk images
- **ext2fs**: ext2 compatibility
- **hostfs**: Host filesystem passthrough (controlled)

### VFS Features
- File locking (flock, fcntl)
- Extended attributes (xattr)
- Access control lists (ACLs)
- Filesystem quotas
- Snapshots
- Union mounts

### Performance
- Directory entry caching
- Read-ahead for sequential access
- Write-back caching
- Async I/O support