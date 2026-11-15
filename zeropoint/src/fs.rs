//! ZEROPOINT Filesystem Utilities
//! 
//! Additional filesystem helpers and abstractions.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::vfs::{VFS, FileType, FileMode, DirEntry, FileMetadata};

pub struct FSUtils {
    vfs: Arc<RwLock<VFS>>,
}

impl FSUtils {
    pub fn new(vfs: Arc<RwLock<VFS>>) -> Self {
        Self { vfs }
    }

    pub async fn find_file(&self, name: &str, start_path: &str) -> Result<Option<String>> {
        let vfs = self.vfs.read().await;
        self.find_file_recursive(&vfs, name, start_path).await
    }

    async fn find_file_recursive(&self, vfs: &VFS, name: &str, path: &str) -> Result<Option<String>> {
        let entries = vfs.readdir(path)?;
        for entry in entries {
            let full_path = if path == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", path, entry.name)
            };
            
            if entry.name == name {
                return Ok(Some(full_path));
            }
            
            if entry.file_type == FileType::Directory {
                if let Some(found) = Box::pin(self.find_file_recursive(vfs, name, &full_path)).await? {
                    return Ok(Some(found));
                }
            }
        }
        Ok(None)
    }

    pub async fn copy_tree(&self, src: &str, dst: &str) -> Result<()> {
        let vfs = self.vfs.read().await;
        let src_meta = vfs.stat(src)?;
        drop(vfs);
        
        if src_meta.file_type == FileType::Directory {
            self.copy_directory(src, dst).await
        } else {
            self.copy_file(src, dst).await
        }
    }

    async fn copy_directory(&self, src: &str, dst: &str) -> Result<()> {
        let mut vfs = self.vfs.write().await;
        
        // Create destination directory
        let dst_parent = Path::new(dst).parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/");
        let dst_name = Path::new(dst).file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid destination path"))?;
        
        let (fs, parent_inode, _) = vfs.resolve_path(dst_parent)?;
        fs.mkdir(parent_inode, dst_name, FileMode::ReadWriteExecute)?;
        
        drop(vfs);
        
        // Copy contents
        let vfs = self.vfs.read().await;
        let entries = vfs.readdir(src)?;
        drop(vfs);
        
        for entry in entries {
            let src_child = if src == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", src, entry.name)
            };
            let dst_child = if dst == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", dst, entry.name)
            };
            
            Box::pin(self.copy_tree(&src_child, &dst_child)).await?;
        }
        
        Ok(())
    }

    async fn copy_file(&self, src: &str, dst: &str) -> Result<()> {
        let mut vfs = self.vfs.write().await;
        let src_fd = vfs.open(src, 0)?;
        
        let dst_parent = Path::new(dst).parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/");
        let dst_name = Path::new(dst).file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid destination path"))?;
        
        let (fs, parent_inode, _) = vfs.resolve_path(dst_parent)?;
        fs.create(parent_inode, dst_name, FileType::Regular, FileMode::ReadWrite)?;
        
        let dst_fd = vfs.open(dst, 1)?;
        
        let mut buf = [0u8; 4096];
        loop {
            let n = vfs.read(src_fd, &mut buf)?;
            if n == 0 { break; }
            vfs.write(dst_fd, &buf[..n])?;
        }
        
        vfs.close(src_fd)?;
        vfs.close(dst_fd)?;
        Ok(())
    }

    pub async fn remove_tree(&self, path: &str) -> Result<()> {
        let vfs = self.vfs.read().await;
        let meta = vfs.stat(path)?;
        drop(vfs);
        
        if meta.file_type == FileType::Directory {
            self.remove_directory(path).await
        } else {
            let mut vfs = self.vfs.write().await;
            vfs.unlink(path)?;
            Ok(())
        }
    }

    async fn remove_directory(&self, path: &str) -> Result<()> {
        let vfs = self.vfs.read().await;
        let entries = vfs.readdir(path)?;
        drop(vfs);
        
        for entry in entries {
            let child_path = if path == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", path, entry.name)
            };
            Box::pin(self.remove_tree(&child_path)).await?;
        }
        
        let mut vfs = self.vfs.write().await;
        vfs.rmdir(path)?;
        Ok(())
    }

    pub async fn get_size(&self, path: &str) -> Result<u64> {
        let vfs = self.vfs.read().await;
        let meta = vfs.stat(path)?;
        drop(vfs);
        
        if meta.file_type == FileType::Directory {
            self.get_directory_size(path).await
        } else {
            Ok(meta.size)
        }
    }

    async fn get_directory_size(&self, path: &str) -> Result<u64> {
        let vfs = self.vfs.read().await;
        let entries = vfs.readdir(path)?;
        drop(vfs);
        
        let mut total = 0u64;
        for entry in entries {
            let child_path = if path == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", path, entry.name)
            };
            total += Box::pin(self.get_size(&child_path)).await?;
        }
        Ok(total)
    }

    pub async fn create_symlink(&self, target: &str, link_path: &str) -> Result<()> {
        let mut vfs = self.vfs.write().await;
        vfs.symlink(target, link_path)?;
        Ok(())
    }

    pub async fn read_symlink(&self, path: &str) -> Result<String> {
        let vfs = self.vfs.read().await;
        vfs.readlink(path)
    }

    pub async fn list_all_files(&self, path: &str) -> Result<Vec<String>> {
        let mut files = Vec::new();
        self.list_files_recursive(path, &mut files).await?;
        Ok(files)
    }

    async fn list_files_recursive(&self, path: &str, files: &mut Vec<String>) -> Result<()> {
        let vfs = self.vfs.read().await;
        let entries = vfs.readdir(path)?;
        drop(vfs);
        
        for entry in entries {
            let full_path = if path == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", path, entry.name)
            };
            files.push(full_path.clone());
            
            if entry.file_type == FileType::Directory {
                Box::pin(self.list_files_recursive(&full_path, files)).await?;
            }
        }
        Ok(())
    }

    pub async fn find_by_pattern(&self, pattern: &str, path: &str) -> Result<Vec<String>> {
        let mut matches = Vec::new();
        self.find_pattern_recursive(pattern, path, &mut matches).await?;
        Ok(matches)
    }

    async fn find_pattern_recursive(&self, pattern: &str, path: &str, matches: &mut Vec<String>) -> Result<()> {
        let vfs = self.vfs.read().await;
        let entries = vfs.readdir(path)?;
        drop(vfs);
        
        for entry in entries {
            let full_path = if path == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", path, entry.name)
            };
            
            if Self::match_pattern(&entry.name, pattern) {
                matches.push(full_path.clone());
            }
            
            if entry.file_type == FileType::Directory {
                Box::pin(self.find_pattern_recursive(pattern, &full_path, matches)).await?;
            }
        }
        Ok(())
    }

    fn match_pattern(name: &str, pattern: &str) -> bool {
        // Simple glob-style matching
        let pattern = pattern.replace("*", ".*").replace("?", ".");
        regex::Regex::new(&format!("^{}$", pattern))
            .map(|re| re.is_match(name))
            .unwrap_or(false)
    }

    pub async fn get_tree(&self, path: &str, max_depth: usize) -> Result<String> {
        let mut output = String::new();
        self.build_tree(path, "", max_depth, 0, &mut output).await?;
        Ok(output)
    }

    async fn build_tree(&self, path: &str, prefix: &str, max_depth: usize, depth: usize, output: &mut String) -> Result<()> {
        if depth > max_depth {
            return Ok(());
        }
        
        let vfs = self.vfs.read().await;
        let entries = vfs.readdir(path)?;
        drop(vfs);
        
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        
        for entry in entries {
            if entry.file_type == FileType::Directory {
                dirs.push(entry);
            } else {
                files.push(entry);
            }
        }
        
        dirs.sort_by_key(|e| e.name.clone());
        files.sort_by_key(|e| e.name.clone());
        
        let all = [dirs, files].concat();
        
        for (i, entry) in all.iter().enumerate() {
            let is_last = i == all.len() - 1;
            let connector = if is_last { "└── " } else { "├── " };
            let new_prefix = if is_last { "    " } else { "│   " };
            
            output.push_str(&format!("{}{}{}\n", prefix, connector, entry.name));
            
            if entry.file_type == FileType::Directory && depth < max_depth {
                let full_path = if path == "/" {
                    format!("/{}", entry.name)
                } else {
                    format!("{}/{}", path, entry.name)
                };
                let next_prefix = format!("{}{}", prefix, new_prefix);
                Box::pin(self.build_tree(&full_path, &next_prefix, max_depth, depth + 1, output)).await?;
            }
        }
        Ok(())
    }
}

pub struct TempFS {
    vfs: Arc<RwLock<VFS>>,
    temp_dir: String,
}

impl TempFS {
    pub fn new(vfs: Arc<RwLock<VFS>>) -> Result<Self> {
        let mut vfs_guard = vfs.blocking_write();
        let temp_dir = "/tmp/zeropoint_temp";
        let _ = vfs_guard.mkdir(temp_dir, FileMode::ReadWriteExecute);
        Ok(Self { vfs, temp_dir: temp_dir.to_string() })
    }

    pub async fn create_temp_file(&self, prefix: &str) -> Result<String> {
        let name = format!("{}_{}", prefix, uuid::Uuid::new_v4().simple());
        let path = format!("{}/{}", self.temp_dir, name);
        
        let mut vfs = self.vfs.write().await;
        let (fs, parent_inode, _) = vfs.resolve_path(&self.temp_dir)?;
        fs.create(parent_inode, &name, FileType::Regular, FileMode::ReadWrite)?;
        
        Ok(path)
    }

    pub async fn create_temp_dir(&self, prefix: &str) -> Result<String> {
        let name = format!("{}_{}", prefix, uuid::Uuid::new_v4().simple());
        let path = format!("{}/{}", self.temp_dir, name);
        
        let mut vfs = self.vfs.write().await;
        let (fs, parent_inode, _) = vfs.resolve_path(&self.temp_dir)?;
        fs.mkdir(parent_inode, &name, FileMode::ReadWriteExecute)?;
        
        Ok(path)
    }

    pub async fn cleanup(&self) -> Result<()> {
        let fs_utils = FSUtils::new(self.vfs.clone());
        fs_utils.remove_tree(&self.temp_dir).await?;
        
        let mut vfs = self.vfs.write().await;
        let (fs, parent_inode, _) = vfs.resolve_path("/tmp")?;
        fs.mkdir(parent_inode, "zeropoint_temp", FileMode::ReadWriteExecute)?;
        Ok(())
    }
}