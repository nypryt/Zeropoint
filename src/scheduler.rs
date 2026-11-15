//! ZEROPOINT Process Scheduler
//! 
//! Implements a preemptive multitasking scheduler with multiple scheduling policies.

use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock as AsyncRwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessState {
    Created,
    Ready,
    Running,
    Blocked,
    Terminated,
    Zombie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    RoundRobin,
    Priority,
    FCFS,
    SJF,
}

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub state: ProcessState,
    pub priority: i32,
    pub policy: SchedulingPolicy,
    pub cpu_time: Duration,
    pub start_time: Instant,
    pub registers: [u32; 16],
    pub pc: u32,
    pub sp: u32,
    pub memory_base: u64,
    pub memory_size: usize,
    pub open_files: Vec<u32>,
    pub exit_code: Option<i32>,
    pub children: Vec<u32>,
    pub thread_id: Option<Uuid>,
}

impl Process {
    pub fn new(pid: u32, ppid: u32, name: String, priority: i32) -> Self {
        let mut registers = [0u32; 16];
        registers[6] = 0x00FF_F000; // SP
        registers[7] = 0x00FF_F000; // FP
        
        Self {
            pid,
            ppid,
            name,
            state: ProcessState::Created,
            priority,
            policy: SchedulingPolicy::RoundRobin,
            cpu_time: Duration::ZERO,
            start_time: Instant::now(),
            registers,
            pc: 0,
            sp: 0x00FF_F000,
            memory_base: 0,
            memory_size: 0,
            open_files: Vec::new(),
            exit_code: None,
            children: Vec::new(),
            thread_id: None,
        }
    }

    pub fn is_alive(&self) -> bool {
        matches!(self.state, ProcessState::Created | ProcessState::Ready | ProcessState::Running | ProcessState::Blocked)
    }
}

pub struct Scheduler {
    processes: HashMap<u32, Process>,
    ready_queues: HashMap<i32, VecDeque<u32>>, // priority -> queue of PIDs
    current_pid: Option<u32>,
    next_pid: u32,
    time_slice: Duration,
    last_switch: Instant,
    policy: SchedulingPolicy,
    idle_pid: u32,
    stats: SchedulerStats,
}

#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub context_switches: u64,
    pub total_processes: u64,
    pub terminated_processes: u64,
    pub total_cpu_time: Duration,
}

impl Scheduler {
    pub fn new() -> Self {
        let mut scheduler = Self {
            processes: HashMap::new(),
            ready_queues: HashMap::new(),
            current_pid: None,
            next_pid: 1,
            time_slice: Duration::from_millis(10),
            last_switch: Instant::now(),
            policy: SchedulingPolicy::RoundRobin,
            idle_pid: 0,
            stats: SchedulerStats::default(),
        };

        // Create idle process
        let idle = Process::new(0, 0, "idle".to_string(), -20);
        scheduler.processes.insert(0, idle);
        scheduler.idle_pid = 0;
        
        scheduler
    }

    pub fn create_process(&mut self, name: String, priority: i32, entry_point: u32, stack_base: u32) -> Result<u32> {
        let pid = self.next_pid;
        self.next_pid += 1;
        
        let mut process = Process::new(pid, self.current_pid.unwrap_or(0), name, priority);
        process.pc = entry_point;
        process.sp = stack_base;
        process.state = ProcessState::Ready;
        
        // Add to ready queue
        self.ready_queues.entry(priority).or_default().push_back(pid);
        self.processes.insert(pid, process);
        self.stats.total_processes += 1;
        
        Ok(pid)
    }

    pub fn fork_process(&mut self, parent_pid: u32) -> Result<u32> {
        let parent = self.processes.get(&parent_pid)
            .ok_or_else(|| anyhow!("Parent process not found"))?
            .clone();
        
        let pid = self.next_pid;
        self.next_pid += 1;
        
        let mut child = Process::new(pid, parent_pid, format!("{} (child)", parent.name), parent.priority);
        child.registers = parent.registers;
        child.pc = parent.pc;
        child.sp = parent.sp;
        child.memory_base = parent.memory_base;
        child.memory_size = parent.memory_size;
        child.open_files = parent.open_files.clone();
        child.policy = parent.policy;
        child.state = ProcessState::Ready;
        
        // In child, return 0 from fork
        child.registers[0] = 0;
        
        // In parent, return child PID
        if let Some(parent_proc) = self.processes.get_mut(&parent_pid) {
            parent_proc.registers[0] = pid;
            parent_proc.children.push(pid);
        }
        
        self.ready_queues.entry(child.priority).or_default().push_back(pid);
        self.processes.insert(pid, child);
        self.stats.total_processes += 1;
        
        Ok(pid)
    }

    pub fn exec_process(&mut self, pid: u32, entry_point: u32, stack_base: u32, args: Vec<String>) -> Result<()> {
        let process = self.processes.get_mut(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        process.pc = entry_point;
        process.sp = stack_base;
        process.registers = [0; 16];
        process.registers[6] = stack_base; // SP
        process.registers[7] = stack_base; // FP
        process.name = args.get(0).cloned().unwrap_or_else(|| "exec".to_string());
        process.state = ProcessState::Ready;
        
        Ok(())
    }

    pub fn terminate_process(&mut self, pid: u32, exit_code: i32) -> Result<()> {
        if pid == 0 {
            return Err(anyhow!("Cannot terminate idle process"));
        }
        
        let process = self.processes.get_mut(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        process.state = ProcessState::Terminated;
        process.exit_code = Some(exit_code);
        process.cpu_time += self.last_switch.elapsed();
        self.stats.terminated_processes += 1;
        self.stats.total_cpu_time += process.cpu_time;
        
        // Remove from ready queue
        for queue in self.ready_queues.values_mut() {
            queue.retain(|&p| p != pid);
        }
        
        // If this was the current process, schedule next
        if self.current_pid == Some(pid) {
            self.current_pid = None;
        }
        
        // Notify parent
        if let Some(parent) = self.processes.get_mut(&process.ppid) {
            parent.children.retain(|&p| p != pid);
        }
        
        // Reparent children to init (pid 1)
        for child_pid in &process.children {
            if let Some(child) = self.processes.get_mut(child_pid) {
                child.ppid = 1;
            }
        }
        
        Ok(())
    }

    pub fn block_process(&mut self, pid: u32) -> Result<()> {
        let process = self.processes.get_mut(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        if process.state == ProcessState::Running {
            process.state = ProcessState::Blocked;
            
            if self.current_pid == Some(pid) {
                self.current_pid = None;
            }
        }
        
        Ok(())
    }

    pub fn unblock_process(&mut self, pid: u32) -> Result<()> {
        let process = self.processes.get_mut(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        if process.state == ProcessState::Blocked {
            process.state = ProcessState::Ready;
            self.ready_queues.entry(process.priority).or_default().push_back(pid);
        }
        
        Ok(())
    }

    pub fn set_priority(&mut self, pid: u32, priority: i32) -> Result<()> {
        let process = self.processes.get_mut(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        // Remove from old queue
        if let Some(queue) = self.ready_queues.get_mut(&process.priority) {
            queue.retain(|&p| p != pid);
        }
        
        process.priority = priority;
        
        // Add to new queue if ready
        if process.state == ProcessState::Ready {
            self.ready_queues.entry(priority).or_default().push_back(pid);
        }
        
        Ok(())
    }

    pub fn set_policy(&mut self, policy: SchedulingPolicy) {
        self.policy = policy;
    }

    pub fn get_current_process(&self) -> Option<&Process> {
        self.current_pid.and_then(|pid| self.processes.get(&pid))
    }

    pub fn get_current_pid(&self) -> Option<u32> {
        self.current_pid
    }

    pub fn get_process(&self, pid: u32) -> Option<&Process> {
        self.processes.get(&pid)
    }

    pub fn get_process_mut(&mut self, pid: u32) -> Option<&mut Process> {
        self.processes.get_mut(&pid)
    }

    pub fn list_processes(&self) -> Vec<&Process> {
        self.processes.values().collect()
    }

    pub fn kill_process(&mut self, pid: u32) -> Result<()> {
        self.terminate_process(pid, -9)
    }

    pub fn schedule(&mut self) -> Option<u32> {
        // Save current process state if running
        if let Some(current_pid) = self.current_pid {
            if let Some(current) = self.processes.get_mut(&current_pid) {
                if current.state == ProcessState::Running {
                    current.state = ProcessState::Ready;
                    current.cpu_time += self.last_switch.elapsed();
                    self.ready_queues.entry(current.priority).or_default().push_back(current_pid);
                }
            }
        }

        // Select next process based on policy
        let next_pid = match self.policy {
            SchedulingPolicy::RoundRobin => self.select_round_robin(),
            SchedulingPolicy::Priority => self.select_priority(),
            SchedulingPolicy::FCFS => self.select_fcfs(),
            SchedulingPolicy::SJF => self.select_sjf(),
        };

        // If no ready process, run idle
        let next_pid = next_pid.unwrap_or(self.idle_pid);

        // Update state
        if let Some(process) = self.processes.get_mut(&next_pid) {
            process.state = ProcessState::Running;
        }

        self.current_pid = Some(next_pid);
        self.last_switch = Instant::now();
        self.stats.context_switches += 1;

        Some(next_pid)
    }

    fn select_round_robin(&self) -> Option<u32> {
        // Simple round-robin: highest priority non-empty queue, then FIFO
        let mut priorities: Vec<i32> = self.ready_queues.keys().copied().collect();
        priorities.sort_by(|a, b| b.cmp(a)); // Higher priority first
        
        for priority in priorities {
            if let Some(queue) = self.ready_queues.get(&priority) {
                if !queue.is_empty() {
                    return queue.front().copied();
                }
            }
        }
        None
    }

    fn select_priority(&self) -> Option<u32> {
        // Priority scheduling: highest priority process
        self.select_round_robin()
    }

    fn select_fcfs(&self) -> Option<u32> {
        // First-come-first-served: earliest created ready process
        let mut earliest: Option<(Instant, u32)> = None;
        
        for (pid, process) in &self.processes {
            if process.state == ProcessState::Ready {
                match earliest {
                    Some((time, _)) if process.start_time < time => {
                        earliest = Some((process.start_time, *pid));
                    }
                    None => {
                        earliest = Some((process.start_time, *pid));
                    }
                    _ => {}
                }
            }
        }
        
        earliest.map(|(_, pid)| pid)
    }

    fn select_sjf(&self) -> Option<u32> {
        // Shortest job first: process with least CPU time used
        let mut shortest: Option<(Duration, u32)> = None;
        
        for (pid, process) in &self.processes {
            if process.state == ProcessState::Ready {
                match shortest {
                    Some((time, _)) if process.cpu_time < time => {
                        shortest = Some((process.cpu_time, *pid));
                    }
                    None => {
                        shortest = Some((process.cpu_time, *pid));
                    }
                    _ => {}
                }
            }
        }
        
        shortest.map(|(_, pid)| pid)
    }

    pub fn save_context(&mut self, pid: u32, registers: [u32; 16], pc: u32, sp: u32) -> Result<()> {
        let process = self.processes.get_mut(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        process.registers = registers;
        process.pc = pc;
        process.sp = sp;
        Ok(())
    }

    pub fn load_context(&self, pid: u32) -> Result<([u32; 16], u32, u32)> {
        let process = self.processes.get(&pid)
            .ok_or_else(|| anyhow!("Process not found"))?;
        
        Ok((process.registers, process.pc, process.sp))
    }

    pub async fn run(&mut self, running: Arc<AsyncRwLock<bool>>) {
        while *running.read().await {
            // Schedule next process
            self.schedule();
            
            // Run for time slice
            tokio::time::sleep(self.time_slice).await;
            
            // Check for blocked processes that can be unblocked
            self.check_blocked_processes().await;
        }
    }

    async fn check_blocked_processes(&mut self) {
        // In a real implementation, this would check I/O completion, timers, etc.
        // For now, we'll just simulate some unblocking
        let blocked_pids: Vec<u32> = self.processes.iter()
            .filter(|(_, p)| p.state == ProcessState::Blocked)
            .map(|(pid, _)| *pid)
            .collect();
        
        for pid in blocked_pids {
            // Simulate random unblocking (10% chance)
            if rand::random::<f32>() < 0.1 {
                let _ = self.unblock_process(pid);
            }
        }
    }

    pub fn get_stats(&self) -> &SchedulerStats {
        &self.stats
    }

    pub fn get_ready_queue_sizes(&self) -> HashMap<i32, usize> {
        self.ready_queues.iter()
            .map(|(k, v)| (*k, v.len()))
            .collect()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}