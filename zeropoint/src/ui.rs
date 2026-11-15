//! ZEROPOINT Terminal User Interface
//! 
//! Main UI showing filesystem, devices, CPU state, memory, and shell.

use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap},
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use crate::cpu::CPU;
use crate::mem::MemoryManager;
use crate::vfs::VFS;
use crate::scheduler::Scheduler;
use crate::device::{DeviceManager, DeviceType, KeyboardDevice, DisplayDevice};
use crate::shell::Shell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UITab {
    Shell,
    CPU,
    Memory,
    Filesystem,
    Devices,
    Processes,
    Network,
    Logs,
}

impl UITab {
    pub fn all() -> [UITab; 8] {
        [UITab::Shell, UITab::CPU, UITab::Memory, UITab::Filesystem, 
         UITab::Devices, UITab::Processes, UITab::Network, UITab::Logs]
    }

    pub fn title(&self) -> &'static str {
        match self {
            UITab::Shell => "Shell",
            UITab::CPU => "CPU",
            UITab::Memory => "Memory",
            UITab::Filesystem => "FS",
            UITab::Devices => "Devices",
            UITab::Processes => "Processes",
            UITab::Network => "Network",
            UITab::Logs => "Logs",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            UITab::Shell => 0,
            UITab::CPU => 1,
            UITab::Memory => 2,
            UITab::Filesystem => 3,
            UITab::Devices => 4,
            UITab::Processes => 5,
            UITab::Network => 6,
            UITab::Logs => 7,
        }
    }
}

pub struct UI {
    cpu: Arc<RwLock<CPU>>,
    memory: Arc<RwLock<MemoryManager>>,
    vfs: Arc<RwLock<VFS>>,
    scheduler: Arc<RwLock<Scheduler>>,
    devices: Arc<RwLock<DeviceManager>>,
    shell: Arc<RwLock<Shell>>,
    
    current_tab: UITab,
    shell_history: VecDeque<String>,
    shell_input: String,
    shell_cursor: usize,
    shell_scroll: usize,
    
    fs_current_path: String,
    fs_list_state: ListState,
    fs_expanded: std::collections::HashSet<String>,
    
    cpu_regs_scroll: usize,
    mem_scroll: usize,
    log_messages: VecDeque<String>,
    log_scroll: usize,
    
    running: bool,
    last_update: Instant,
    update_interval: Duration,
    
    // For keyboard input handling
    input_tx: mpsc::UnboundedSender<KeyEvent>,
    input_rx: mpsc::UnboundedReceiver<KeyEvent>,
}

impl UI {
    pub fn new(
        cpu: Arc<RwLock<CPU>>,
        memory: Arc<RwLock<MemoryManager>>,
        vfs: Arc<RwLock<VFS>>,
        scheduler: Arc<RwLock<Scheduler>>,
        devices: Arc<RwLock<DeviceManager>>,
        shell: Arc<RwLock<Shell>>,
    ) -> Self {
        let (input_tx, input_rx) = mpsc::unbounded_channel();
        
        let mut fs_list_state = ListState::default();
        fs_list_state.select(Some(0));
        
        Self {
            cpu,
            memory,
            vfs,
            scheduler,
            devices,
            shell,
            current_tab: UITab::Shell,
            shell_history: VecDeque::with_capacity(1000),
            shell_input: String::new(),
            shell_cursor: 0,
            shell_scroll: 0,
            fs_current_path: "/".to_string(),
            fs_list_state,
            fs_expanded: std::collections::HashSet::new(),
            cpu_regs_scroll: 0,
            mem_scroll: 0,
            log_messages: VecDeque::with_capacity(500),
            log_scroll: 0,
            running: true,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(100),
            input_tx,
            input_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Spawn input handler
        let input_tx = self.input_tx.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(Event::Key(key)) = event::read() {
                    if input_tx.send(key).is_err() {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        // Initial shell prompt
        self.add_shell_output(&format!("{}", self.shell.read().await.get_prompt())).await;

        // Main loop
        while self.running {
            // Handle input
            while let Ok(key) = self.input_rx.try_recv() {
                self.handle_key(key).await?;
            }

            // Update state periodically
            if self.last_update.elapsed() >= self.update_interval {
                self.update_state().await;
                self.last_update = Instant::now();
            }

            // Render
            terminal.draw(|f| self.render(f))?;

            tokio::time::sleep(Duration::from_millis(16)).await; // ~60 FPS
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }
            KeyCode::Tab => {
                // Cycle tabs forward
                let tabs = UITab::all();
                let next = (self.current_tab.index() + 1) % tabs.len();
                self.current_tab = tabs[next];
            }
            KeyCode::BackTab => {
                // Cycle tabs backward
                let tabs = UITab::all();
                let next = (self.current_tab.index() + tabs.len() - 1) % tabs.len();
                self.current_tab = tabs[next];
            }
            KeyCode::F(n) if n >= 1 && n <= 8 => {
                // Direct tab access via F1-F8
                let tabs = UITab::all();
                if (n as usize) <= tabs.len() {
                    self.current_tab = tabs[n as usize - 1];
                }
            }
            _ => {
                // Tab-specific handling
                match self.current_tab {
                    UITab::Shell => self.handle_shell_key(key).await?,
                    UITab::Filesystem => self.handle_fs_key(key).await?,
                    UITab::CPU => self.handle_cpu_key(key).await?,
                    UITab::Memory => self.handle_mem_key(key).await?,
                    UITab::Logs => self.handle_logs_key(key).await?,
                    _ => {}
                }
            }
        }
        Ok(())
    }

    async fn handle_shell_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                let input = self.shell_input.trim().to_string();
                if !input.is_empty() {
                    self.shell_history.push_back(format!("{}", input));
                    self.add_shell_output(&format!("{}", input)).await;
                    
                    // Execute command
                    let mut shell = self.shell.write().await;
                    let output = shell.run_command(&input).await.unwrap_or_else(|e| format!("Error: {}", e));
                    drop(shell);
                    
                    if !output.is_empty() {
                        for line in output.lines() {
                            self.add_shell_output(line).await;
                        }
                    }
                    
                    // Check if shell is still running
                    let shell = self.shell.read().await;
                    if !shell.is_running() {
                        self.running = false;
                    }
                    drop(shell);
                }
                
                // Add new prompt
                let shell = self.shell.read().await;
                self.add_shell_output(&format!("{}", shell.get_prompt())).await;
                
                self.shell_input.clear();
                self.shell_cursor = 0;
            }
            KeyCode::Backspace => {
                if self.shell_cursor > 0 {
                    self.shell_input.remove(self.shell_cursor - 1);
                    self.shell_cursor -= 1;
                }
            }
            KeyCode::Delete => {
                if self.shell_cursor < self.shell_input.len() {
                    self.shell_input.remove(self.shell_cursor);
                }
            }
            KeyCode::Left => {
                if self.shell_cursor > 0 {
                    self.shell_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.shell_cursor < self.shell_input.len() {
                    self.shell_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.shell_cursor = 0;
            }
            KeyCode::End => {
                self.shell_cursor = self.shell_input.len();
            }
            KeyCode::Up => {
                // History up
                if !self.shell_history.is_empty() {
                    // TODO: Implement history navigation
                }
            }
            KeyCode::Down => {
                // History down
            }
            KeyCode::Char(c) => {
                self.shell_input.insert(self.shell_cursor, c);
                self.shell_cursor += 1;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_fs_key(&mut self, key: KeyEvent) -> Result<()> {
        let vfs = self.vfs.read().await;
        let entries = vfs.readdir(&self.fs_current_path).unwrap_or_default();
        drop(vfs);
        
        match key.code {
            KeyCode::Up => {
                if let Some(selected) = self.fs_list_state.selected() {
                    if selected > 0 {
                        self.fs_list_state.select(Some(selected - 1));
                    }
                }
            }
            KeyCode::Down => {
                if let Some(selected) = self.fs_list_state.selected() {
                    if selected + 1 < entries.len() {
                        self.fs_list_state.select(Some(selected + 1));
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.fs_list_state.selected() {
                    if selected < entries.len() {
                        let entry = &entries[selected];
                        let new_path = if self.fs_current_path == "/" {
                            format!("/{}", entry.name)
                        } else {
                            format!("{}/{}", self.fs_current_path, entry.name)
                        };
                        
                        if entry.file_type == crate::vfs::FileType::Directory {
                            self.fs_current_path = new_path;
                            self.fs_list_state.select(Some(0));
                        } else {
                            // Try to view file
                            self.view_file(&new_path).await;
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                // Go up one directory
                if self.fs_current_path != "/" {
                    self.fs_current_path = std::path::Path::new(&self.fs_current_path)
                        .parent()
                        .and_then(|p| p.to_str())
                        .unwrap_or("/")
                        .to_string();
                    self.fs_list_state.select(Some(0));
                }
            }
            KeyCode::Char('r') => {
                // Refresh
            }
            _ => {}
        }
        Ok(())
    }

    async fn view_file(&mut self, path: &str) {
        let mut vfs = self.vfs.write().await;
        if let Ok(fd) = vfs.open(path, 0) {
            let mut content = String::new();
            let mut buf = [0u8; 4096];
            loop {
                match vfs.read(fd, &mut buf) {
                    Ok(0) => break,
                    Ok(n) => content.push_str(&String::from_utf8_lossy(&buf[..n])),
                    Err(_) => break,
                }
            }
            let _ = vfs.close(fd);
            self.add_log(&format!("File: {}", path));
            for line in content.lines().take(50) {
                self.add_log(line);
            }
            if content.lines().count() > 50 {
                self.add_log("... (truncated)");
            }
        }
    }

    async fn handle_cpu_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                if self.cpu_regs_scroll > 0 {
                    self.cpu_regs_scroll -= 1;
                }
            }
            KeyCode::Down => {
                self.cpu_regs_scroll += 1;
            }
            KeyCode::Char('r') => {
                // Reset CPU
                let mut cpu = self.cpu.write().await;
                cpu.halted = false;
                cpu.cycles = 0;
                cpu.registers = [0; 16];
                cpu.registers[6] = 0x00FF_F000;
                cpu.registers[7] = 0x00FF_F000;
            }
            KeyCode::Char('s') => {
                // Step CPU
                let mut cpu = self.cpu.write().await;
                let _ = cpu.step().await;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_mem_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                if self.mem_scroll > 0 {
                    self.mem_scroll -= 1;
                }
            }
            KeyCode::Down => {
                self.mem_scroll += 1;
            }
            KeyCode::PageUp => {
                self.mem_scroll = self.mem_scroll.saturating_sub(20);
            }
            KeyCode::PageDown => {
                self.mem_scroll += 20;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_logs_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                if self.log_scroll > 0 {
                    self.log_scroll -= 1;
                }
            }
            KeyCode::Down => {
                if self.log_scroll < self.log_messages.len().saturating_sub(1) {
                    self.log_scroll += 1;
                }
            }
            KeyCode::PageUp => {
                self.log_scroll = self.log_scroll.saturating_sub(20);
            }
            KeyCode::PageDown => {
                self.log_scroll = (self.log_scroll + 20).min(self.log_messages.len().saturating_sub(1));
            }
            KeyCode::Char('c') => {
                self.log_messages.clear();
                self.log_scroll = 0;
            }
            _ => {}
        }
        Ok(())
    }

    async fn update_state(&mut self) {
        // Update devices
        let mut devices = self.devices.write().await;
        let _ = devices.tick(self.update_interval);
        
        // Check for interrupts
        if let Some(irq) = devices.poll_interrupts() {
            self.add_log(&format!("IRQ {} triggered", irq)).await;
            devices.ack_interrupt(irq);
        }
    }

    async fn add_shell_output(&mut self, line: &str) {
        self.shell_history.push_back(line.to_string());
        if self.shell_history.len() > 1000 {
            self.shell_history.pop_front();
        }
    }

    async fn add_log(&mut self, line: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        self.log_messages.push_back(format!("[{}] {}", timestamp, line));
        if self.log_messages.len() > 500 {
            self.log_messages.pop_front();
        }
    }

    fn render(&mut self, f: &mut Frame) {
        let size = f.size();
        
        // Main layout: tabs at top, content below
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Status bar
            ])
            .split(size);

        // Render tabs
        self.render_tabs(f, chunks[0]);
        
        // Render content based on current tab
        match self.current_tab {
            UITab::Shell => self.render_shell(f, chunks[1]),
            UITab::CPU => self.render_cpu(f, chunks[1]),
            UITab::Memory => self.render_memory(f, chunks[1]),
            UITab::Filesystem => self.render_filesystem(f, chunks[1]),
            UITab::Devices => self.render_devices(f, chunks[1]),
            UITab::Processes => self.render_processes(f, chunks[1]),
            UITab::Network => self.render_network(f, chunks[1]),
            UITab::Logs => self.render_logs(f, chunks[1]),
        }
        
        // Render status bar
        self.render_status_bar(f, chunks[2]);
    }

    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        let tabs = UITab::all();
        let titles: Vec<Span> = tabs.iter()
            .map(|tab| {
                let style = if *tab == self.current_tab {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Span::styled(format!(" {} ", tab.title()), style)
            })
            .collect();

        let tabs_widget = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title(" ZEROPOINT "))
            .select(self.current_tab.index())
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_widget(tabs_widget, area);
    }

    fn render_shell(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // History
                Constraint::Length(3),  // Input line
            ])
            .split(area);

        // History
        let history_items: Vec<ListItem> = self.shell_history.iter()
            .map(|line| ListItem::new(Line::from(line.clone())))
            .collect();
        
        let history = List::new(history_items)
            .block(Block::default().borders(Borders::ALL).title(" Shell "))
            .style(Style::default().fg(Color::Green));
        
        f.render_widget(history, chunks[0]);

        // Input line
        let prompt = self.shell.blocking_read().get_prompt();
        let input_text = format!("{}{}", prompt, self.shell_input);
        let input = Paragraph::new(input_text)
            .block(Block::default().borders(Borders::ALL).title(" Input "))
            .style(Style::default().fg(Color::White));
        
        f.render_widget(input, chunks[1]);
        
        // Set cursor position
        f.set_cursor_position((
            chunks[1].x + prompt.len() as u16 + self.shell_cursor as u16 + 1,
            chunks[1].y + 1,
        ));
    }

    fn render_cpu(&self, f: &mut Frame, area: Rect) {
        let cpu = self.cpu.blocking_read();
        
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Registers
                Constraint::Percentage(50), // Flags & info
            ])
            .split(area);

        // Registers table
        let mut rows = Vec::new();
        for i in 0..16 {
            let reg = crate::cpu::Register::from_u8(i).unwrap();
            let value = cpu.registers[i];
            rows.push(Row::new(vec![
                reg.name(),
                &format!("0x{:08X}", value),
                &format!("{}", value),
            ]));
        }
        
        let regs_table = Table::new(rows, [Constraint::Length(6), Constraint::Length(12), Constraint::Length(12)])
            .header(Row::new(vec!["Reg", "Hex", "Dec"]).style(Style::default().add_modifier(Modifier::BOLD)))
            .block(Block::default().borders(Borders::ALL).title(" Registers "))
            .style(Style::default().fg(Color::Cyan));
        
        f.render_widget(regs_table, chunks[0]);

        // CPU info
        let info_text = vec![
            Line::from(vec![
                Span::styled("PC: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("0x{:08X}", cpu.get_pc())),
            ]),
            Line::from(vec![
                Span::styled("SP: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("0x{:08X}", cpu.get_sp())),
            ]),
            Line::from(vec![
                Span::styled("LR: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("0x{:08X}", cpu.get_lr())),
            ]),
            Line::from(vec![
                Span::styled("Cycles: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", cpu.cycles)),
            ]),
            Line::from(vec![
                Span::styled("Halted: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(if cpu.halted { "Yes" } else { "No" }),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Flags: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw(format!("  N={} Z={} C={} V={}", 
                    cpu.flags.n as u8, cpu.flags.z as u8, cpu.flags.c as u8, cpu.flags.v as u8)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Breakpoints: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", cpu.breakpoints.len())),
            ]),
            Line::from(vec![
                Span::styled("Trace Mode: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(if cpu.trace_mode { "On" } else { "Off" }),
            ]),
        ];
        
        let info = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL).title(" CPU Info "))
            .wrap(Wrap { trim: true });
        
        f.render_widget(info, chunks[1]);
    }

    fn render_memory(&self, f: &mut Frame, area: Rect) {
        let memory = self.memory.blocking_read();
        let stats = memory.get_stats();
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12), // Stats
                Constraint::Min(0),     // Regions
            ])
            .split(area);

        // Memory stats
        let stats_text = vec![
            Line::from(vec![
                Span::styled("Total: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{} KB", stats.total_memory / 1024)),
            ]),
            Line::from(vec![
                Span::styled("Used:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{} KB", stats.used_memory / 1024)),
            ]),
            Line::from(vec![
                Span::styled("Free:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{} KB", stats.free_memory / 1024)),
            ]),
            Line::from(vec![
                Span::styled("Frames: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{} total, {} used, {} free", 
                    stats.total_frames, stats.used_frames, stats.free_frames)),
            ]),
            Line::from(vec![
                Span::styled("Processes: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", stats.num_processes)),
            ]),
        ];
        
        let stats_widget = Paragraph::new(stats_text)
            .block(Block::default().borders(Borders::ALL).title(" Memory Statistics "))
            .wrap(Wrap { trim: true });
        
        f.render_widget(stats_widget, chunks[0]);

        // Process memory regions
        let mut rows = Vec::new();
        for (pid, regions) in memory.regions.iter() {
            for region in regions {
                rows.push(Row::new(vec![
                    format!("PID {}", pid),
                    format!("0x{:08X}", region.base),
                    format!("{} KB", region.size / 1024),
                    format!("{:?}", region.permissions),
                    region.name.clone(),
                ]));
            }
        }
        
        if rows.is_empty() {
            rows.push(Row::new(vec!["-", "-", "-", "-", "No memory regions"]));
        }
        
        let regions_table = Table::new(rows, [
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(20),
        ])
        .header(Row::new(vec!["Process", "Base", "Size", "Perms", "Name"])
            .style(Style::default().add_modifier(Modifier::BOLD)))
        .block(Block::default().borders(Borders::ALL).title(" Memory Regions "))
        .style(Style::default().fg(Color::Magenta));
        
        f.render_widget(regions_table, chunks[1]);
    }

    fn render_filesystem(&mut self, f: &mut Frame, area: Rect) {
        let vfs = self.vfs.blocking_read();
        let entries = vfs.readdir(&self.fs_current_path).unwrap_or_default();
        drop(vfs);
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Path bar
                Constraint::Min(0),     // File list
            ])
            .split(area);

        // Path bar
        let path_text = format!("Path: {}", self.fs_current_path);
        let path_bar = Paragraph::new(path_text)
            .block(Block::default().borders(Borders::ALL).title(" Filesystem "))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(path_bar, chunks[0]);

        // File list
        let items: Vec<ListItem> = entries.iter()
            .map(|entry| {
                let icon = match entry.file_type {
                    crate::vfs::FileType::Directory => "📁 ",
                    crate::vfs::FileType::Regular => "📄 ",
                    crate::vfs::FileType::Symlink => "🔗 ",
                    _ => "❓ ",
                };
                let style = if entry.file_type == crate::vfs::FileType::Directory {
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::styled(format!("{} {}", icon, entry.name), style))
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Contents "))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");
        
        f.render_stateful_widget(list, chunks[1], &mut self.fs_list_state);
    }

    fn render_devices(&mut self, f: &mut Frame, area: Rect) {
        let devices = self.devices.blocking_read();
        let device_list = devices.list_devices().unwrap_or_default();
        drop(devices);
        
        let device_text = Paragraph::new(device_list)
            .block(Block::default().borders(Borders::ALL).title(" Virtual Devices "))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Cyan));
        
        f.render_widget(device_text, area);
    }

    fn render_processes(&self, f: &mut Frame, area: Rect) {
        let scheduler = self.scheduler.blocking_read();
        let processes = scheduler.list_processes();
        let stats = scheduler.get_stats();
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Scheduler stats
                Constraint::Min(0),     // Process list
            ])
            .split(area);

        // Scheduler stats
        let stats_text = vec![
            Line::from(vec![
                Span::styled("Policy: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:?}", scheduler.policy)),
            ]),
            Line::from(vec![
                Span::styled("Time Slice: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:?}", scheduler.time_slice)),
            ]),
            Line::from(vec![
                Span::styled("Context Switches: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", stats.context_switches)),
            ]),
            Line::from(vec![
                Span::styled("Total Processes: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", stats.total_processes)),
            ]),
            Line::from(vec![
                Span::styled("Terminated: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", stats.terminated_processes)),
            ]),
            Line::from(vec![
                Span::styled("Total CPU Time: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:?}", stats.total_cpu_time)),
            ]),
        ];
        
        let stats_widget = Paragraph::new(stats_text)
            .block(Block::default().borders(Borders::ALL).title(" Scheduler Stats "))
            .wrap(Wrap { trim: true });
        
        f.render_widget(stats_widget, chunks[0]);

        // Process table
        let mut rows = Vec::new();
        for p in processes {
            rows.push(Row::new(vec![
                format!("{}", p.pid),
                format!("{}", p.ppid),
                format!("{:?}", p.state),
                format!("{}", p.priority),
                format!("{:?}", p.policy),
                format!("{:?}", p.cpu_time),
                p.name.clone(),
            ]));
        }
        
        let proc_table = Table::new(rows, [
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(15),
            Constraint::Min(20),
        ])
        .header(Row::new(vec!["PID", "PPID", "State", "Pri", "Policy", "CPU Time", "Name"])
            .style(Style::default().add_modifier(Modifier::BOLD)))
        .block(Block::default().borders(Borders::ALL).title(" Processes "))
        .style(Style::default().fg(Color::Green));
        
        f.render_widget(proc_table, chunks[1]);
    }

    fn render_network(&mut self, f: &mut Frame, area: Rect) {
        let devices = self.devices.blocking_read();
        let network = devices.get_device(DeviceType::Network);
        
        if let Some(net) = network {
            if let Some(net_dev) = net.as_any_mut().downcast_ref::<crate::device::NetworkDevice>() {
                let stats = &net_dev.stats;
                let net_text = vec![
                    Line::from(vec![
                        Span::styled("MAC: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 
                            net_dev.mac[0], net_dev.mac[1], net_dev.mac[2], 
                            net_dev.mac[3], net_dev.mac[4], net_dev.mac[5])),
                    ]),
                    Line::from(vec![
                        Span::styled("IP: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}.{}.{}.{}", 
                            net_dev.ip[0], net_dev.ip[1], net_dev.ip[2], net_dev.ip[3])),
                    ]),
                    Line::from(vec![
                        Span::styled("Netmask: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}.{}.{}.{}", 
                            net_dev.netmask[0], net_dev.netmask[1], net_dev.netmask[2], net_dev.netmask[3])),
                    ]),
                    Line::from(vec![
                        Span::styled("Gateway: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}.{}.{}.{}", 
                            net_dev.gateway[0], net_dev.gateway[1], net_dev.gateway[2], net_dev.gateway[3])),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("RX Packets: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", stats.rx_packets)),
                    ]),
                    Line::from(vec![
                        Span::styled("TX Packets: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", stats.tx_packets)),
                    ]),
                    Line::from(vec![
                        Span::styled("RX Bytes: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", stats.rx_bytes)),
                    ]),
                    Line::from(vec![
                        Span::styled("TX Bytes: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", stats.tx_bytes)),
                    ]),
                    Line::from(vec![
                        Span::styled("RX Errors: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", stats.rx_errors)),
                    ]),
                    Line::from(vec![
                        Span::styled("TX Errors: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", stats.tx_errors)),
                    ]),
                    Line::from(vec![
                        Span::styled("RX Dropped: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", stats.rx_dropped)),
                    ]),
                    Line::from(vec![
                        Span::styled("TX Dropped: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", stats.tx_dropped)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("RX Queue: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", net_dev.rx_buffer.len())),
                    ]),
                    Line::from(vec![
                        Span::styled("TX Queue: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", net_dev.tx_buffer.len())),
                    ]),
                ];
                
                let net_widget = Paragraph::new(net_text)
                    .block(Block::default().borders(Borders::ALL).title(" Network "))
                    .wrap(Wrap { trim: true });
                
                f.render_widget(net_widget, area);
            }
        }
    }

    fn render_logs(&self, f: &mut Frame, area: Rect) {
        let start = self.log_scroll.min(self.log_messages.len());
        let end = (start + area.height as usize).min(self.log_messages.len());
        
        let lines: Vec<Line> = self.log_messages.iter()
            .skip(start)
            .take(end - start)
            .map(|msg| Line::from(msg.clone()))
            .collect();
        
        let logs = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" System Logs "))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Gray));
        
        f.render_widget(logs, area);
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let cpu = self.cpu.blocking_read();
        let memory = self.memory.blocking_read();
        let stats = memory.get_stats();
        
        let status = format!(
            "CPU: {} cycles | Mem: {}KB/{}KB | Tab: {} | F1-F8: tabs | Ctrl+C: quit",
            cpu.cycles,
            stats.used_memory / 1024,
            stats.total_memory / 1024,
            self.current_tab.title()
        );
        
        let bar = Paragraph::new(status)
            .style(Style::default().fg(Color::Yellow).bg(Color::Black))
            .alignment(Alignment::Center);
        
        f.render_widget(bar, area);
    }
}