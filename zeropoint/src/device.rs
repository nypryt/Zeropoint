//! ZEROPOINT Device Emulation
//! 
//! Emulates virtual devices: keyboard, display, disk, network, timer, RTC, etc.

use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock as AsyncRwLock;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    Keyboard,
    Display,
    Disk,
    Network,
    Timer,
    RTC,
    Serial,
    Parallel,
    Audio,
    Mouse,
    GPIO,
    InterruptController,
    DMA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Uninitialized,
    Ready,
    Busy,
    Error,
    Offline,
}

#[derive(Debug, Clone)]
pub struct DeviceRegisters {
    pub control: u32,
    pub status: u32,
    pub data: u32,
    pub address: u32,
    pub interrupt_enable: u32,
    pub interrupt_status: u32,
    pub dma_address: u32,
    pub dma_count: u32,
    pub reserved: [u32; 8],
}

impl Default for DeviceRegisters {
    fn default() -> Self {
        Self {
            control: 0,
            status: 0,
            data: 0,
            address: 0,
            interrupt_enable: 0,
            interrupt_status: 0,
            dma_address: 0,
            dma_count: 0,
            reserved: [0; 8],
        }
    }
}

pub trait Device: Send + Sync {
    fn device_type(&self) -> DeviceType;
    fn name(&self) -> &str;
    fn base_address(&self) -> u64;
    fn size(&self) -> u64;
    fn state(&self) -> DeviceState;
    fn set_state(&mut self, state: DeviceState);
    fn registers(&self) -> &DeviceRegisters;
    fn registers_mut(&mut self) -> &mut DeviceRegisters;
    fn read(&mut self, offset: u64) -> Result<u32>;
    fn write(&mut self, offset: u64, value: u32) -> Result<()>;
    fn reset(&mut self) -> Result<()>;
    fn tick(&mut self, delta: Duration) -> Result<()>;
    fn interrupt_pending(&self) -> bool;
    fn handle_interrupt(&mut self) -> Result<()>;
}

pub struct KeyboardDevice {
    registers: DeviceRegisters,
    state: DeviceState,
    key_buffer: VecDeque<u32>,
    led_state: u8,
    repeat_delay: Duration,
    repeat_rate: Duration,
    last_key: Option<u32>,
    last_key_time: Option<Instant>,
}

impl KeyboardDevice {
    pub fn new(base_address: u64) -> Self {
        Self {
            registers: DeviceRegisters::default(),
            state: DeviceState::Ready,
            key_buffer: VecDeque::with_capacity(256),
            led_state: 0,
            repeat_delay: Duration::from_millis(500),
            repeat_rate: Duration::from_millis(50),
            last_key: None,
            last_key_time: None,
        }
    }

    pub fn push_key(&mut self, scancode: u32, pressed: bool) {
        let key_data = (scancode & 0xFF) | if pressed { 0x8000 } else { 0 };
        self.key_buffer.push_back(key_data);
        
        if self.key_buffer.len() > 256 {
            self.key_buffer.pop_front();
        }
        
        // Set interrupt status
        self.registers.interrupt_status |= 0x1;
        self.registers.status |= 0x1; // Data ready
    }

    pub fn set_leds(&mut self, leds: u8) {
        self.led_state = leds;
        self.registers.data = leds as u32;
    }
}

impl Device for KeyboardDevice {
    fn device_type(&self) -> DeviceType { DeviceType::Keyboard }
    fn name(&self) -> &str { "PS/2 Keyboard" }
    fn base_address(&self) -> u64 { 0xFF00_0000 }
    fn size(&self) -> u64 { 0x1000 }
    fn state(&self) -> DeviceState { self.state }
    fn set_state(&mut self, state: DeviceState) { self.state = state; }
    fn registers(&self) -> &DeviceRegisters { &self.registers }
    fn registers_mut(&mut self) -> &mut DeviceRegisters { &mut self.registers }

    fn read(&mut self, offset: u64) -> Result<u32> {
        match offset {
            0x00 => Ok(self.registers.data), // Data port
            0x04 => Ok(self.registers.status), // Status port
            0x08 => Ok(self.registers.control), // Control port
            0x0C => Ok(self.registers.interrupt_status), // Interrupt status
            0x10 => Ok(self.registers.interrupt_enable), // Interrupt enable
            _ => Ok(0),
        }
    }

    fn write(&mut self, offset: u64, value: u32) -> Result<()> {
        match offset {
            0x00 => { // Data port (LED control)
                self.set_leds(value as u8);
            }
            0x04 => { // Command port
                match value {
                    0xED => { // Set LEDs
                        // Next byte will be LED data
                    }
                    0xEE => { // Echo
                        self.push_key(0xEE, true);
                    }
                    0xF0 => { // Set scan code set
                        // Next byte selects set
                    }
                    0xF4 => { // Enable scanning
                        self.registers.control |= 0x1;
                    }
                    0xF5 => { // Disable scanning
                        self.registers.control &= !0x1;
                    }
                    0xFF => { // Reset
                        self.reset()?;
                    }
                    _ => {}
                }
            }
            0x08 => { // Control register
                self.registers.control = value;
            }
            0x0C => { // Interrupt status (write to clear)
                self.registers.interrupt_status &= !value;
            }
            0x10 => { // Interrupt enable
                self.registers.interrupt_enable = value;
            }
            _ => {}
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.key_buffer.clear();
        self.registers = DeviceRegisters::default();
        self.registers.status = 0x01; // Self-test passed
        self.state = DeviceState::Ready;
        Ok(())
    }

    fn tick(&mut self, delta: Duration) -> Result<()> {
        // Handle key repeat
        if let (Some(last_key), Some(last_time)) = (self.last_key, self.last_key_time) {
            let elapsed = last_time.elapsed();
            if elapsed > self.repeat_delay && elapsed > self.repeat_rate {
                self.push_key(last_key, true);
                self.last_key_time = Some(Instant::now());
            }
        }
        Ok(())
    }

    fn interrupt_pending(&self) -> bool {
        (self.registers.interrupt_status & self.registers.interrupt_enable) != 0
    }

    fn handle_interrupt(&mut self) -> Result<()> {
        self.registers.interrupt_status &= !0x1;
        self.registers.status &= !0x1;
        Ok(())
    }
}

pub struct DisplayDevice {
    registers: DeviceRegisters,
    state: DeviceState,
    width: u32,
    height: u32,
    bpp: u32,
    framebuffer: Vec<u8>,
    cursor_x: u32,
    cursor_y: u32,
    cursor_visible: bool,
    palette: Vec<u32>,
    double_buffer: bool,
    front_buffer: usize,
    dirty_rects: Vec<(u32, u32, u32, u32)>,
}

impl DisplayDevice {
    pub fn new(base_address: u64, width: u32, height: u32) -> Self {
        let fb_size = (width * height * 4) as usize; // 32-bit color
        Self {
            registers: DeviceRegisters::default(),
            state: DeviceState::Ready,
            width,
            height,
            bpp: 32,
            framebuffer: vec![0; fb_size * 2], // Double buffer
            cursor_x: 0,
            cursor_y: 0,
            cursor_visible: true,
            palette: (0..256).map(|i| (i << 16) | (i << 8) | i).collect(),
            double_buffer: true,
            front_buffer: 0,
            dirty_rects: Vec::new(),
        }
    }

    pub fn get_framebuffer(&self) -> &[u8] {
        let offset = self.front_buffer * (self.width * self.height * 4) as usize;
        &self.framebuffer[offset..offset + (self.width * self.height * 4) as usize]
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            let back_buffer = 1 - self.front_buffer;
            let offset = back_buffer * (self.width * self.height * 4) as usize;
            let pixel_offset = (y * self.width + x) * 4;
            let idx = offset + pixel_offset as usize;
            if idx + 3 < self.framebuffer.len() {
                self.framebuffer[idx] = color as u8;
                self.framebuffer[idx + 1] = (color >> 8) as u8;
                self.framebuffer[idx + 2] = (color >> 16) as u8;
                self.framebuffer[idx + 3] = (color >> 24) as u8;
            }
            self.dirty_rects.push((x, y, 1, 1));
        }
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    pub fn swap_buffers(&mut self) {
        self.front_buffer = 1 - self.front_buffer;
        self.registers.interrupt_status |= 0x1; // VSync interrupt
        self.dirty_rects.clear();
    }

    pub fn write_text(&mut self, x: u32, y: u32, text: &str, fg: u32, bg: u32) {
        // Simple 8x16 font rendering (placeholder)
        let mut cx = x;
        for ch in text.chars() {
            if ch == '\n' {
                cx = x;
                continue;
            }
            // Draw character (simplified)
            self.fill_rect(cx, y, 8, 16, bg);
            // In real implementation, would use font bitmap
            cx += 8;
            if cx >= self.width {
                break;
            }
        }
    }
}

impl Device for DisplayDevice {
    fn device_type(&self) -> DeviceType { DeviceType::Display }
    fn name(&self) -> &str { "VGA Display" }
    fn base_address(&self) -> u64 { 0xFF01_0000 }
    fn size(&self) -> u64 { 0x10000 }
    fn state(&self) -> DeviceState { self.state }
    fn set_state(&mut self, state: DeviceState) { self.state = state; }
    fn registers(&self) -> &DeviceRegisters { &self.registers }
    fn registers_mut(&mut self) -> &mut DeviceRegisters { &mut self.registers }

    fn read(&mut self, offset: u64) -> Result<u32> {
        match offset {
            0x00 => Ok(self.width),
            0x04 => Ok(self.height),
            0x08 => Ok(self.bpp),
            0x0C => Ok(self.registers.address), // Framebuffer address
            0x10 => Ok(self.registers.control),
            0x14 => Ok(self.registers.status),
            0x18 => Ok(self.registers.interrupt_status),
            0x1C => Ok(self.registers.interrupt_enable),
            0x20 => Ok(self.cursor_x),
            0x24 => Ok(self.cursor_y),
            _ => Ok(0),
        }
    }

    fn write(&mut self, offset: u64, value: u32) -> Result<()> {
        match offset {
            0x0C => { // Framebuffer address
                self.registers.address = value;
            }
            0x10 => { // Control
                self.registers.control = value;
                if value & 0x1 != 0 {
                    self.swap_buffers();
                }
            }
            0x18 => { // Interrupt status (clear)
                self.registers.interrupt_status &= !value;
            }
            0x1C => { // Interrupt enable
                self.registers.interrupt_enable = value;
            }
            0x20 => { // Cursor X
                self.cursor_x = value.min(self.width - 1);
            }
            0x24 => { // Cursor Y
                self.cursor_y = value.min(self.height - 1);
            }
            _ => {}
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.framebuffer.fill(0);
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.registers = DeviceRegisters::default();
        self.registers.address = 0xFF80_0000; // Framebuffer address
        self.registers.status = 0x1;
        self.state = DeviceState::Ready;
        Ok(())
    }

    fn tick(&mut self, delta: Duration) -> Result<()> {
        // Simulate VSync at 60Hz
        static mut LAST_VSYNC: Option<Instant> = None;
        unsafe {
            if LAST_VSYNC.is_none() {
                LAST_VSYNC = Some(Instant::now());
            }
            if LAST_VSYNC.unwrap().elapsed() >= Duration::from_millis(16) {
                self.registers.interrupt_status |= 0x1;
                LAST_VSYNC = Some(Instant::now());
            }
        }
        Ok(())
    }

    fn interrupt_pending(&self) -> bool {
        (self.registers.interrupt_status & self.registers.interrupt_enable) != 0
    }

    fn handle_interrupt(&mut self) -> Result<()> {
        self.registers.interrupt_status &= !0x1;
        Ok(())
    }
}

pub struct DiskDevice {
    registers: DeviceRegisters,
    state: DeviceState,
    sectors: u64,
    sector_size: u32,
    data: Vec<u8>,
    read_only: bool,
    model: String,
    serial: String,
    firmware: String,
    dma_active: bool,
    dma_direction: bool, // true = read from disk, false = write to disk
}

impl DiskDevice {
    pub fn new(base_address: u64, size_mb: u64) -> Self {
        let sectors = (size_mb * 1024 * 1024) / 512;
        let data = vec![0u8; sectors as usize * 512];
        
        Self {
            registers: DeviceRegisters::default(),
            state: DeviceState::Ready,
            sectors,
            sector_size: 512,
            data,
            read_only: false,
            model: "ZEROPOINT Virtual Disk".to_string(),
            serial: "ZP001".to_string(),
            firmware: "1.0".to_string(),
            dma_active: false,
            dma_direction: true,
        }
    }

    pub fn read_sector(&self, sector: u64, buf: &mut [u8]) -> Result<()> {
        if sector >= self.sectors {
            return Err(anyhow!("Sector out of bounds"));
        }
        let offset = sector as usize * self.sector_size as usize;
        let end = offset + self.sector_size as usize;
        buf.copy_from_slice(&self.data[offset..end]);
        Ok(())
    }

    pub fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<()> {
        if self.read_only {
            return Err(anyhow!("Disk is read-only"));
        }
        if sector >= self.sectors {
            return Err(anyhow!("Sector out of bounds"));
        }
        if buf.len() != self.sector_size as usize {
            return Err(anyhow!("Invalid sector size"));
        }
        let offset = sector as usize * self.sector_size as usize;
        self.data[offset..offset + self.sector_size as usize].copy_from_slice(buf);
        Ok(())
    }
}

impl Device for DiskDevice {
    fn device_type(&self) -> DeviceType { DeviceType::Disk }
    fn name(&self) -> &str { "VirtIO Block Device" }
    fn base_address(&self) -> u64 { 0xFF02_0000 }
    fn size(&self) -> u64 { 0x1000 }
    fn state(&self) -> DeviceState { self.state }
    fn set_state(&mut self, state: DeviceState) { self.state = state; }
    fn registers(&self) -> &DeviceRegisters { &self.registers }
    fn registers_mut(&mut self) -> &mut DeviceRegisters { &mut self.registers }

    fn read(&mut self, offset: u64) -> Result<u32> {
        match offset {
            0x00 => Ok(self.registers.data),
            0x04 => Ok(self.registers.status),
            0x08 => Ok(self.sector_size),
            0x0C => Ok(self.sectors as u32),
            0x10 => Ok((self.sectors >> 32) as u32),
            0x14 => Ok(self.registers.address), // Sector number
            0x18 => Ok(self.registers.control),
            0x1C => Ok(self.registers.interrupt_status),
            0x20 => Ok(self.registers.interrupt_enable),
            0x24 => Ok(self.registers.dma_address),
            0x28 => Ok(self.registers.dma_count),
            _ => Ok(0),
        }
    }

    fn write(&mut self, offset: u64, value: u32) -> Result<()> {
        match offset {
            0x00 => { // Data port (for PIO)
                self.registers.data = value;
            }
            0x14 => { // Sector address
                self.registers.address = value;
            }
            0x18 => { // Control
                self.registers.control = value;
                // Check for command
                match value & 0xFF {
                    0x20 => { // Read sector(s)
                        let count = self.registers.dma_count.max(1);
                        self.registers.status = 0x1; // Busy
                        self.registers.interrupt_status = 0;
                        // In real implementation, would DMA
                        self.registers.status = 0x2; // Ready
                        self.registers.interrupt_status |= 0x1;
                    }
                    0x30 => { // Write sector(s)
                        if self.read_only {
                            self.registers.status = 0x4; // Error
                        } else {
                            self.registers.status = 0x1;
                            self.registers.interrupt_status = 0;
                            self.registers.status = 0x2;
                            self.registers.interrupt_status |= 0x1;
                        }
                    }
                    0xEC => { // Identify
                        self.registers.status = 0x2;
                        self.registers.data = 0x0040; // Non-magnetic
                    }
                    _ => {}
                }
            }
            0x1C => { // Interrupt status (clear)
                self.registers.interrupt_status &= !value;
            }
            0x20 => { // Interrupt enable
                self.registers.interrupt_enable = value;
            }
            0x24 => { // DMA address
                self.registers.dma_address = value;
            }
            0x28 => { // DMA count
                self.registers.dma_count = value;
            }
            _ => {}
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.registers = DeviceRegisters::default();
        self.registers.status = 0x2; // Ready
        self.state = DeviceState::Ready;
        Ok(())
    }

    fn tick(&mut self, _delta: Duration) -> Result<()> {
        Ok(())
    }

    fn interrupt_pending(&self) -> bool {
        (self.registers.interrupt_status & self.registers.interrupt_enable) != 0
    }

    fn handle_interrupt(&mut self) -> Result<()> {
        self.registers.interrupt_status &= !0x1;
        Ok(())
    }
}

pub struct NetworkDevice {
    registers: DeviceRegisters,
    state: DeviceState,
    mac: [u8; 6],
    ip: [u8; 4],
    netmask: [u8; 4],
    gateway: [u8; 4],
    rx_buffer: VecDeque<Vec<u8>>,
    tx_buffer: VecDeque<Vec<u8>>,
    promiscuous: bool,
    multicast: Vec<[u8; 6]>,
    stats: NetworkStats,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
}

impl NetworkDevice {
    pub fn new(base_address: u64) -> Self {
        Self {
            registers: DeviceRegisters::default(),
            state: DeviceState::Ready,
            mac: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
            ip: [10, 0, 2, 15],
            netmask: [255, 255, 255, 0],
            gateway: [10, 0, 2, 2],
            rx_buffer: VecDeque::with_capacity(128),
            tx_buffer: VecDeque::with_capacity(128),
            promiscuous: false,
            multicast: Vec::new(),
            stats: NetworkStats::default(),
        }
    }

    pub fn receive_packet(&mut self, packet: Vec<u8>) {
        if self.rx_buffer.len() < 128 {
            self.stats.rx_packets += 1;
            self.stats.rx_bytes += packet.len() as u64;
            self.rx_buffer.push_back(packet);
            self.registers.interrupt_status |= 0x1; // RX interrupt
        } else {
            self.stats.rx_dropped += 1;
        }
    }

    pub fn transmit_packet(&mut self) -> Option<Vec<u8>> {
        if let Some(packet) = self.tx_buffer.pop_front() {
            self.stats.tx_packets += 1;
            self.stats.tx_bytes += packet.len() as u64;
            self.registers.interrupt_status |= 0x2; // TX interrupt
            Some(packet)
        } else {
            None
        }
    }
}

impl Device for NetworkDevice {
    fn device_type(&self) -> DeviceType { DeviceType::Network }
    fn name(&self) -> &str { "VirtIO Network" }
    fn base_address(&self) -> u64 { 0xFF03_0000 }
    fn size(&self) -> u64 { 0x1000 }
    fn state(&self) -> DeviceState { self.state }
    fn set_state(&mut self, state: DeviceState) { self.state = state; }
    fn registers(&self) -> &DeviceRegisters { &self.registers }
    fn registers_mut(&mut self) -> &mut DeviceRegisters { &mut self.registers }

    fn read(&mut self, offset: u64) -> Result<u32> {
        match offset {
            0x00 => Ok(u32::from_ne_bytes([self.mac[0], self.mac[1], self.mac[2], self.mac[3]])),
            0x04 => Ok(u16::from_ne_bytes([self.mac[4], self.mac[5]]) as u32),
            0x08 => Ok(u32::from_ne_bytes(self.ip)),
            0x0C => Ok(u32::from_ne_bytes(self.netmask)),
            0x10 => Ok(u32::from_ne_bytes(self.gateway)),
            0x14 => Ok(self.registers.status),
            0x18 => Ok(self.registers.control),
            0x1C => Ok(self.registers.interrupt_status),
            0x20 => Ok(self.registers.interrupt_enable),
            0x24 => Ok(self.rx_buffer.len() as u32),
            0x28 => Ok(self.tx_buffer.len() as u32),
            0x2C => Ok(self.stats.rx_packets as u32),
            0x30 => Ok(self.stats.tx_packets as u32),
            0x34 => Ok(self.stats.rx_bytes as u32),
            0x38 => Ok(self.stats.tx_bytes as u32),
            _ => Ok(0),
        }
    }

    fn write(&mut self, offset: u64, value: u32) -> Result<()> {
        match offset {
            0x18 => { // Control
                self.registers.control = value;
                if value & 0x1 != 0 { // Start RX
                    self.registers.status |= 0x1;
                }
                if value & 0x2 != 0 { // Start TX
                    self.registers.status |= 0x2;
                }
            }
            0x1C => { // Interrupt status (clear)
                self.registers.interrupt_status &= !value;
            }
            0x20 => { // Interrupt enable
                self.registers.interrupt_enable = value;
            }
            0x3C => { // TX data port
                // In real implementation, would build packet
                self.tx_buffer.push_back(value.to_ne_bytes().to_vec());
            }
            _ => {}
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.rx_buffer.clear();
        self.tx_buffer.clear();
        self.registers = DeviceRegisters::default();
        self.registers.status = 0x3; // RX ready, TX ready
        self.state = DeviceState::Ready;
        Ok(())
    }

    fn tick(&mut self, _delta: Duration) -> Result<()> {
        Ok(())
    }

    fn interrupt_pending(&self) -> bool {
        (self.registers.interrupt_status & self.registers.interrupt_enable) != 0
    }

    fn handle_interrupt(&mut self) -> Result<()> {
        self.registers.interrupt_status = 0;
        Ok(())
    }
}

pub struct TimerDevice {
    registers: DeviceRegisters,
    state: DeviceState,
    frequency: u64,
    counter: u64,
    compare: u64,
    periodic: bool,
    running: bool,
    last_tick: Instant,
}

impl TimerDevice {
    pub fn new(base_address: u64, frequency: u64) -> Self {
        Self {
            registers: DeviceRegisters::default(),
            state: DeviceState::Ready,
            frequency,
            counter: 0,
            compare: 0,
            periodic: false,
            running: false,
            last_tick: Instant::now(),
        }
    }

    pub fn set_frequency(&mut self, hz: u64) {
        self.frequency = hz;
    }

    pub fn set_oneshot(&mut self, ticks: u64) {
        self.compare = self.counter + ticks;
        self.periodic = false;
        self.running = true;
    }

    pub fn set_periodic(&mut self, hz: u64) {
        self.frequency = hz;
        self.compare = self.frequency / hz;
        self.periodic = true;
        self.running = true;
    }
}

impl Device for TimerDevice {
    fn device_type(&self) -> DeviceType { DeviceType::Timer }
    fn name(&self) -> &str { "HPET Timer" }
    fn base_address(&self) -> u64 { 0xFF04_0000 }
    fn size(&self) -> u64 { 0x1000 }
    fn state(&self) -> DeviceState { self.state }
    fn set_state(&mut self, state: DeviceState) { self.state = state; }
    fn registers(&self) -> &DeviceRegisters { &self.registers }
    fn registers_mut(&mut self) -> &mut DeviceRegisters { &mut self.registers }

    fn read(&mut self, offset: u64) -> Result<u32> {
        match offset {
            0x00 => Ok(self.counter as u32),
            0x04 => Ok((self.counter >> 32) as u32),
            0x08 => Ok(self.compare as u32),
            0x0C => Ok((self.compare >> 32) as u32),
            0x10 => Ok(self.registers.control),
            0x14 => Ok(self.registers.status),
            0x18 => Ok(self.registers.interrupt_status),
            0x1C => Ok(self.registers.interrupt_enable),
            _ => Ok(0),
        }
    }

    fn write(&mut self, offset: u64, value: u32) -> Result<()> {
        match offset {
            0x08 => { // Compare low
                self.compare = (self.compare & 0xFFFFFFFF00000000) | value as u64;
            }
            0x0C => { // Compare high
                self.compare = (self.compare & 0xFFFFFFFF) | ((value as u64) << 32);
            }
            0x10 => { // Control
                self.registers.control = value;
                self.running = value & 0x1 != 0;
                self.periodic = value & 0x2 != 0;
            }
            0x18 => { // Interrupt status (clear)
                self.registers.interrupt_status &= !value;
            }
            0x1C => { // Interrupt enable
                self.registers.interrupt_enable = value;
            }
            _ => {}
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.counter = 0;
        self.compare = 0;
        self.running = false;
        self.periodic = false;
        self.registers = DeviceRegisters::default();
        self.state = DeviceState::Ready;
        Ok(())
    }

    fn tick(&mut self, delta: Duration) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        let ticks = (delta.as_nanos() as u64 * self.frequency) / 1_000_000_000;
        self.counter += ticks;

        if self.counter >= self.compare {
            self.registers.interrupt_status |= 0x1;
            if self.periodic {
                self.counter -= self.compare;
            } else {
                self.running = false;
            }
        }
        Ok(())
    }

    fn interrupt_pending(&self) -> bool {
        (self.registers.interrupt_status & self.registers.interrupt_enable) != 0
    }

    fn handle_interrupt(&mut self) -> Result<()> {
        self.registers.interrupt_status &= !0x1;
        Ok(())
    }
}

pub struct RTCDevice {
    registers: DeviceRegisters,
    state: DeviceState,
    base_time: SystemTime,
}

impl RTCDevice {
    pub fn new(base_address: u64) -> Self {
        Self {
            registers: DeviceRegisters::default(),
            state: DeviceState::Ready,
            base_time: SystemTime::now(),
        }
    }
}

impl Device for RTCDevice {
    fn device_type(&self) -> DeviceType { DeviceType::RTC }
    fn name(&self) -> &str { "CMOS RTC" }
    fn base_address(&self) -> u64 { 0xFF05_0000 }
    fn size(&self) -> u64 { 0x100 }
    fn state(&self) -> DeviceState { self.state }
    fn set_state(&mut self, state: DeviceState) { self.state = state; }
    fn registers(&self) -> &DeviceRegisters { &self.registers }
    fn registers_mut(&mut self) -> &mut DeviceRegisters { &mut self.registers }

    fn read(&mut self, offset: u64) -> Result<u32> {
        let elapsed = SystemTime::now().duration_since(self.base_time).unwrap_or_default();
        let secs = elapsed.as_secs();
        
        match offset {
            0x00 => Ok((secs % 60) as u32),        // Seconds
            0x04 => Ok((secs / 60 % 60) as u32),   // Minutes
            0x08 => Ok((secs / 3600 % 24) as u32), // Hours
            0x0C => Ok((secs / 86400) as u32),     // Days
            0x10 => Ok(self.registers.control),
            0x14 => Ok(self.registers.status),
            _ => Ok(0),
        }
    }

    fn write(&mut self, offset: u64, value: u32) -> Result<()> {
        match offset {
            0x10 => { self.registers.control = value; }
            _ => {}
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.base_time = SystemTime::now();
        self.registers = DeviceRegisters::default();
        self.state = DeviceState::Ready;
        Ok(())
    }

    fn tick(&mut self, _delta: Duration) -> Result<()> {
        Ok(())
    }

    fn interrupt_pending(&self) -> bool { false }
    fn handle_interrupt(&mut self) -> Result<()> { Ok(()) }
}

pub struct DeviceManager {
    devices: HashMap<DeviceType, Box<dyn Device>>,
    address_map: HashMap<u64, DeviceType>, // base_address -> device_type
    interrupt_controller: InterruptController,
    dma_controller: DMAController,
}

struct InterruptController {
    enabled: bool,
    pending: u32,
    mask: u32,
    vectors: HashMap<u32, DeviceType>, // IRQ -> device
}

impl InterruptController {
    fn new() -> Self {
        Self {
            enabled: true,
            pending: 0,
            mask: 0,
            vectors: HashMap::new(),
        }
    }

    fn register_irq(&mut self, irq: u32, device: DeviceType) {
        self.vectors.insert(irq, device);
    }

    fn raise_irq(&mut self, irq: u32) {
        if self.enabled && (self.mask & (1 << irq)) == 0 {
            self.pending |= 1 << irq;
        }
    }

    fn ack_irq(&mut self, irq: u32) {
        self.pending &= !(1 << irq);
    }

    fn get_pending_irq(&self) -> Option<u32> {
        for i in 0..32 {
            if self.pending & (1 << i) != 0 {
                return Some(i);
            }
        }
        None
    }
}

struct DMAController {
    channels: [DMAChannel; 8],
}

#[derive(Debug, Clone, Default)]
struct DMAChannel {
    active: bool,
    source: u64,
    destination: u64,
    count: u32,
    direction: bool, // true = device to memory
    mode: u8,
}

impl DMAController {
    fn new() -> Self {
        Self { channels: Default::default() }
    }

    fn start_transfer(&mut self, channel: usize, src: u64, dst: u64, count: u32, dir: bool) -> Result<()> {
        if channel >= 8 {
            return Err(anyhow!("Invalid DMA channel"));
        }
        self.channels[channel] = DMAChannel {
            active: true,
            source: src,
            destination: dst,
            count,
            direction: dir,
            mode: 0,
        };
        Ok(())
    }
}

impl DeviceManager {
    pub fn new() -> Self {
        let mut manager = Self {
            devices: HashMap::new(),
            address_map: HashMap::new(),
            interrupt_controller: InterruptController::new(),
            dma_controller: DMAController::new(),
        };

        // Register standard devices
        manager.register_device(Box::new(KeyboardDevice::new(0xFF00_0000)));
        manager.register_device(Box::new(DisplayDevice::new(0xFF01_0000, 1024, 768)));
        manager.register_device(Box::new(DiskDevice::new(0xFF02_0000, 64))); // 64MB
        manager.register_device(Box::new(NetworkDevice::new(0xFF03_0000)));
        manager.register_device(Box::new(TimerDevice::new(0xFF04_0000, 1_000_000))); // 1MHz
        manager.register_device(Box::new(RTCDevice::new(0xFF05_0000)));

        // Setup IRQ mappings
        manager.interrupt_controller.register_irq(1, DeviceType::Keyboard);
        manager.interrupt_controller.register_irq(2, DeviceType::Display);
        manager.interrupt_controller.register_irq(3, DeviceType::Disk);
        manager.interrupt_controller.register_irq(4, DeviceType::Network);
        manager.interrupt_controller.register_irq(5, DeviceType::Timer);
        manager.interrupt_controller.register_irq(6, DeviceType::RTC);

        manager
    }

    fn register_device(&mut self, device: Box<dyn Device>) {
        let dev_type = device.device_type();
        let base = device.base_address();
        self.address_map.insert(base, dev_type);
        self.devices.insert(dev_type, device);
    }

    pub fn get_device(&self, dev_type: DeviceType) -> Option<&Box<dyn Device>> {
        self.devices.get(&dev_type)
    }

    pub fn get_device_mut(&mut self, dev_type: DeviceType) -> Option<&mut Box<dyn Device>> {
        self.devices.get_mut(&dev_type)
    }

    pub fn read_device(&mut self, address: u64) -> Result<u32> {
        // Find device by address
        for (&base, &dev_type) in &self.address_map {
            let device = self.devices.get(&dev_type).unwrap();
            let size = device.size();
            if address >= base && address < base + size {
                let offset = address - base;
                return device.read(offset);
            }
        }
        Err(anyhow!("No device at address 0x{:X}", address))
    }

    pub fn write_device(&mut self, address: u64, value: u32) -> Result<()> {
        for (&base, &dev_type) in &self.address_map {
            let device = self.devices.get(&dev_type).unwrap();
            let size = device.size();
            if address >= base && address < base + size {
                let offset = address - base;
                return device.write(offset, value);
            }
        }
        Err(anyhow!("No device at address 0x{:X}", address))
    }

    pub fn tick(&mut self, delta: Duration) -> Result<()> {
        for device in self.devices.values_mut() {
            device.tick(delta)?;
            
            // Check for interrupts
            if device.interrupt_pending() {
                // Find IRQ for this device
                for (&irq, &dev_type) in &self.interrupt_controller.vectors {
                    if dev_type == device.device_type() {
                        self.interrupt_controller.raise_irq(irq);
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn poll_interrupts(&mut self) -> Option<u32> {
        self.interrupt_controller.get_pending_irq()
    }

    pub fn ack_interrupt(&mut self, irq: u32) {
        self.interrupt_controller.ack_irq(irq);
        
        // Also clear in device
        if let Some(&dev_type) = self.interrupt_controller.vectors.get(&irq) {
            if let Some(device) = self.devices.get_mut(&dev_type) {
                let _ = device.handle_interrupt();
            }
        }
    }

    pub fn list_devices(&self) -> Result<String> {
        let mut output = String::from("Virtual Devices:\n");
        output.push_str(&format!("{:<20} {:<12} {:<10} {:<10} {}\n", "Name", "Type", "Base", "Size", "State"));
        output.push_str(&"-".repeat(80));
        output.push('\n');
        
        for device in self.devices.values() {
            output.push_str(&format!(
                "{:<20} {:<12} 0x{:08X} 0x{:08X} {:?}\n",
                device.name(),
                format!("{:?}", device.device_type()),
                device.base_address(),
                device.size(),
                device.state()
            ));
        }
        Ok(output)
    }

    pub fn get_keyboard(&mut self) -> Option<&mut KeyboardDevice> {
        self.devices.get_mut(&DeviceType::Keyboard)
            .and_then(|d| d.as_any_mut().downcast_mut::<KeyboardDevice>())
    }

    pub fn get_display(&mut self) -> Option<&mut DisplayDevice> {
        self.devices.get_mut(&DeviceType::Display)
            .and_then(|d| d.as_any_mut().downcast_mut::<DisplayDevice>())
    }

    pub fn get_disk(&mut self) -> Option<&mut DiskDevice> {
        self.devices.get_mut(&DeviceType::Disk)
            .and_then(|d| d.as_any_mut().downcast_mut::<DiskDevice>())
    }

    pub fn get_network(&mut self) -> Option<&mut NetworkDevice> {
        self.devices.get_mut(&DeviceType::Network)
            .and_then(|d| d.as_any_mut().downcast_mut::<NetworkDevice>())
    }

    pub fn get_timer(&mut self) -> Option<&mut TimerDevice> {
        self.devices.get_mut(&DeviceType::Timer)
            .and_then(|d| d.as_any_mut().downcast_mut::<TimerDevice>())
    }

    pub fn get_rtc(&mut self) -> Option<&mut RTCDevice> {
        self.devices.get_mut(&DeviceType::RTC)
            .and_then(|d| d.as_any_mut().downcast_mut::<RTCDevice>())
    }

    pub async fn poll_loop(&mut self, running: Arc<tokio::sync::RwLock<bool>>) {
        let mut interval = tokio::time::interval(Duration::from_millis(10));
        while *running.read().await {
            interval.tick().await;
            let _ = self.tick(Duration::from_millis(10));
        }
    }
}

// Extension trait for downcasting
trait DeviceExt {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: Device + 'static> DeviceExt for T {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Device for Box<dyn Device> {
    fn device_type(&self) -> DeviceType { (**self).device_type() }
    fn name(&self) -> &str { (**self).name() }
    fn base_address(&self) -> u64 { (**self).base_address() }
    fn size(&self) -> u64 { (**self).size() }
    fn state(&self) -> DeviceState { (**self).state() }
    fn set_state(&mut self, state: DeviceState) { (**self).set_state(state) }
    fn registers(&self) -> &DeviceRegisters { (**self).registers() }
    fn registers_mut(&mut self) -> &mut DeviceRegisters { (**self).registers_mut() }
    fn read(&mut self, offset: u64) -> Result<u32> { (**self).read(offset) }
    fn write(&mut self, offset: u64, value: u32) -> Result<()> { (**self).write(offset, value) }
    fn reset(&mut self) -> Result<()> { (**self).reset() }
    fn tick(&mut self, delta: Duration) -> Result<()> { (**self).tick(delta) }
    fn interrupt_pending(&self) -> bool { (**self).interrupt_pending() }
    fn handle_interrupt(&mut self) -> Result<()> { (**self).handle_interrupt() }
}