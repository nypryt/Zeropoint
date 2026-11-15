//! ZEROPOINT - A virtual computer inside a terminal

pub mod cpu;
pub mod mem;
pub mod vfs;
pub mod asm;
pub mod shell;
pub mod scheduler;
pub mod device;
pub mod ui;
pub mod fs;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cpu::CPU;
use crate::mem::MemoryManager;
use crate::vfs::VFS;
use crate::scheduler::Scheduler;
use crate::device::DeviceManager;
use crate::shell::Shell;
use crate::ui::UI;

pub struct ZeroPoint {
    pub cpu: Arc<RwLock<CPU>>,
    pub memory: Arc<RwLock<MemoryManager>>,
    pub vfs: Arc<RwLock<VFS>>,
    pub scheduler: Arc<RwLock<Scheduler>>,
    pub devices: Arc<RwLock<DeviceManager>>,
    pub shell: Arc<RwLock<Shell>>,
    pub ui: Arc<RwLock<UI>>,
    pub running: Arc<RwLock<bool>>,
}

impl ZeroPoint {
    pub async fn new() -> Result<Self> {
        let memory = Arc::new(RwLock::new(MemoryManager::new(64 * 1024 * 1024)?)); // 64MB
        let vfs = Arc::new(RwLock::new(VFS::new()?));
        let cpu = Arc::new(RwLock::new(CPU::new(Arc::clone(&memory), Arc::clone(&vfs))));
        let scheduler = Arc::new(RwLock::new(Scheduler::new()));
        let devices = Arc::new(RwLock::new(DeviceManager::new()));
        let shell = Arc::new(RwLock::new(Shell::new(
            Arc::clone(&cpu),
            Arc::clone(&memory),
            Arc::clone(&vfs),
            Arc::clone(&scheduler),
            Arc::clone(&devices),
        )));
        let ui = Arc::new(RwLock::new(UI::new(
            Arc::clone(&cpu),
            Arc::clone(&memory),
            Arc::clone(&vfs),
            Arc::clone(&scheduler),
            Arc::clone(&devices),
            Arc::clone(&shell),
        )));

        Ok(Self {
            cpu,
            memory,
            vfs,
            scheduler,
            devices,
            shell,
            ui,
            running: Arc::new(RwLock::new(true)),
        })
    }

    pub async fn run(&self) -> Result<()> {
        // Start the CPU execution loop
        let cpu_clone = Arc::clone(&self.cpu);
        let running_clone = Arc::clone(&self.running);
        tokio::spawn(async move {
            let mut cpu = cpu_clone.write().await;
            cpu.run(running_clone).await;
        });

        // Start the scheduler
        let scheduler_clone = Arc::clone(&self.scheduler);
        let running_clone = Arc::clone(&self.running);
        tokio::spawn(async move {
            let mut scheduler = scheduler_clone.write().await;
            scheduler.run(running_clone).await;
        });

        // Start device polling
        let devices_clone = Arc::clone(&self.devices);
        let running_clone = Arc::clone(&self.running);
        tokio::spawn(async move {
            let mut devices = devices_clone.write().await;
            devices.poll_loop(running_clone).await;
        });

        // Run the UI (blocking)
        let mut ui = self.ui.write().await;
        ui.run().await?;

        Ok(())
    }

    pub async fn shutdown(&self) {
        *self.running.write().await = false;
    }
}