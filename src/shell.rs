//! ZEROPOINT Shell
//! 
//! Custom shell for the ZEROPOINT virtual computer with built-in commands.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::cpu::CPU;
use crate::mem::MemoryManager;
use crate::vfs::VFS;
use crate::scheduler::Scheduler;
use crate::device::DeviceManager;
use crate::asm::{Assembler, AssembledProgram};

pub struct Shell {
    cpu: Arc<RwLock<CPU>>,
    memory: Arc<RwLock<MemoryManager>>,
    vfs: Arc<RwLock<VFS>>,
    scheduler: Arc<RwLock<Scheduler>>,
    devices: Arc<RwLock<DeviceManager>>,
    history: Vec<String>,
    history_index: usize,
    prompt: String,
    running: bool,
    builtins: HashMap<String, BuiltinCommand>,
    aliases: HashMap<String, String>,
    variables: HashMap<String, String>,
}

type BuiltinFn = fn(&mut Shell, &[&str]) -> Result<String>;

struct BuiltinCommand {
    name: String,
    description: String,
    usage: String,
    function: BuiltinFn,
}

impl Shell {
    pub fn new(
        cpu: Arc<RwLock<CPU>>,
        memory: Arc<RwLock<MemoryManager>>,
        vfs: Arc<RwLock<VFS>>,
        scheduler: Arc<RwLock<Scheduler>>,
        devices: Arc<RwLock<DeviceManager>>,
    ) -> Self {
        let mut shell = Self {
            cpu,
            memory,
            vfs,
            scheduler,
            devices,
            history: Vec::new(),
            history_index: 0,
            prompt: "zeropoint> ".to_string(),
            running: true,
            builtins: HashMap::new(),
            aliases: HashMap::new(),
            variables: HashMap::new(),
        };

        shell.register_builtins();
        shell.register_aliases();
        
        shell
    }

    fn register_builtins(&mut self) {
        let builtins: Vec<BuiltinCommand> = vec![
            BuiltinCommand {
                name: "help".to_string(),
                description: "Show help for commands".to_string(),
                usage: "help [command]".to_string(),
                function: Self::cmd_help,
            },
            BuiltinCommand {
                name: "exit".to_string(),
                description: "Exit the shell".to_string(),
                usage: "exit [code]".to_string(),
                function: Self::cmd_exit,
            },
            BuiltinCommand {
                name: "clear".to_string(),
                description: "Clear the screen".to_string(),
                usage: "clear".to_string(),
                function: Self::cmd_clear,
            },
            BuiltinCommand {
                name: "echo".to_string(),
                description: "Print arguments".to_string(),
                usage: "echo [args...]".to_string(),
                function: Self::cmd_echo,
            },
            BuiltinCommand {
                name: "pwd".to_string(),
                description: "Print working directory".to_string(),
                usage: "pwd".to_string(),
                function: Self::cmd_pwd,
            },
            BuiltinCommand {
                name: "cd".to_string(),
                description: "Change directory".to_string(),
                usage: "cd [path]".to_string(),
                function: Self::cmd_cd,
            },
            BuiltinCommand {
                name: "ls".to_string(),
                description: "List directory contents".to_string(),
                usage: "ls [path]".to_string(),
                function: Self::cmd_ls,
            },
            BuiltinCommand {
                name: "cat".to_string(),
                description: "Display file contents".to_string(),
                usage: "cat <file>".to_string(),
                function: Self::cmd_cat,
            },
            BuiltinCommand {
                name: "mkdir".to_string(),
                description: "Create directory".to_string(),
                usage: "mkdir <path>".to_string(),
                function: Self::cmd_mkdir,
            },
            BuiltinCommand {
                name: "rmdir".to_string(),
                description: "Remove empty directory".to_string(),
                usage: "rmdir <path>".to_string(),
                function: Self::cmd_rmdir,
            },
            BuiltinCommand {
                name: "rm".to_string(),
                description: "Remove file".to_string(),
                usage: "rm <path>".to_string(),
                function: Self::cmd_rm,
            },
            BuiltinCommand {
                name: "touch".to_string(),
                description: "Create empty file".to_string(),
                usage: "touch <path>".to_string(),
                function: Self::cmd_touch,
            },
            BuiltinCommand {
                name: "cp".to_string(),
                description: "Copy file".to_string(),
                usage: "cp <src> <dst>".to_string(),
                function: Self::cmd_cp,
            },
            BuiltinCommand {
                name: "mv".to_string(),
                description: "Move/rename file".to_string(),
                usage: "mv <src> <dst>".to_string(),
                function: Self::cmd_mv,
            },
            BuiltinCommand {
                name: "stat".to_string(),
                description: "Show file status".to_string(),
                usage: "stat <path>".to_string(),
                function: Self::cmd_stat,
            },
            BuiltinCommand {
                name: "ps".to_string(),
                description: "List processes".to_string(),
                usage: "ps".to_string(),
                function: Self::cmd_ps,
            },
            BuiltinCommand {
                name: "kill".to_string(),
                description: "Kill process".to_string(),
                usage: "kill <pid>".to_string(),
                function: Self::cmd_kill,
            },
            BuiltinCommand {
                name: "mem".to_string(),
                description: "Show memory statistics".to_string(),
                usage: "mem".to_string(),
                function: Self::cmd_mem,
            },
            BuiltinCommand {
                name: "cpu".to_string(),
                description: "Show CPU state".to_string(),
                usage: "cpu".to_string(),
                function: Self::cmd_cpu,
            },
            BuiltinCommand {
                name: "regs".to_string(),
                description: "Show CPU registers".to_string(),
                usage: "regs".to_string(),
                function: Self::cmd_regs,
            },
            BuiltinCommand {
                name: "devices".to_string(),
                description: "List virtual devices".to_string(),
                usage: "devices".to_string(),
                function: Self::cmd_devices,
            },
            BuiltinCommand {
                name: "mount".to_string(),
                description: "Show mount points".to_string(),
                usage: "mount".to_string(),
                function: Self::cmd_mount,
            },
            BuiltinCommand {
                name: "asm".to_string(),
                description: "Assemble and run assembly code".to_string(),
                usage: "asm <file> | -e \"code\"".to_string(),
                function: Self::cmd_asm,
            },
            BuiltinCommand {
                name: "history".to_string(),
                description: "Show command history".to_string(),
                usage: "history [-c]".to_string(),
                function: Self::cmd_history,
            },
            BuiltinCommand {
                name: "alias".to_string(),
                description: "Create command alias".to_string(),
                usage: "alias [name[=value]]".to_string(),
                function: Self::cmd_alias,
            },
            BuiltinCommand {
                name: "unalias".to_string(),
                description: "Remove command alias".to_string(),
                usage: "unalias <name>".to_string(),
                function: Self::cmd_unalias,
            },
            BuiltinCommand {
                name: "set".to_string(),
                description: "Set shell variable".to_string(),
                usage: "set [name[=value]]".to_string(),
                function: Self::cmd_set,
            },
            BuiltinCommand {
                name: "unset".to_string(),
                description: "Unset shell variable".to_string(),
                usage: "unset <name>".to_string(),
                function: Self::cmd_unset,
            },
            BuiltinCommand {
                name: "export".to_string(),
                description: "Export variable to environment".to_string(),
                usage: "export <name>[=value]".to_string(),
                function: Self::cmd_export,
            },
            BuiltinCommand {
                name: "env".to_string(),
                description: "Show environment variables".to_string(),
                usage: "env".to_string(),
                function: Self::cmd_env,
            },
            BuiltinCommand {
                name: "reboot".to_string(),
                description: "Reboot the virtual system".to_string(),
                usage: "reboot".to_string(),
                function: Self::cmd_reboot,
            },
            BuiltinCommand {
                name: "shutdown".to_string(),
                description: "Shutdown the virtual system".to_string(),
                usage: "shutdown".to_string(),
                function: Self::cmd_shutdown,
            },
            BuiltinCommand {
                name: "version".to_string(),
                description: "Show ZEROPOINT version".to_string(),
                usage: "version".to_string(),
                function: Self::cmd_version,
            },
            BuiltinCommand {
                name: "uptime".to_string(),
                description: "Show system uptime".to_string(),
                usage: "uptime".to_string(),
                function: Self::cmd_uptime,
            },
            BuiltinCommand {
                name: "free".to_string(),
                description: "Show free memory".to_string(),
                usage: "free".to_string(),
                function: Self::cmd_free,
            },
            BuiltinCommand {
                name: "df".to_string(),
                description: "Show disk usage".to_string(),
                usage: "df [path]".to_string(),
                function: Self::cmd_df,
            },
            BuiltinCommand {
                name: "whoami".to_string(),
                description: "Show current user".to_string(),
                usage: "whoami".to_string(),
                function: Self::cmd_whoami,
            },
            BuiltinCommand {
                name: "date".to_string(),
                description: "Show current date/time".to_string(),
                usage: "date".to_string(),
                function: Self::cmd_date,
            },
        ];

        for builtin in builtins {
            self.builtins.insert(builtin.name.clone(), builtin);
        }
    }

    fn register_aliases(&mut self) {
        self.aliases.insert("ll".to_string(), "ls -la".to_string());
        self.aliases.insert("la".to_string(), "ls -a".to_string());
        self.aliases.insert("l".to_string(), "ls -CF".to_string());
        self.aliases.insert("..".to_string(), "cd ..".to_string());
        self.aliases.insert("...".to_string(), "cd ../..".to_string());
        self.aliases.insert("h".to_string(), "history".to_string());
        self.aliases.insert("cls".to_string(), "clear".to_string());
    }

    pub async fn run_command(&mut self, input: &str) -> Result<String> {
        let input = input.trim();
        if input.is_empty() {
            return Ok(String::new());
        }

        // Add to history
        self.history.push(input.to_string());
        self.history_index = self.history.len();

        // Expand aliases
        let expanded = self.expand_aliases(input);
        
        // Parse command
        let parts: Vec<&str> = expanded.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(String::new());
        }

        let cmd = parts[0];
        let args = &parts[1..];

        // Check for builtin
        if let Some(builtin) = self.builtins.get(cmd) {
            return (builtin.function)(self, args);
        }

        // Try to execute as program
        self.execute_program(cmd, args).await
    }

    fn expand_aliases(&self, input: &str) -> String {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return input.to_string();
        }
        
        if let Some(expanded) = self.aliases.get(parts[0]) {
            let mut result = expanded.clone();
            if parts.len() > 1 {
                result.push(' ');
                result.push_str(&parts[1..].join(" "));
            }
            return result;
        }
        
        input.to_string()
    }

    async fn execute_program(&mut self, program: &str, args: &[&str]) -> Result<String> {
        // Check if it's a file in the filesystem
        let vfs = self.vfs.read().await;
        let path = if program.starts_with('/') || program.starts_with("./") || program.starts_with("../") {
            program.to_string()
        } else {
            format!("{}/{}", vfs.getcwd(), program)
        };
        
        // Try to open and read the file
        match vfs.open(&path, 0) {
            Ok(fd) => {
                let mut buf = Vec::new();
                let mut temp_buf = [0u8; 4096];
                let mut vfs_mut = self.vfs.write().await;
                loop {
                    let n = vfs_mut.read(fd, &mut temp_buf)?;
                    if n == 0 { break; }
                    buf.extend_from_slice(&temp_buf[..n]);
                }
                let _ = vfs_mut.close(fd);
                drop(vfs_mut);
                
                // Try to assemble and run if it looks like assembly
                if path.ends_with(".asm") || path.ends_with(".s") || 
                   buf.starts_with(b";") || buf.starts_with(b".") || 
                   buf.windows(6).any(|w| w == b"_start") {
                    return self.run_assembly(&String::from_utf8_lossy(&buf)).await;
                }
                
                // Otherwise treat as binary
                Err(anyhow!("Binary execution not yet implemented"))
            }
            Err(_) => Err(anyhow!("Command not found: {}", program)),
        }
    }

    async fn run_assembly(&mut self, source: &str) -> Result<String> {
        let mut assembler = Assembler::new();
        let program = assembler.assemble(source)
            .map_err(|e| anyhow!("Assembly failed: {}", e))?;
        
        // Load into memory
        let mut cpu = self.cpu.write().await;
        let mut memory = self.memory.write().await;
        
        let base_addr = 0x10000u64;
        for (i, &word) in program.code.iter().enumerate() {
            memory.write32(1, base_addr + (i * 4) as u64, word).await?;
        }
        
        // Set PC to entry point
        if let Some(entry) = program.entry_point {
            cpu.set_pc(base_addr as u32 + entry);
        } else {
            cpu.set_pc(base_addr as u32);
        }
        
        cpu.halted = false;
        
        // Run for a limited number of cycles
        let mut output = String::new();
        for _ in 0..100000 {
            if cpu.halted { break; }
            if let Err(e) = cpu.step().await {
                return Err(anyhow!("Execution error: {}", e));
            }
        }
        
        if cpu.halted {
            output.push_str(&format!("Program exited. Cycles: {}", cpu.cycles));
        } else {
            output.push_str("Program stopped (cycle limit reached)");
        }
        
        cpu.dump_registers();
        Ok(output)
    }

    // Builtin command implementations
    fn cmd_help(&mut self, args: &[&str]) -> Result<String> {
        let mut output = String::new();
        
        if args.is_empty() {
            output.push_str("ZEROPOINT Shell - Available Commands:\n\n");
            
            let mut builtins: Vec<_> = self.builtins.values().collect();
            builtins.sort_by_key(|b| b.name.as_str());
            
            for builtin in builtins {
                output.push_str(&format!("  {:<12} - {}\n", builtin.name, builtin.description));
            }
            
            output.push_str("\nAliases:\n");
            for (alias, cmd) in &self.aliases {
                output.push_str(&format!("  {:<8} = {}\n", alias, cmd));
            }
            
            output.push_str("\nType 'help <command>' for more information.\n");
        } else {
            let cmd = args[0];
            if let Some(builtin) = self.builtins.get(cmd) {
                output.push_str(&format!("{}: {}\n", builtin.name, builtin.description));
                output.push_str(&format!("Usage: {}\n", builtin.usage));
            } else if self.aliases.contains_key(cmd) {
                output.push_str(&format!("Alias: {} = {}\n", cmd, self.aliases[cmd]));
            } else {
                return Err(anyhow!("No help for: {}", cmd));
            }
        }
        
        Ok(output)
    }

    fn cmd_exit(&mut self, args: &[&str]) -> Result<String> {
        let code = args.get(0).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
        self.running = false;
        Ok(format!("Exiting with code {}", code))
    }

    fn cmd_clear(&mut self, _args: &[&str]) -> Result<String> {
        Ok("\x1b[2J\x1b[H".to_string())
    }

    fn cmd_echo(&mut self, args: &[&str]) -> Result<String> {
        Ok(args.join(" "))
    }

    fn cmd_pwd(&mut self, _args: &[&str]) -> Result<String> {
        let vfs = self.vfs.blocking_read();
        Ok(vfs.getcwd().to_string())
    }

    fn cmd_cd(&mut self, args: &[&str]) -> Result<String> {
        let path = args.get(0).unwrap_or("/home");
        let mut vfs = self.vfs.blocking_write();
        vfs.chdir(path)?;
        Ok(String::new())
    }

    fn cmd_ls(&mut self, args: &[&str]) -> Result<String> {
        let path = args.get(0).unwrap_or(".");
        let vfs = self.vfs.blocking_read();
        let entries = vfs.readdir(path)?;
        
        let mut output = String::new();
        for entry in entries {
            let type_char = match entry.file_type {
                crate::vfs::FileType::Directory => "d",
                crate::vfs::FileType::Regular => "-",
                crate::vfs::FileType::Symlink => "l",
                _ => "?",
            };
            output.push_str(&format!("{} {:<20} {}\n", type_char, entry.inode, entry.name));
        }
        Ok(output)
    }

    fn cmd_cat(&mut self, args: &[&str]) -> Result<String> {
        let path = args.get(0).ok_or_else(|| anyhow!("Usage: cat <file>"))?;
        let mut vfs = self.vfs.blocking_write();
        let fd = vfs.open(path, 0)?;
        
        let mut output = String::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = vfs.read(fd, &mut buf)?;
            if n == 0 { break; }
            output.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        vfs.close(fd)?;
        Ok(output)
    }

    fn cmd_mkdir(&mut self, args: &[&str]) -> Result<String> {
        let path = args.get(0).ok_or_else(|| anyhow!("Usage: mkdir <path>"))?;
        let mut vfs = self.vfs.blocking_write();
        vfs.mkdir(path, crate::vfs::FileMode::ReadWriteExecute)?;
        Ok(String::new())
    }

    fn cmd_rmdir(&mut self, args: &[&str]) -> Result<String> {
        let path = args.get(0).ok_or_else(|| anyhow!("Usage: rmdir <path>"))?;
        let mut vfs = self.vfs.blocking_write();
        vfs.rmdir(path)?;
        Ok(String::new())
    }

    fn cmd_rm(&mut self, args: &[&str]) -> Result<String> {
        let path = args.get(0).ok_or_else(|| anyhow!("Usage: rm <path>"))?;
        let mut vfs = self.vfs.blocking_write();
        vfs.unlink(path)?;
        Ok(String::new())
    }

    fn cmd_touch(&mut self, args: &[&str]) -> Result<String> {
        let path = args.get(0).ok_or_else(|| anyhow!("Usage: touch <path>"))?;
        let mut vfs = self.vfs.blocking_write();
        let parent = std::path::Path::new(path).parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/");
        let name = std::path::Path::new(path).file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid path"))?;
        let (fs, parent_inode, _) = vfs.resolve_path(parent)?;
        fs.create(parent_inode, name, crate::vfs::FileType::Regular, crate::vfs::FileMode::ReadWrite)?;
        Ok(String::new())
    }

    fn cmd_cp(&mut self, args: &[&str]) -> Result<String> {
        if args.len() != 2 {
            return Err(anyhow!("Usage: cp <src> <dst>"));
        }
        let mut vfs = self.vfs.blocking_write();
        let src_fd = vfs.open(args[0], 0)?;
        let dst_parent = std::path::Path::new(args[1]).parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/");
        let dst_name = std::path::Path::new(args[1]).file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid destination"))?;
        
        let (fs, parent_inode, _) = vfs.resolve_path(dst_parent)?;
        fs.create(parent_inode, dst_name, crate::vfs::FileType::Regular, crate::vfs::FileMode::ReadWrite)?;
        
        let dst_fd = vfs.open(args[1], 1)?;
        
        let mut buf = [0u8; 4096];
        loop {
            let n = vfs.read(src_fd, &mut buf)?;
            if n == 0 { break; }
            vfs.write(dst_fd, &buf[..n])?;
        }
        
        vfs.close(src_fd)?;
        vfs.close(dst_fd)?;
        Ok(String::new())
    }

    fn cmd_mv(&mut self, args: &[&str]) -> Result<String> {
        if args.len() != 2 {
            return Err(anyhow!("Usage: mv <src> <dst>"));
        }
        // Simple implementation: copy then delete
        self.cmd_cp(args)?;
        self.cmd_rm(&[args[0]])?;
        Ok(String::new())
    }

    fn cmd_stat(&mut self, args: &[&str]) -> Result<String> {
        let path = args.get(0).ok_or_else(|| anyhow!("Usage: stat <path>"))?;
        let vfs = self.vfs.blocking_read();
        let meta = vfs.stat(path)?;
        
        Ok(format!(
            "File: {}\nInode: {}\nType: {:?}\nMode: {:?}\nSize: {}\nLinks: {}\nAtime: {}\nMtime: {}\nCtime: {}",
            path, meta.inode, meta.file_type, meta.mode, meta.size, meta.links, meta.atime, meta.mtime, meta.ctime
        ))
    }

    fn cmd_ps(&mut self, _args: &[&str]) -> Result<String> {
        let scheduler = self.scheduler.blocking_read();
        let processes = scheduler.list_processes();
        
        let mut output = String::from("PID    STATE       NAME\n");
        for p in processes {
            output.push_str(&format!("{:<6} {:<12} {}\n", p.pid, format!("{:?}", p.state), p.name));
        }
        Ok(output)
    }

    fn cmd_kill(&mut self, args: &[&str]) -> Result<String> {
        let pid = args.get(0).ok_or_else(|| anyhow!("Usage: kill <pid>"))?.parse::<u32>()?;
        let mut scheduler = self.scheduler.blocking_write();
        scheduler.kill_process(pid)?;
        Ok(format!("Killed process {}", pid))
    }

    fn cmd_mem(&mut self, _args: &[&str]) -> Result<String> {
        let memory = self.memory.blocking_read();
        let stats = memory.get_stats();
        
        Ok(format!(
            "Memory Statistics:\n  Total: {} KB\n  Used:  {} KB\n  Free:  {} KB\n  Frames: {} total, {} used, {} free\n  Processes: {}",
            stats.total_memory / 1024,
            stats.used_memory / 1024,
            stats.free_memory / 1024,
            stats.total_frames, stats.used_frames, stats.free_frames,
            stats.num_processes
        ))
    }

    fn cmd_cpu(&mut self, _args: &[&str]) -> Result<String> {
        let cpu = self.cpu.blocking_read();
        Ok(format!(
            "CPU State:\n  PC:  {:08X}\n  SP:  {:08X}\n  LR:  {:08X}\n  Cycles: {}\n  Halted: {}\n  Flags: N={} Z={} C={} V={}",
            cpu.get_pc(), cpu.get_sp(), cpu.get_lr(), cpu.cycles, cpu.halted,
            cpu.flags.n as u8, cpu.flags.z as u8, cpu.flags.c as u8, cpu.flags.v as u8
        ))
    }

    fn cmd_regs(&mut self, _args: &[&str]) -> Result<String> {
        let cpu = self.cpu.blocking_read();
        let mut output = String::from("Registers:\n");
        for i in 0..16 {
            let reg = crate::cpu::Register::from_u8(i).unwrap();
            output.push_str(&format!("  {}: {:08X} ({})\n", reg.name(), cpu.registers[i], cpu.registers[i]));
        }
        output.push_str(&format!("  Flags: N={} Z={} C={} V={}\n", 
            cpu.flags.n as u8, cpu.flags.z as u8, cpu.flags.c as u8, cpu.flags.v as u8));
        Ok(output)
    }

    fn cmd_devices(&mut self, _args: &[&str]) -> Result<String> {
        let devices = self.devices.blocking_read();
        devices.list_devices()
    }

    fn cmd_mount(&mut self, _args: &[&str]) -> Result<String> {
        let vfs = self.vfs.blocking_read();
        let mounts = vfs.list_mounts();
        
        let mut output = String::from("Mount points:\n");
        for mp in mounts {
            output.push_str(&format!("  {} on {} ({})\n", mp.fs.name(), mp.mount_point, mp.root_inode));
        }
        Ok(output)
    }

    fn cmd_asm(&mut self, args: &[&str]) -> Result<String> {
        if args.is_empty() {
            return Err(anyhow!("Usage: asm <file> | -e \"code\""));
        }
        
        if args[0] == "-e" {
            if args.len() < 2 {
                return Err(anyhow!("Usage: asm -e \"code\""));
            }
            let code = args[1..].join(" ");
            return self.run_assembly(&code).await;
        }
        
        let path = args[0];
        let mut vfs = self.vfs.blocking_write();
        let fd = vfs.open(path, 0)?;
        let mut source = String::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = vfs.read(fd, &mut buf)?;
            if n == 0 { break; }
            source.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        vfs.close(fd)?;
        drop(vfs);
        
        self.run_assembly(&source).await
    }

    fn cmd_history(&mut self, args: &[&str]) -> Result<String> {
        if args.contains(&"-c") {
            self.history.clear();
            self.history_index = 0;
            return Ok("History cleared".to_string());
        }
        
        let mut output = String::new();
        for (i, cmd) in self.history.iter().enumerate() {
            output.push_str(&format!("{:4}  {}\n", i + 1, cmd));
        }
        Ok(output)
    }

    fn cmd_alias(&mut self, args: &[&str]) -> Result<String> {
        if args.is_empty() {
            let mut output = String::new();
            for (alias, cmd) in &self.aliases {
                output.push_str(&format!("alias {}='{}'\n", alias, cmd));
            }
            return Ok(output);
        }
        
        for arg in args {
            if let Some(eq_pos) = arg.find('=') {
                let name = &arg[..eq_pos];
                let value = &arg[eq_pos+1..];
                self.aliases.insert(name.to_string(), value.trim_matches('"').to_string());
            } else {
                if let Some(cmd) = self.aliases.get(arg) {
                    println!("alias {}='{}'", arg, cmd);
                }
            }
        }
        Ok(String::new())
    }

    fn cmd_unalias(&mut self, args: &[&str]) -> Result<String> {
        let name = args.get(0).ok_or_else(|| anyhow!("Usage: unalias <name>"))?;
        self.aliases.remove(name);
        Ok(String::new())
    }

    fn cmd_set(&mut self, args: &[&str]) -> Result<String> {
        if args.is_empty() {
            let mut output = String::new();
            for (k, v) in &self.variables {
                output.push_str(&format!("{}={}\n", k, v));
            }
            return Ok(output);
        }
        
        for arg in args {
            if let Some(eq_pos) = arg.find('=') {
                let name = &arg[..eq_pos];
                let value = &arg[eq_pos+1..];
                self.variables.insert(name.to_string(), value.to_string());
            }
        }
        Ok(String::new())
    }

    fn cmd_unset(&mut self, args: &[&str]) -> Result<String> {
        let name = args.get(0).ok_or_else(|| anyhow!("Usage: unset <name>"))?;
        self.variables.remove(name);
        Ok(String::new())
    }

    fn cmd_export(&mut self, args: &[&str]) -> Result<String> {
        let name = args.get(0).ok_or_else(|| anyhow!("Usage: export <name>[=value]"))?;
        if let Some(eq_pos) = name.find('=') {
            let var_name = &name[..eq_pos];
            let value = &name[eq_pos+1..];
            self.variables.insert(var_name.to_string(), value.to_string());
        } else {
            // Export existing variable
            if let Some(value) = self.variables.get(name).cloned() {
                // In a real shell, this would export to environment
                println!("export {}={}", name, value);
            }
        }
        Ok(String::new())
    }

    fn cmd_env(&mut self, _args: &[&str]) -> Result<String> {
        let mut output = String::new();
        for (k, v) in &self.variables {
            output.push_str(&format!("{}={}\n", k, v));
        }
        Ok(output)
    }

    fn cmd_reboot(&mut self, _args: &[&str]) -> Result<String> {
        let mut cpu = self.cpu.blocking_write();
        cpu.halted = false;
        cpu.cycles = 0;
        cpu.registers = [0; 16];
        cpu.registers[crate::cpu::Register::SP as usize] = 0x00FF_F000;
        cpu.registers[crate::cpu::Register::FP as usize] = 0x00FF_F000;
        Ok("System rebooting...".to_string())
    }

    fn cmd_shutdown(&mut self, _args: &[&str]) -> Result<String> {
        self.running = false;
        Ok("Shutting down...".to_string())
    }

    fn cmd_version(&mut self, _args: &[&str]) -> Result<String> {
        Ok("ZEROPOINT v0.1.0 - Virtual Computer System".to_string())
    }

    fn cmd_uptime(&mut self, _args: &[&str]) -> Result<String> {
        let cpu = self.cpu.blocking_read();
        Ok(format!("Uptime: {} cycles", cpu.cycles))
    }

    fn cmd_free(&mut self, _args: &[&str]) -> Result<String> {
        self.cmd_mem(_args)
    }

    fn cmd_df(&mut self, args: &[&str]) -> Result<String> {
        let path = args.get(0).unwrap_or("/");
        let vfs = self.vfs.blocking_read();
        let meta = vfs.stat(path)?;
        
        Ok(format!(
            "Filesystem     Size  Used  Avail Use%\n{}  {}  {}  {}  0%",
            path, meta.size, meta.blocks, meta.size - meta.blocks
        ))
    }

    fn cmd_whoami(&mut self, _args: &[&str]) -> Result<String> {
        Ok("root".to_string())
    }

    fn cmd_date(&mut self, _args: &[&str]) -> Result<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(format!("{}", chrono::DateTime::from_timestamp(now as i64, 0)
            .unwrap_or_default()
            .format("%a %b %d %H:%M:%S %Y")))
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn get_prompt(&self) -> &str {
        &self.prompt
    }

    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
    }

    pub fn get_history(&self) -> &[String] {
        &self.history
    }

    pub fn get_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }
}