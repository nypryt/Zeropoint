use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
    CharacterDevice,
    BlockDevice,
    Fifo,
    Socket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    Read = 0o400,
    Write = 0o200,
    Execute = 0o100,
    ReadWrite = 0o600,
    ReadExecute = 0o500,
    WriteExecute = 0o300,
    ReadWriteExecute = 0o700,
}

impl FileMode {
    pub fn can_read(&self) -> bool { *self as u32 & 0o400 != 0 }
    pub fn can_write(&self) -> bool { *self as u32 & 0o200 != 0 }
    pub fn can_execute(&self) -> bool { *self as u32 & 0o100 != 0 }
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub inode: u64,
    pub file_type: FileType,
    pub mode: FileMode,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub links: u32,
    pub block_size: u32,
    pub blocks: u64,
}

impl FileMetadata {
    pub fn new(file_type: FileType, mode: FileMode) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Self {
            inode: 0, // Will be set by VFS
            file_type,
            mode,
            uid: 0,
            gid: 0,
            size: 0,
            atime: now,
            mtime: now,
            ctime: now,
            links: 1,
            block_size: 4096,
            blocks: 0,
        }
    }
}

pub trait FileSystem: Send + Sync {
    fn name(&self) -> &str;
    fn read(&self, inode: u64, offset: u64, buf: &mut [u8]) -> Result<usize>;
    fn write(&self, inode: u64, offset: u64, buf: &[u8]) -> Result<usize>;
    fn create(&self, parent: u64, name: &str, file_type: FileType, mode: FileMode) -> Result<u64>;
    fn unlink(&self, parent: u64, name: &str) -> Result<()>;
    fn mkdir(&self, parent: u64, name: &str, mode: FileMode) -> Result<u64>;
    fn rmdir(&self, parent: u64, name: &str) -> Result<()>;
    fn lookup(&self, parent: u64, name: &str) -> Result<Option<u64>>;
    fn readdir(&self, inode: u64, offset: u64, entries: &mut Vec<DirEntry>) -> Result<usize>;
    fn stat(&self, inode: u64) -> Result<FileMetadata>;
    fn setattr(&self, inode: u64, metadata: &FileMetadata) -> Result<()>;
    fn symlink(&self, parent: u64, name: &str, target: &str) -> Result<u64>;
    fn readlink(&self, inode: u64) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub inode: u64,
    pub name: String,
    pub file_type: FileType,
}

pub struct InMemoryFileSystem {
    files: RwLock<HashMap<u64, InMemoryFile>>,
    next_inode: RwLock<u64>,
}

struct InMemoryFile {
    metadata: FileMetadata,
    data: Vec<u8>,
    children: HashMap<String, u64>, // For directories
    symlink_target: Option<String>,
}

impl InMemoryFileSystem {
    pub fn new() -> Self {
        let fs = Self {
            files: RwLock::new(HashMap::new()),
            next_inode: RwLock::new(1),
        };
        
        // Create root directory
        let root_inode = fs.allocate_inode();
        let mut files = fs.files.write();
        files.insert(root_inode, InMemoryFile {
            metadata: FileMetadata::new(FileType::Directory, FileMode::ReadWriteExecute),
            data: Vec::new(),
            children: HashMap::new(),
            symlink_target: None,
        });
        files.get_mut(&root_inode).unwrap().metadata.inode = root_inode;
        
        fs
    }

    fn allocate_inode(&self) -> u64 {
        let mut next = self.next_inode.write();
        let inode = *next;
        *next += 1;
        inode
    }
}

impl FileSystem for InMemoryFileSystem {
    fn name(&self) -> &str { "inmem" }

    fn read(&self, inode: u64, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let files = self.files.read();
        let file = files.get(&inode).ok_or_else(|| anyhow!("File not found"))?;
        
        if file.metadata.file_type != FileType::Regular {
            return Err(anyhow!("Not a regular file"));
        }
        
        let start = offset as usize;
        let end = (offset + buf.len() as u64).min(file.data.len() as u64) as usize;
        if start >= file.data.len() {
            return Ok(0);
        }
        
        let len = end - start;
        buf[..len].copy_from_slice(&file.data[start..end]);
        Ok(len)
    }

    fn write(&self, inode: u64, offset: u64, buf: &[u8]) -> Result<usize> {
        let mut files = self.files.write();
        let file = files.get_mut(&inode).ok_or_else(|| anyhow!("File not found"))?;
        
        if file.metadata.file_type != FileType::Regular {
            return Err(anyhow!("Not a regular file"));
        }
        
        let end = offset as usize + buf.len();
        if end > file.data.len() {
            file.data.resize(end, 0);
        }
        
        file.data[offset as usize..end].copy_from_slice(buf);
        file.metadata.size = file.metadata.size.max(end as u64);
        file.metadata.mtime = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        file.metadata.ctime = file.metadata.mtime;
        
        Ok(buf.len())
    }

    fn create(&self, parent: u64, name: &str, file_type: FileType, mode: FileMode) -> Result<u64> {
        let mut files = self.files.write();
        let parent_file = files.get_mut(&parent).ok_or_else(|| anyhow!("Parent not found"))?;
        
        if parent_file.metadata.file_type != FileType::Directory {
            return Err(anyhow!("Not a directory"));
        }
        
        if parent_file.children.contains_key(name) {
            return Err(anyhow!("File already exists"));
        }
        
        let inode = self.allocate_inode();
        let new_file = InMemoryFile {
            metadata: FileMetadata::new(file_type, mode),
            data: Vec::new(),
            children: HashMap::new(),
            symlink_target: None,
        };
        new_file.metadata.inode = inode;
        
        parent_file.children.insert(name.to_string(), inode);
        files.insert(inode, new_file);
        
        Ok(inode)
    }

    fn unlink(&self, parent: u64, name: &str) -> Result<()> {
        let mut files = self.files.write();
        let parent_file = files.get_mut(&parent).ok_or_else(|| anyhow!("Parent not found"))?;
        
        if parent_file.metadata.file_type != FileType::Directory {
            return Err(anyhow!("Not a directory"));
        }
        
        let inode = parent_file.children.remove(name).ok_or_else(|| anyhow!("File not found"))?;
        files.remove(&inode);
        
        Ok(())
    }

    fn mkdir(&self, parent: u64, name: &str, mode: FileMode) -> Result<u64> {
        self.create(parent, name, FileType::Directory, mode)
    }

    fn rmdir(&self, parent: u64, name: &str) -> Result<()> {
        let mut files = self.files.write();
        let parent_file = files.get_mut(&parent).ok_or_else(|| anyhow!("Parent not found"))?;
        
        if parent_file.metadata.file_type != FileType::Directory {
            return Err(anyhow!("Not a directory"));
        }
        
        let inode = parent_file.children.get(name).ok_or_else(|| anyhow!("Directory not found"))?;
        let dir_file = files.get(inode).ok_or_else(|| anyhow!("Directory not found"))?;
        
        if dir_file.metadata.file_type != FileType::Directory {
            return Err(anyhow!("Not a directory"));
        }
        
        if !dir_file.children.is_empty() {
            return Err(anyhow!("Directory not empty"));
        }
        
        parent_file.children.remove(name);
        files.remove(inode);
        
        Ok(())
    }

    fn lookup(&self, parent: u64, name: &str) -> Result<Option<u64>> {
        let files = self.files.read();
        let parent_file = files.get(&parent).ok_or_else(|| anyhow!("Parent not found"))?;
        
        if parent_file.metadata.file_type != FileType::Directory {
            return Err(anyhow!("Not a directory"));
        }
        
        Ok(parent_file.children.get(name).copied())
    }

    fn readdir(&self, inode: u64, offset: u64, entries: &mut Vec<DirEntry>) -> Result<usize> {
        let files = self.files.read();
        let file = files.get(&inode).ok_or_else(|| anyhow!("Directory not found"))?;
        
        if file.metadata.file_type != FileType::Directory {
            return Err(anyhow!("Not a directory"));
        }
        
        let mut count = 0;
        for (name, &child_inode) in &file.children {
            if count >= offset as usize {
                if let Some(child) = files.get(&child_inode) {
                    entries.push(DirEntry {
                        inode: child_inode,
                        name: name.clone(),
                        file_type: child.metadata.file_type,
                    });
                }
            }
            count += 1;
        }
        
        Ok(count.saturating_sub(offset as usize))
    }

    fn stat(&self, inode: u64) -> Result<FileMetadata> {
        let files = self.files.read();
        let file = files.get(&inode).ok_or_else(|| anyhow!("File not found"))?;
        Ok(file.metadata.clone())
    }

    fn setattr(&self, inode: u64, metadata: &FileMetadata) -> Result<()> {
        let mut files = self.files.write();
        let file = files.get_mut(&inode).ok_or_else(|| anyhow!("File not found"))?;
        file.metadata = metadata.clone();
        Ok(())
    }

    fn symlink(&self, parent: u64, name: &str, target: &str) -> Result<u64> {
        let mut files = self.files.write();
        let parent_file = files.get_mut(&parent).ok_or_else(|| anyhow!("Parent not found"))?;
        
        if parent_file.metadata.file_type != FileType::Directory {
            return Err(anyhow!("Not a directory"));
        }
        
        if parent_file.children.contains_key(name) {
            return Err(anyhow!("File already exists"));
        }
        
        let inode = self.allocate_inode();
        let new_file = InMemoryFile {
            metadata: FileMetadata::new(FileType::Symlink, FileMode::ReadWriteExecute),
            data: Vec::new(),
            children: HashMap::new(),
            symlink_target: Some(target.to_string()),
        };
        new_file.metadata.inode = inode;
        
        parent_file.children.insert(name.to_string(), inode);
        files.insert(inode, new_file);
        
        Ok(inode)
    }

    fn readlink(&self, inode: u64) -> Result<String> {
        let files = self.files.read();
        let file = files.get(&inode).ok_or_else(|| anyhow!("File not found"))?;
        
        if file.metadata.file_type != FileType::Symlink {
            return Err(anyhow!("Not a symlink"));
        }
        
        file.symlink_target.clone().ok_or_else(|| anyhow!("Symlink target not found"))
    }
}

pub struct VFS {
    filesystems: HashMap<String, Arc<dyn FileSystem>>,
    mounts: HashMap<String, MountPoint>,
    root_fs: String,
    root_inode: u64,
    fd_table: HashMap<u32, FileHandle>,
    next_fd: u32,
    cwd: String,
}

struct MountPoint {
    fs: Arc<dyn FileSystem>,
    mount_point: String,
    root_inode: u64,
}

struct FileHandle {
    fs: Arc<dyn FileSystem>,
    inode: u64,
    offset: u64,
    flags: u32,
    path: String,
}

impl VFS {
    pub fn new() -> Result<Self> {
        let mut vfs = Self {
            filesystems: HashMap::new(),
            mounts: HashMap::new(),
            root_fs: String::new(),
            root_inode: 0,
            fd_table: HashMap::new(),
            next_fd: 3, // 0=stdin, 1=stdout, 2=stderr
            cwd: "/".to_string(),
        };
        
        // Register in-memory filesystem
        let inmem_fs = Arc::new(InMemoryFileSystem::new());
        let root_inode = 1; // Root inode from InMemoryFileSystem
        vfs.register_filesystem("inmem", inmem_fs.clone())?;
        vfs.mount("inmem", "/", inmem_fs, root_inode)?;
        vfs.root_fs = "inmem".to_string();
        vfs.root_inode = root_inode;
        
        // Create standard directories
        vfs.mkdir("/bin", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/etc", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/home", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/tmp", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/dev", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/proc", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/sys", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/mnt", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/root", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/usr", FileMode::ReadWriteExecute)?;
        vfs.mkdir("/var", FileMode::ReadWriteExecute)?;
        
        Ok(vfs)
    }

    fn register_filesystem(&mut self, name: &str, fs: Arc<dyn FileSystem>) -> Result<()> {
        if self.filesystems.contains_key(name) {
            return Err(anyhow!("Filesystem already registered"));
        }
        self.filesystems.insert(name.to_string(), fs);
        Ok(())
    }

    fn mount(&mut self, fs_name: &str, mount_point: &str, fs: Arc<dyn FileSystem>, root_inode: u64) -> Result<()> {
        if self.mounts.contains_key(mount_point) {
            return Err(anyhow!("Mount point already in use"));
        }
        
        let mp = MountPoint {
            fs: fs.clone(),
            mount_point: mount_point.to_string(),
            root_inode,
        };
        
        self.mounts.insert(mount_point.to_string(), mp);
        Ok(())
    }

    fn resolve_path(&self, path: &str) -> Result<(Arc<dyn FileSystem>, u64, String)> {
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", self.cwd, path).replace("//", "/")
        };
        
        // Find longest matching mount point
        let mut best_match = "/";
        let mut best_fs = self.filesystems.get(&self.root_fs).unwrap().clone();
        let mut best_inode = self.root_inode;
        
        for (mp_path, mp) in &self.mounts {
            if path.starts_with(mp_path) && mp_path.len() > best_match.len() {
                best_match = mp_path;
                best_fs = mp.fs.clone();
                best_inode = mp.root_inode;
            }
        }
        
        let relative_path = path.strip_prefix(best_match).unwrap_or(&path);
        let relative_path = relative_path.trim_start_matches('/');
        
        if relative_path.is_empty() {
            return Ok((best_fs, best_inode, "".to_string()));
        }
        
        let components: Vec<&str> = relative_path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_inode = best_inode;
        
        for (i, component) in components.iter().enumerate() {
            let inode = best_fs.lookup(current_inode, component)?
                .ok_or_else(|| anyhow!("Path not found: {}", component))?;
            
            if i == components.len() - 1 {
                return Ok((best_fs, inode, component.to_string()));
            }
            
            // Check if it's a directory
            let meta = best_fs.stat(inode)?;
            if meta.file_type != FileType::Directory {
                return Err(anyhow!("Not a directory: {}", component));
            }
            
            current_inode = inode;
        }
        
        Ok((best_fs, current_inode, "".to_string()))
    }

    pub fn open(&mut self, path: &str, flags: u32) -> Result<u32> {
        let (fs, inode, name) = self.resolve_path(path)?;
        let fd = self.next_fd;
        self.next_fd += 1;
        
        let handle = FileHandle {
            fs,
            inode,
            offset: 0,
            flags,
            path: path.to_string(),
        };
        
        self.fd_table.insert(fd, handle);
        Ok(fd)
    }

    pub fn close(&mut self, fd: u32) -> Result<()> {
        self.fd_table.remove(&fd).ok_or_else(|| anyhow!("Bad file descriptor"))?;
        Ok(())
    }

    pub fn read(&mut self, fd: u32, buf: &mut [u8]) -> Result<usize> {
        let handle = self.fd_table.get_mut(&fd).ok_or_else(|| anyhow!("Bad file descriptor"))?;
        let n = handle.fs.read(handle.inode, handle.offset, buf)?;
        handle.offset += n as u64;
        Ok(n)
    }

    pub fn write(&mut self, fd: u32, buf: &[u8]) -> Result<usize> {
        let handle = self.fd_table.get_mut(&fd).ok_or_else(|| anyhow!("Bad file descriptor"))?;
        let n = handle.fs.write(handle.inode, handle.offset, buf)?;
        handle.offset += n as u64;
        Ok(n)
    }

    pub fn seek(&mut self, fd: u32, offset: i64, whence: u32) -> Result<u64> {
        let handle = self.fd_table.get_mut(&fd).ok_or_else(|| anyhow!("Bad file descriptor"))?;
        let meta = handle.fs.stat(handle.inode)?;
        
        let new_offset = match whence {
            0 => offset as u64, // SEEK_SET
            1 => (handle.offset as i64 + offset) as u64, // SEEK_CUR
            2 => (meta.size as i64 + offset) as u64, // SEEK_END
            _ => return Err(anyhow!("Invalid whence")),
        };
        
        handle.offset = new_offset.min(meta.size);
        Ok(handle.offset)
    }

    pub fn mkdir(&mut self, path: &str, mode: FileMode) -> Result<()> {
        let parent_path = std::path::Path::new(path).parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/");
        let name = std::path::Path::new(path).file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid path"))?;
        
        let (fs, parent_inode, _) = self.resolve_path(parent_path)?;
        fs.mkdir(parent_inode, name, mode)?;
        Ok(())
    }

    pub fn rmdir(&mut self, path: &str) -> Result<()> {
        let parent_path = std::path::Path::new(path).parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/");
        let name = std::path::Path::new(path).file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid path"))?;
        
        let (fs, parent_inode, _) = self.resolve_path(parent_path)?;
        fs.rmdir(parent_inode, name)?;
        Ok(())
    }

    pub fn unlink(&mut self, path: &str) -> Result<()> {
        let parent_path = std::path::Path::new(path).parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/");
        let name = std::path::Path::new(path).file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid path"))?;
        
        let (fs, parent_inode, _) = self.resolve_path(parent_path)?;
        fs.unlink(parent_inode, name)?;
        Ok(())
    }

    pub fn stat(&self, path: &str) -> Result<FileMetadata> {
        let (fs, inode, _) = self.resolve_path(path)?;
        fs.stat(inode)
    }

    pub fn readdir(&self, path: &str) -> Result<Vec<DirEntry>> {
        let (fs, inode, _) = self.resolve_path(path)?;
        let mut entries = Vec::new();
        fs.readdir(inode, 0, &mut entries)?;
        Ok(entries)
    }

    pub fn symlink(&mut self, target: &str, link_path: &str) -> Result<()> {
        let parent_path = std::path::Path::new(link_path).parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/");
        let name = std::path::Path::new(link_path).file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid path"))?;
        
        let (fs, parent_inode, _) = self.resolve_path(parent_path)?;
        fs.symlink(parent_inode, name, target)?;
        Ok(())
    }

    pub fn readlink(&self, path: &str) -> Result<String> {
        let (fs, inode, _) = self.resolve_path(path)?;
        fs.readlink(inode)
    }

    pub fn chdir(&mut self, path: &str) -> Result<()> {
        let (fs, inode, _) = self.resolve_path(path)?;
        let meta = fs.stat(inode)?;
        if meta.file_type != FileType::Directory {
            return Err(anyhow!("Not a directory"));
        }
        self.cwd = path.to_string();
        Ok(())
    }

    pub fn getcwd(&self) -> &str {
        &self.cwd
    }

    pub fn list_mounts(&self) -> Vec<&MountPoint> {
        self.mounts.values().collect()
    }

    pub fn list_open_files(&self) -> Vec<(u32, &FileHandle)> {
        self.fd_table.iter().map(|(fd, h)| (*fd, h)).collect()
    }
}