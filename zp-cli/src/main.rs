//! ZEROPOINT Host CLI (zp)
//! 
//! Manage ZEROPOINT VMs and install guest OSes like Ubuntu, Alpine, etc.

use anyhow::{Result, anyhow, Context};
use clap::{Parser, Subcommand, Args};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs as async_fs;

const ZP_DIR: &str = ".zeropoint";
const VM_DIR: &str = "vms";
const IMAGES_DIR: &str = "images";
const CONFIG_FILE: &str = "config.toml";

#[derive(Parser)]
#[command(name = "zp", version, about = "ZEROPOINT VM Manager - Install and run guest OSes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a guest OS (Ubuntu, Alpine, Debian, etc.)
    Install(InstallArgs),
    /// List installed VMs
    List,
    /// Start a VM
    Start(StartArgs),
    /// Stop a VM
    Stop(StopArgs),
    /// Remove a VM
    Remove(RemoveArgs),
    /// Configure host integration for a VM
    Config(ConfigArgs),
    /// Show VM status
    Status(StatusArgs),
    /// Run a command in a VM
    Exec(ExecArgs),
    /// Create a new blank VM
    Create(CreateArgs),
}

#[derive(Args)]
struct InstallArgs {
    /// Distribution to install (ubuntu, alpine, debian, fedora, arch)
    #[arg(value_name = "DISTRO")]
    distro: String,
    /// VM name (defaults to distro name)
    #[arg(short, long)]
    name: Option<String>,
    /// Version/release (e.g., 24.04, latest, edge)
    #[arg(short, long, default_value = "latest")]
    version: String,
    /// Architecture (x86_64, aarch64)
    #[arg(short, long, default_value = "x86_64")]
    arch: String,
    /// Enable host filesystem access
    #[arg(long)]
    host_fs: bool,
    /// Enable host network
    #[arg(long)]
    host_net: bool,
    /// Memory in MB
    #[arg(long, default_value = "2048")]
    memory: usize,
    /// CPU cores
    #[arg(long, default_value = "2")]
    cpus: usize,
    /// Disk size in GB
    #[arg(long, default_value = "20")]
    disk: usize,
}

#[derive(Args)]
struct StartArgs {
    /// VM name
    #[arg(value_name = "NAME")]
    name: String,
    /// Run in background (detached)
    #[arg(short, long)]
    detach: bool,
    /// Enable host filesystem
    #[arg(long)]
    host_fs: bool,
    /// Enable host network
    #[arg(long)]
    host_net: bool,
}

#[derive(Args)]
struct StopArgs {
    /// VM name
    #[arg(value_name = "NAME")]
    name: String,
    /// Force kill
    #[arg(short, long)]
    force: bool,
}

#[derive(Args)]
struct RemoveArgs {
    /// VM name
    #[arg(value_name = "NAME")]
    name: String,
    /// Also remove disk image
    #[arg(long)]
    purge: bool,
}

#[derive(Args)]
struct ConfigArgs {
    /// VM name
    #[arg(value_name = "NAME")]
    name: String,
    /// Enable host filesystem access
    #[arg(long)]
    host_fs: Option<bool>,
    /// Enable host network
    #[arg(long)]
    host_net: Option<bool>,
    /// Set memory (MB)
    #[arg(long)]
    memory: Option<usize>,
    /// Set CPU cores
    #[arg(long)]
    cpus: Option<usize>,
    /// Add port forward (host:guest)
    #[arg(long)]
    port_forward: Vec<String>,
    /// Mount host directory (host_path:vm_path)
    #[arg(long)]
    mount: Vec<String>,
}

#[derive(Args)]
struct StatusArgs {
    /// VM name
    #[arg(value_name = "NAME")]
    name: String,
}

#[derive(Args)]
struct ExecArgs {
    /// VM name
    #[arg(value_name = "NAME")]
    name: String,
    /// Command to run
    #[arg(value_name = "COMMAND", num_args = 1..)]
    command: Vec<String>,
}

#[derive(Args)]
struct CreateArgs {
    /// VM name
    #[arg(value_name = "NAME")]
    name: String,
    /// Base image to use (none, ubuntu, alpine)
    #[arg(short, long, default_value = "none")]
    base: String,
    /// Memory in MB
    #[arg(long, default_value = "1024")]
    memory: usize,
    /// CPU cores
    #[arg(long, default_value = "1")]
    cpus: usize,
    /// Disk size in GB
    #[arg(long, default_value = "10")]
    disk: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct ZpConfig {
    vms: std::collections::HashMap<String, VmConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct VmConfig {
    name: String,
    distro: Option<String>,
    version: Option<String>,
    arch: String,
    memory_mb: usize,
    cpus: usize,
    disk_gb: usize,
    host_fs: bool,
    host_net: bool,
    disk_path: PathBuf,
    port_forwards: Vec<PortForward>,
    mounts: Vec<Mount>,
    created_at: String,
    status: VmStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum VmStatus {
    Created,
    Running,
    Stopped,
    Error(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PortForward {
    host_port: u16,
    guest_port: u16,
    protocol: String, // tcp, udp
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Mount {
    host_path: PathBuf,
    vm_path: PathBuf,
    readonly: bool,
}

// Distro definitions
#[derive(Debug)]
struct DistroInfo {
    name: &'static str,
    versions: &'static [&'static str],
    base_url: &'static str,
    image_pattern: &'static str,
    checksum_url: Option<&'static str>,
    default_version: &'static str,
    default_user: &'static str,
}

const DISTROS: &[DistroInfo] = &[
    DistroInfo {
        name: "ubuntu",
        versions: &["24.04", "23.10", "22.04", "20.04", "latest"],
        base_url: "https://cloud-images.ubuntu.com",
        image_pattern: "/{version}/server/ubuntu-{version}-server-cloudimg-{arch}.img",
        checksum_url: Some("https://cloud-images.ubuntu.com/{version}/server/SHA256SUMS"),
        default_version: "24.04",
        default_user: "ubuntu",
    },
    DistroInfo {
        name: "alpine",
        versions: &["3.20", "3.19", "3.18", "latest", "edge"],
        base_url: "https://dl-cdn.alpinelinux.org/alpine",
        image_pattern: "/v{version}/releases/{arch}/alpine-virt-{version}-{arch}.tar.gz",
        checksum_url: Some("https://dl-cdn.alpinelinux.org/alpine/v{version}/releases/{arch}/alpine-virt-{version}-{arch}.sha256"),
        default_version: "3.20",
        default_user: "root",
    },
    DistroInfo {
        name: "debian",
        versions: &["12", "11", "sid", "latest"],
        base_url: "https://cloud.debian.org/images/cloud",
        image_pattern: "/{version}/debian-{version}-genericcloud-{arch}.qcow2",
        checksum_url: Some("https://cloud.debian.org/images/cloud/{version}/SHA256SUMS"),
        default_version: "12",
        default_user: "debian",
    },
    DistroInfo {
        name: "fedora",
        versions: &["40", "39", "latest"],
        base_url: "https://download.fedoraproject.org/pub/fedora/linux/releases",
        image_pattern: "/{version}/Cloud/{arch}/images/Fedora-Cloud-Base-{version}-1.{arch}.qcow2",
        checksum_url: Some("https://download.fedoraproject.org/pub/fedora/linux/releases/{version}/Cloud/{arch}/images/Fedora-Cloud-Base-{version}-1.{arch}.qcow2.CHECKSUM"),
        default_version: "40",
        default_user: "fedora",
    },
    DistroInfo {
        name: "arch",
        versions: &["latest"],
        base_url: "https://geo.mirror.pkgbuild.com/images",
        image_pattern: "/latest/Arch-Linux-x86_64-cloudimg.qcow2",
        checksum_url: None,
        default_version: "latest",
        default_user: "arch",
    },
];

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize ZP directory
    let zp_dir = get_zp_dir()?;
    fs::create_dir_all(&zp_dir)?;
    fs::create_dir_all(zp_dir.join(VM_DIR))?;
    fs::create_dir_all(zp_dir.join(IMAGES_DIR))?;
    
    match cli.command {
        Commands::Install(args) => install(distro, args).await,
        Commands::List => list_vms(&zp_dir).await,
        Commands::Start(args) => start_vm(&zp_dir, args).await,
        Commands::Stop(args) => stop_vm(&zp_dir, args).await,
        Commands::Remove(args) => remove_vm(&zp_dir, args).await,
        Commands::Config(args) => config_vm(&zp_dir, args).await,
        Commands::Status(args) => status_vm(&zp_dir, args).await,
        Commands::Exec(args) => exec_vm(&zp_dir, args).await,
        Commands::Create(args) => create_vm(&zp_dir, args).await,
    }
}

fn get_zp_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(ZP_DIR))
}

async fn install(zp_dir: &Path, args: InstallArgs) -> Result<()> {
    let distro_name = args.distro.to_lowercase();
    let distro = DISTROS.iter()
        .find(|d| d.name == distro_name)
        .ok_or_else(|| anyhow!("Unknown distro: {}. Supported: ubuntu, alpine, debian, fedora, arch", distro_name))?;
    
    let version = if args.version == "latest" {
        distro.default_version
    } else {
        &args.version
    };
    
    let vm_name = args.name.unwrap_or_else(|| format!("{}-{}", distro.name, version));
    
    println!("Installing {} {} ({})...", distro.name, version, args.arch);
    
    // Download image
    let image_path = download_image(&zp_dir, distro, version, &args.arch).await?;
    
    // Create VM disk
    let vm_dir = zp_dir.join(VM_DIR).join(&vm_name);
    fs::create_dir_all(&vm_dir)?;
    
    let disk_path = vm_dir.join("disk.qcow2");
    create_disk_from_image(&image_path, &disk_path, args.disk).await?;
    
    // Create VM config
    let config = VmConfig {
        name: vm_name.clone(),
        distro: Some(distro.name.to_string()),
        version: Some(version.to_string()),
        arch: args.arch.clone(),
        memory_mb: args.memory,
        cpus: args.cpus,
        disk_gb: args.disk,
        host_fs: args.host_fs,
        host_net: args.host_net,
        disk_path: disk_path.clone(),
        port_forwards: vec![],
        mounts: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
        status: VmStatus::Created,
    };
    
    save_vm_config(&zp_dir, &config)?;
    
    println!("✓ VM '{}' created successfully!", vm_name);
    println!("  Disk: {}", disk_path.display());
    println!("  Memory: {} MB", args.memory);
    println!("  CPUs: {}", args.cpus);
    println!("\nStart with: zp start {}", vm_name);
    
    Ok(())
}

async fn download_image(zp_dir: &Path, distro: &DistroInfo, version: &str, arch: &str) -> Result<PathBuf> {
    let client = Client::new();
    
    // Build URL
    let url = format!("{}{}", 
        distro.base_url,
        distro.image_pattern
            .replace("{version}", version)
            .replace("{arch}", arch)
    );
    
    let images_dir = zp_dir.join(IMAGES_DIR);
    let filename = url.split('/').last().unwrap_or("image");
    let local_path = images_dir.join(filename);
    
    // Check if already downloaded and verified
    if local_path.exists() {
        if let Some(checksum_url) = distro.checksum_url {
            let expected = fetch_checksum(&client, checksum_url, version, arch, filename).await?;
            let actual = compute_sha256(&local_path)?;
            if expected == actual {
                println!("✓ Image already cached and verified");
                return Ok(local_path);
            }
        }
    }
    
    println!("Downloading {}...", url);
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} [{elapsed_precise}] {msg}")?);
    pb.set_message("Downloading...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    
    let response = client.get(&url).send().await?;
    let total_size = response.content_length().unwrap_or(0);
    
    if total_size > 0 {
        pb.set_length(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("█▉▊▋▌▍▎▏  "));
    }
    
    let mut file = File::create(&local_path)?;
    let mut stream = response.bytes_stream();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        pb.inc(chunk.len() as u64);
    }
    
    pb.finish_with_message("Download complete");
    
    // Verify checksum
    if let Some(checksum_url) = distro.checksum_url {
        println!("Verifying checksum...");
        let expected = fetch_checksum(&client, checksum_url, version, arch, filename).await?;
        let actual = compute_sha256(&local_path)?;
        if expected != actual {
            fs::remove_file(&local_path)?;
            return Err(anyhow!("Checksum mismatch! Expected {}, got {}", expected, actual));
        }
        println!("✓ Checksum verified");
    }
    
    Ok(local_path)
}

async fn fetch_checksum(client: &Client, checksum_url: &str, version: &str, arch: &str, filename: &str) -> Result<String> {
    let url = checksum_url
        .replace("{version}", version)
        .replace("{arch}", arch);
    
    let text = client.get(&url).send().await?.text().await?;
    
    for line in text.lines() {
        if line.contains(filename) {
            return Ok(line.split_whitespace().next().unwrap_or("").to_string());
        }
    }
    
    Err(anyhow!("Checksum not found for {}", filename))
}

fn compute_sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    
    Ok(hex::encode(hasher.finalize()))
}

async fn create_disk_from_image(image_path: &Path, disk_path: &Path, size_gb: usize) -> Result<()> {
    // Use qemu-img to convert and resize
    let output = Command::new("qemu-img")
        .args([
            "convert", "-f", "qcow2", "-O", "qcow2",
            image_path.to_str().unwrap(),
            disk_path.to_str().unwrap()
        ])
        .output()?;
    
    if !output.status.success() {
        // Try raw format
        let output = Command::new("qemu-img")
            .args([
                "convert", "-f", "raw", "-O", "qcow2",
                image_path.to_str().unwrap(),
                disk_path.to_str().unwrap()
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow!("Failed to convert image: {}", String::from_utf8_lossy(&output.stderr)));
        }
    }
    
    // Resize
    let output = Command::new("qemu-img")
        .args(["resize", disk_path.to_str().unwrap(), &format!("{}G", size_gb)])
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to resize disk: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

fn save_vm_config(zp_dir: &Path, config: &VmConfig) -> Result<()> {
    let config_path = zp_dir.join(VM_DIR).join(&config.name).join("config.toml");
    let toml = toml::to_string_pretty(config)?;
    fs::write(config_path, toml)?;
    Ok(())
}

fn load_vm_config(zp_dir: &Path, name: &str) -> Result<VmConfig> {
    let config_path = zp_dir.join(VM_DIR).join(name).join("config.toml");
    let toml = fs::read_to_string(config_path)?;
    Ok(toml::from_str(&toml)?)
}

fn load_all_vms(zp_dir: &Path) -> Result<Vec<VmConfig>> {
    let vm_dir = zp_dir.join(VM_DIR);
    let mut vms = Vec::new();
    
    if !vm_dir.exists() {
        return Ok(vms);
    }
    
    for entry in fs::read_dir(vm_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(config) = load_vm_config(zp_dir, &name) {
                vms.push(config);
            }
        }
    }
    
    Ok(vms)
}

async fn list_vms(zp_dir: &Path) -> Result<()> {
    let vms = load_all_vms(zp_dir)?;
    
    if vms.is_empty() {
        println!("No VMs installed. Use 'zp install <distro>' to create one.");
        return Ok(());
    }
    
    println!("{:<20} {:<12} {:<8} {:<8} {:<8} {:<10}", "NAME", "DISTRO", "MEM(MB)", "CPUS", "DISK(GB)", "STATUS");
    println!("{}", "-".repeat(80));
    
    for vm in vms {
        let distro = vm.distro.as_deref().unwrap_or("custom");
        let status = format!("{:?}", vm.status);
        println!("{:<20} {:<12} {:<8} {:<8} {:<8} {:<10}", 
            vm.name, distro, vm.memory_mb, vm.cpus, vm.disk_gb, status);
    }
    
    Ok(())
}

async fn start_vm(zp_dir: &Path, args: StartArgs) -> Result<()> {
    let mut config = load_vm_config(zp_dir, &args.name)?;
    
    // Override with CLI flags
    if args.host_fs { config.host_fs = true; }
    if args.host_net { config.host_net = true; }
    
    config.status = VmStatus::Running;
    save_vm_config(zp_dir, &config)?;
    
    println!("Starting VM '{}'...", args.name);
    
    // Build qemu command
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args([
        "-name", &args.name,
        "-m", &config.memory_mb.to_string(),
        "-smp", &config.cpus.to_string(),
        "-drive", &format!("file={},format=qcow2,if=virtio", config.disk_path.display()),
        "-netdev", "user,id=net0",
        "-device", "virtio-net-pci,netdev=net0",
        "-display", "gtk",
        "-daemonize",
    ]);
    
    // Host filesystem (virtio-9p)
    if config.host_fs {
        cmd.arg("-virtfs").arg("local,path=/home,mount_tag=hostfs,security_model=mapped-xattr");
    }
    
    // Port forwards
    for pf in &config.port_forwards {
        cmd.arg("-netdev").arg(format!("user,id=net{},hostfwd=tcp::{}::{}", 
            pf.host_port, pf.host_port, pf.guest_port));
    }
    
    let output = cmd.output()?;
    
    if !output.status.success() {
        config.status = VmStatus::Error(String::from_utf8_lossy(&output.stderr).to_string());
        save_vm_config(zp_dir, &config)?;
        return Err(anyhow!("Failed to start VM: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    println!("✓ VM '{}' started", args.name);
    println!("  Connect via display window or SSH (if configured)");
    
    Ok(())
}

async fn stop_vm(zp_dir: &Path, args: StopArgs) -> Result<()> {
    let mut config = load_vm_config(zp_dir, &args.name)?;
    
    println!("Stopping VM '{}'...", args.name);
    
    // Try graceful shutdown first
    let output = Command::new("pkill")
        .args(["-f", &format!("qemu-system.*{}", args.name)])
        .output()?;
    
    if !output.status.success() && args.force {
        Command::new("pkill")
            .args(["-9", "-f", &format!("qemu-system.*{}", args.name)])
            .output()?;
    }
    
    config.status = VmStatus::Stopped;
    save_vm_config(zp_dir, &config)?;
    
    println!("✓ VM '{}' stopped", args.name);
    Ok(())
}

async fn remove_vm(zp_dir: &Path, args: RemoveArgs) -> Result<()> {
    let vm_dir = zp_dir.join(VM_DIR).join(&args.name);
    
    if !vm_dir.exists() {
        return Err(anyhow!("VM '{}' not found", args.name));
    }
    
    // Stop if running
    let _ = stop_vm(zp_dir, StopArgs { name: args.name.clone(), force: true }).await;
    
    if args.purge {
        fs::remove_dir_all(&vm_dir)?;
        println!("✓ VM '{}' removed (including disk)", args.name);
    } else {
        // Keep disk, just remove config
        let config_path = vm_dir.join("config.toml");
        fs::remove_file(config_path)?;
        println!("✓ VM '{}' config removed (disk preserved at {})", args.name, vm_dir.join("disk.qcow2").display());
    }
    
    Ok(())
}

async fn config_vm(zp_dir: &Path, args: ConfigArgs) -> Result<()> {
    let mut config = load_vm_config(zp_dir, &args.name)?;
    
    if let Some(v) = args.host_fs { config.host_fs = v; }
    if let Some(v) = args.host_net { config.host_net = v; }
    if let Some(v) = args.memory { config.memory_mb = v; }
    if let Some(v) = args.cpus { config.cpus = v; }
    
    for pf in args.port_forward {
        let parts: Vec<&str> = pf.split(':').collect();
        if parts.len() == 2 {
            config.port_forwards.push(PortForward {
                host_port: parts[0].parse()?,
                guest_port: parts[1].parse()?,
                protocol: "tcp".to_string(),
            });
        }
    }
    
    for mount in args.mount {
        let parts: Vec<&str> = mount.split(':').collect();
        if parts.len() == 2 {
            config.mounts.push(Mount {
                host_path: PathBuf::from(parts[0]),
                vm_path: PathBuf::from(parts[1]),
                readonly: false,
            });
        }
    }
    
    save_vm_config(zp_dir, &config)?;
    
    println!("✓ VM '{}' configured", args.name);
    println!("  Host FS: {}", config.host_fs);
    println!("  Host Net: {}", config.host_net);
    println!("  Memory: {} MB", config.memory_mb);
    println!("  CPUs: {}", config.cpus);
    println!("  Port forwards: {}", config.port_forwards.len());
    println!("  Mounts: {}", config.mounts.len());
    
    Ok(())
}

async fn status_vm(zp_dir: &Path, args: StatusArgs) -> Result<()> {
    let config = load_vm_config(zp_dir, &args.name)?;
    
    println!("VM: {}", config.name);
    println!("  Distro: {} {}", config.distro.as_deref().unwrap_or("custom"), config.version.as_deref().unwrap_or(""));
    println!("  Arch: {}", config.arch);
    println!("  Memory: {} MB", config.memory_mb);
    println!("  CPUs: {}", config.cpus);
    println!("  Disk: {} GB ({})", config.disk_gb, config.disk_path.display());
    println!("  Host FS: {}", config.host_fs);
    println!("  Host Net: {}", config.host_net);
    println!("  Status: {:?}", config.status);
    println!("  Created: {}", config.created_at);
    
    if !config.port_forwards.is_empty() {
        println!("  Port forwards:");
        for pf in &config.port_forwards {
            println!("    {}:{} -> {} ({})", pf.host_port, pf.guest_port, pf.guest_port, pf.protocol);
        }
    }
    
    if !config.mounts.is_empty() {
        println!("  Mounts:");
        for m in &config.mounts {
            println!("    {} -> {} ({})", m.host_path.display(), m.vm_path.display(), if m.readonly { "ro" } else { "rw" });
        }
    }
    
    Ok(())
}

async fn exec_vm(zp_dir: &Path, args: ExecArgs) -> Result<()> {
    let config = load_vm_config(zp_dir, &args.name)?;
    
    if !matches!(config.status, VmStatus::Running) {
        return Err(anyhow!("VM '{}' is not running", args.name));
    }
    
    // Use QEMU guest agent or SSH to execute command
    let command = args.command.join(" ");
    
    // For now, just show the command that would be run
    println!("Would execute in VM '{}': {}", args.name, command);
    println!("(Full implementation requires QEMU guest agent or SSH setup)");
    
    Ok(())
}

async fn create_vm(zp_dir: &Path, args: CreateArgs) -> Result<()> {
    let vm_dir = zp_dir.join(VM_DIR).join(&args.name);
    
    if vm_dir.exists() {
        return Err(anyhow!("VM '{}' already exists", args.name));
    }
    
    fs::create_dir_all(&vm_dir)?;
    
    let disk_path = vm_dir.join("disk.qcow2");
    
    // Create empty disk
    let output = Command::new("qemu-img")
        .args(["create", "-f", "qcow2", disk_path.to_str().unwrap(), &format!("{}G", args.disk)])
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to create disk: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    let config = VmConfig {
        name: args.name.clone(),
        distro: None,
        version: None,
        arch: "x86_64".to_string(),
        memory_mb: args.memory,
        cpus: args.cpus,
        disk_gb: args.disk,
        host_fs: false,
        host_net: false,
        disk_path,
        port_forwards: vec![],
        mounts: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
        status: VmStatus::Created,
    };
    
    save_vm_config(zp_dir, &config)?;
    
    println!("✓ Blank VM '{}' created", args.name);
    println!("  Disk: {} GB", args.disk);
    println!("  Memory: {} MB", args.memory);
    println!("  CPUs: {}", args.cpus);
    println!("\nInstall an OS with: zp start {} --cdrom /path/to/iso", args.name);
    
    Ok(())
}