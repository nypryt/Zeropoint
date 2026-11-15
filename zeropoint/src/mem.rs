use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

const PAGE_SIZE: usize = 4096;
const USER_SPACE_START: u64 = 0x0000_0000;
const USER_SPACE_END: u64 = 0x7FFF_FFFF;
const KERNEL_SPACE_START: u64 = 0x8000_0000;
const KERNEL_SPACE_END: u64 = 0xFFFF_FFFF;
const DEVICE_BASE: u64 = 0xFF00_0000;
const DEVICE_END: u64 = 0xFFFF_FFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagePermission {
    None = 0,
    Read = 1,
    Write = 2,
    Execute = 4,
    ReadWrite = 3,
    ReadExecute = 5,
    WriteExecute = 6,
    ReadWriteExecute = 7,
}

impl PagePermission {
    pub fn can_read(&self) -> bool {
        *self as u8 & 1 != 0
    }
    pub fn can_write(&self) -> bool {
        *self as u8 & 2 != 0
    }
    pub fn can_execute(&self) -> bool {
        *self as u8 & 4 != 0
    }
}

#[derive(Debug, Clone)]
pub struct PageTableEntry {
    pub physical_addr: u64,
    pub permissions: PagePermission,
    pub present: bool,
    pub user_accessible: bool,
    pub dirty: bool,
    pub accessed: bool,
}

impl PageTableEntry {
    pub fn new(physical_addr: u64, permissions: PagePermission, user_accessible: bool) -> Self {
        Self {
            physical_addr,
            permissions,
            present: true,
            user_accessible,
            dirty: false,
            accessed: false,
        }
    }

    pub fn empty() -> Self {
        Self {
            physical_addr: 0,
            permissions: PagePermission::None,
            present: false,
            user_accessible: false,
            dirty: false,
            accessed: false,
        }
    }
}

pub struct PageTable {
    entries: Vec<PageTableEntry>,
}

impl PageTable {
    pub fn new(num_pages: usize) -> Self {
        Self {
            entries: vec![PageTableEntry::empty(); num_pages],
        }
    }

    pub fn map(&mut self, virt_page: usize, entry: PageTableEntry) -> Result<()> {
        if virt_page >= self.entries.len() {
            return Err(anyhow!("Virtual page out of bounds"));
        }
        self.entries[virt_page] = entry;
        Ok(())
    }

    pub fn unmap(&mut self, virt_page: usize) -> Result<()> {
        if virt_page >= self.entries.len() {
            return Err(anyhow!("Virtual page out of bounds"));
        }
        self.entries[virt_page] = PageTableEntry::empty();
        Ok(())
    }

    pub fn get(&self, virt_page: usize) -> Option<&PageTableEntry> {
        self.entries.get(virt_page)
    }

    pub fn get_mut(&mut self, virt_page: usize) -> Option<&mut PageTableEntry> {
        self.entries.get_mut(virt_page)
    }
}

#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub base: u64,
    pub size: usize,
    pub permissions: PagePermission,
    pub name: String,
    pub is_mapped: bool,
    pub file_backed: Option<String>, // Path to file if memory-mapped
    pub offset: u64, // Offset in file
}

impl MemoryRegion {
    pub fn new(base: u64, size: usize, permissions: PagePermission, name: String) -> Self {
        Self {
            base,
            size,
            permissions,
            name,
            is_mapped: true,
            file_backed: None,
            offset: 0,
        }
    }

    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.base + self.size as u64
    }

    pub fn end(&self) -> u64 {
        self.base + self.size as u64
    }
}

pub struct MemoryManager {
    physical_memory: Vec<u8>,
    page_tables: HashMap<u32, PageTable>, // PID -> PageTable
    regions: HashMap<u32, Vec<MemoryRegion>>, // PID -> regions
    free_frames: Vec<u64>,
    frame_size: usize,
    total_frames: usize,
    used_frames: usize,
    next_pid: u32,
    kernel_page_table: PageTable,
}

impl MemoryManager {
    pub fn new(size: usize) -> Result<Self> {
        let total_frames = size / PAGE_SIZE;
        let mut free_frames = Vec::with_capacity(total_frames);
        for i in 0..total_frames {
            free_frames.push((i * PAGE_SIZE) as u64);
        }

        let num_pages = (KERNEL_SPACE_END - KERNEL_SPACE_START + 1) as usize / PAGE_SIZE;
        let kernel_page_table = PageTable::new(num_pages);

        Ok(Self {
            physical_memory: vec![0u8; size],
            page_tables: HashMap::new(),
            regions: HashMap::new(),
            free_frames,
            frame_size: PAGE_SIZE,
            total_frames,
            used_frames: 0,
            next_pid: 1,
            kernel_page_table,
        })
    }

    fn allocate_frame(&mut self) -> Result<u64> {
        if self.free_frames.is_empty() {
            return Err(anyhow!("Out of physical memory"));
        }
        let frame = self.free_frames.pop().unwrap();
        self.used_frames += 1;
        Ok(frame)
    }

    fn free_frame(&mut self, frame: u64) {
        self.free_frames.push(frame);
        self.used_frames -= 1;
    }

    pub fn create_process(&mut self) -> Result<u32> {
        let pid = self.next_pid;
        self.next_pid += 1;
        
        let num_pages = (USER_SPACE_END - USER_SPACE_START + 1) as usize / PAGE_SIZE;
        let page_table = PageTable::new(num_pages);
        self.page_tables.insert(pid, page_table);
        self.regions.insert(pid, Vec::new());
        
        // Map kernel space into process page table (shared)
        // In a real OS, this would be a separate kernel page table
        // For simplicity, we'll share the kernel mappings
        
        Ok(pid)
    }

    pub fn destroy_process(&mut self, pid: u32) -> Result<()> {
        if let Some(page_table) = self.page_tables.remove(&pid) {
            // Free all allocated frames
            for entry in page_table.entries {
                if entry.present {
                    self.free_frame(entry.physical_addr);
                }
            }
        }
        if let Some(regions) = self.regions.remove(&pid) {
            for region in regions {
                if region.is_mapped {
                    // Unmap pages
                    let start_page = (region.base - USER_SPACE_START) as usize / PAGE_SIZE;
                    let num_pages = (region.size + PAGE_SIZE - 1) / PAGE_SIZE;
                    if let Some(page_table) = self.page_tables.get_mut(&pid) {
                        for i in 0..num_pages {
                            let _ = page_table.unmap(start_page + i);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn map_memory(&mut self, pid: u32, virt_addr: u64, size: usize, permissions: PagePermission, name: String) -> Result<u64> {
        let page_table = self.page_tables.get_mut(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        let aligned_addr = virt_addr & !(PAGE_SIZE as u64 - 1);
        let aligned_size = ((size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
        let num_pages = aligned_size / PAGE_SIZE;

        // Check for overlapping regions
        if let Some(regions) = self.regions.get(&pid) {
            for region in regions {
                if region.is_mapped && 
                   !(aligned_addr + aligned_size as u64 <= region.base || 
                     aligned_addr >= region.end()) {
                    return Err(anyhow!("Memory region overlaps with existing region"));
                }
            }
        }

        let mut phys_addrs = Vec::new();
        for _ in 0..num_pages {
            phys_addrs.push(self.allocate_frame()?);
        }

        let start_page = (aligned_addr - USER_SPACE_START) as usize / PAGE_SIZE;
        for (i, &phys_addr) in phys_addrs.iter().enumerate() {
            let entry = PageTableEntry::new(
                phys_addr,
                permissions,
                true, // user accessible
            );
            page_table.map(start_page + i, entry)?;
        }

        let region = MemoryRegion::new(aligned_addr, aligned_size, permissions, name);
        self.regions.get_mut(&pid).unwrap().push(region);

        Ok(aligned_addr)
    }

    pub fn unmap_memory(&mut self, pid: u32, virt_addr: u64) -> Result<()> {
        let page_table = self.page_tables.get_mut(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        let regions = self.regions.get_mut(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;

        let region_idx = regions.iter().position(|r| r.base == virt_addr)
            .ok_or_else(|| anyhow!("Region not found"))?;
        
        let region = regions.remove(region_idx);
        let start_page = (region.base - USER_SPACE_START) as usize / PAGE_SIZE;
        let num_pages = (region.size + PAGE_SIZE - 1) / PAGE_SIZE;

        for i in 0..num_pages {
            if let Some(entry) = page_table.get_mut(start_page + i) {
                if entry.present {
                    self.free_frame(entry.physical_addr);
                    *entry = PageTableEntry::empty();
                }
            }
        }

        Ok(())
    }

    fn translate(&self, pid: u32, virt_addr: u64, write: bool) -> Result<u64> {
        if virt_addr >= KERNEL_SPACE_START {
            // Kernel space - direct mapping for simplicity
            return Ok(virt_addr - KERNEL_SPACE_START);
        }

        let page_table = self.page_tables.get(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        let virt_page = (virt_addr - USER_SPACE_START) as usize / PAGE_SIZE;
        let offset = virt_addr as usize % PAGE_SIZE;
        
        let entry = page_table.get(virt_page)
            .ok_or_else(|| anyhow!("Page not mapped"))?;
        
        if !entry.present {
            return Err(anyhow!("Page not present"));
        }
        
        if write && !entry.permissions.can_write() {
            return Err(anyhow!("Write permission denied"));
        }
        
        if !write && !entry.permissions.can_read() {
            return Err(anyhow!("Read permission denied"));
        }
        
        // Mark accessed
        if let Some(page_table_mut) = self.page_tables.get(&pid) {
            if let Some(entry_mut) = page_table_mut.get_mut(virt_page) {
                entry_mut.accessed = true;
                if write {
                    entry_mut.dirty = true;
                }
            }
        }
        
        Ok(entry.physical_addr + offset as u64)
    }

    pub async fn read8(&self, pid: u32, addr: u64) -> Result<u8> {
        let phys_addr = self.translate(pid, addr, false)?;
        if phys_addr >= self.physical_memory.len() as u64 {
            return Err(anyhow!("Physical address out of bounds"));
        }
        Ok(self.physical_memory[phys_addr as usize])
    }

    pub async fn read16(&self, pid: u32, addr: u64) -> Result<u16> {
        let low = self.read8(pid, addr).await? as u16;
        let high = self.read8(pid, addr + 1).await? as u16;
        Ok(low | (high << 8))
    }

    pub async fn read32(&self, pid: u32, addr: u64) -> Result<u32> {
        let b0 = self.read8(pid, addr).await? as u32;
        let b1 = self.read8(pid, addr + 1).await? as u32;
        let b2 = self.read8(pid, addr + 2).await? as u32;
        let b3 = self.read8(pid, addr + 3).await? as u32;
        Ok(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
    }

    pub async fn read64(&self, pid: u32, addr: u64) -> Result<u64> {
        let low = self.read32(pid, addr).await? as u64;
        let high = self.read32(pid, addr + 4).await? as u64;
        Ok(low | (high << 32))
    }

    pub async fn write8(&mut self, pid: u32, addr: u64, value: u8) -> Result<()> {
        let phys_addr = self.translate(pid, addr, true)?;
        if phys_addr >= self.physical_memory.len() as u64 {
            return Err(anyhow!("Physical address out of bounds"));
        }
        self.physical_memory[phys_addr as usize] = value;
        Ok(())
    }

    pub async fn write16(&mut self, pid: u32, addr: u64, value: u16) -> Result<()> {
        self.write8(pid, addr, value as u8).await?;
        self.write8(pid, addr + 1, (value >> 8) as u8).await?;
        Ok(())
    }

    pub async fn write32(&mut self, pid: u32, addr: u64, value: u32) -> Result<()> {
        self.write8(pid, addr, value as u8).await?;
        self.write8(pid, addr + 1, (value >> 8) as u8).await?;
        self.write8(pid, addr + 2, (value >> 16) as u8).await?;
        self.write8(pid, addr + 3, (value >> 24) as u8).await?;
        Ok(())
    }

    pub async fn write64(&mut self, pid: u32, addr: u64, value: u64) -> Result<()> {
        self.write32(pid, addr, value as u32).await?;
        self.write32(pid, addr + 4, (value >> 32) as u32).await?;
        Ok(())
    }

    pub async fn read_bytes(&self, pid: u32, addr: u64, buf: &mut [u8]) -> Result<()> {
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = self.read8(pid, addr + i as u64).await?;
        }
        Ok(())
    }

    pub async fn write_bytes(&mut self, pid: u32, addr: u64, buf: &[u8]) -> Result<()> {
        for (i, &byte) in buf.iter().enumerate() {
            self.write8(pid, addr + i as u64, byte).await?;
        }
        Ok(())
    }

    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            total_memory: self.total_frames * self.frame_size,
            used_memory: self.used_frames * self.frame_size,
            free_memory: self.free_frames.len() * self.frame_size,
            total_frames: self.total_frames,
            used_frames: self.used_frames,
            free_frames: self.free_frames.len(),
            num_processes: self.page_tables.len(),
        }
    }

    pub fn get_regions(&self, pid: u32) -> Option<Vec<MemoryRegion>> {
        self.regions.get(&pid).cloned()
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_memory: usize,
    pub used_memory: usize,
    pub free_memory: usize,
    pub total_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
    pub num_processes: usize,
}