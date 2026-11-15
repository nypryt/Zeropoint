# ZEROPOINT CLI (zp)

Host-side CLI for managing ZEROPOINT VMs and installing guest OSes.

## Installation

```bash
# From the ZEROPOINT repo root
cargo install --path zp-cli
```

Or build and copy binary:
```bash
cargo build --release -p zp-cli
cp target/release/zp ~/.cargo/bin/
```

## Usage

```bash
# Install Ubuntu 24.04
zp install ubuntu

# Install specific version
zp install ubuntu --version 22.04

# Install Alpine Linux
zp install alpine

# Custom VM name and resources
zp install ubuntu --name my-ubuntu --memory 4096 --cpus 4 --disk 50

# List VMs
zp list

# Start VM
zp start my-ubuntu

# Start with host integration
zp start my-ubuntu --host-fs --host-net

# Stop VM
zp stop my-ubuntu

# Configure host integration
zp config my-ubuntu --host-fs true --host-net true --port-forward 2222:22 --mount /home/user/projects:/home/user/projects

# Run command in VM
zp exec my-ubuntu -- ls -la /home

# Remove VM
zp remove my-ubuntu

# Remove VM and disk
zp remove my-ubuntu --purge
```

## Supported Distributions

| Distro | Versions | Default User |
|--------|----------|--------------|
| ubuntu | 24.04, 23.10, 22.04, 20.04, latest | ubuntu |
| alpine | 3.20, 3.19, 3.18, latest, edge | root |
| debian | 12, 11, sid, latest | debian |
| fedora | 40, 39, latest | fedora |
| arch | latest | arch |

## Host Integration

Enable features per-VM:
- `--host-fs` - Access host filesystem via 9p/virtio-fs (mounts at `/mnt/host`)
- `--host-net` - Host network access via port forwarding
- `--port-forward 2222:22` - Forward host port to guest
- `--mount /host/path:/guest/path` - Mount host directory

## Requirements

- `qemu-system-x86_64` and `qemu-img` in PATH
- Linux/macOS/Windows (with WSL2)
- Rust 1.70+ (for building)

## Configuration

Config stored in `~/.zeropoint/`:
```
~/.zeropoint/
├── config.toml          # Global config
├── vms/
│   └── <vm-name>/
│       ├── config.toml  # VM config
│       └── disk.qcow2   # VM disk
└── images/              # Cached base images
```

## Examples

```bash
# Quick Ubuntu dev environment
zp install ubuntu --name dev --host-fs --host-net --port-forward 2222:22
zp start dev
# SSH: ssh -p 2222 ubuntu@localhost

# Alpine minimal container-like VM
zp install alpine --name tiny --memory 512 --disk 2
zp start tiny

# Custom blank VM for own OS
zp create myos --disk 20 --memory 2048
zp start myos --cdrom /path/to/custom.iso
```