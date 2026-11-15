# ZEROPOINT Device Emulation

## Overview

ZEROPOINT emulates a complete set of virtual hardware devices through memory-mapped I/O. Each device has a fixed base address in the kernel space and communicates with the CPU through register reads/writes and interrupts.

## Device Address Map

```
0xFF00_0000 - 0xFF00_0FFF  Keyboard Controller (4KB)   IRQ 1
0xFF01_0000 - 0xFF01_FFFF  Display Controller (64KB)   IRQ 2
0xFF02_0000 - 0xFF02_0FFF  Disk Controller (4KB)       IRQ 3
0xFF03_0000 - 0xFF03_0FFF  Network Controller (4KB)    IRQ 4
0xFF04_0000 - 0xFF04_0FFF  Timer (4KB)                 IRQ 5
0xFF05_0000 - 0xFF05_00FF  RTC (256B)                  IRQ 6
0xFF80_0000 - 0xFFFF_FFFF  Framebuffer / Device Memory
```

## Device Interface

All devices implement the `Device` trait:

```rust
trait Device: Send + Sync {
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
```

## Standard Register Layout

Each device has a common register set at fixed offsets:

| Offset | Register | Description |
|--------|----------|-------------|
| 0x00 | Data | Data port (device-specific) |
| 0x04 | Status | Device status flags |
| 0x08 | Control | Control/configuration |
| 0x0C | Interrupt Status | Pending interrupts |
| 0x10 | Interrupt Enable | Enabled interrupts |
| 0x14 | Address | Address register (if applicable) |
| 0x18 | DMA Address | DMA source/destination |
| 0x1C | DMA Count | DMA transfer count |

## Keyboard Controller (PS/2 Style)

### Base Address: 0xFF00_0000
### Size: 4KB
### IRQ: 1

### Registers

| Offset | Name | Access | Description |
|--------|------|--------|-------------|
| 0x00 | Data | R | Scancode buffer (bit 15 = break code) |
| 0x04 | Status | R | Bit 0 = output buffer full |
| 0x04 | Command | W | Controller commands |
| 0x08 | Control | R/W | Controller configuration |
| 0x0C | IRQ Status | R/W | Interrupt status (write to clear) |
| 0x10 | IRQ Enable | R/W | Interrupt enable mask |

### Commands (write to 0x04)

| Command | Description |
|---------|-------------|
| 0xED | Set LEDs (next byte = LED state) |
| 0xEE | Echo (returns 0xEE) |
| 0xF0 | Select scan code set (next byte = 1/2/3) |
| 0xF4 | Enable scanning |
| 0xF5 | Disable scanning |
| 0xFF | Reset keyboard |

### LED Control
- Bit 0: Scroll Lock
- Bit 1: Num Lock
- Bit 2: Caps Lock

### Scancodes
- Make codes: 0x00-0x7F (key press)
- Break codes: 0x80-0xFF (key release, bit 7 set)
- Extended codes: Prefixed with 0xE0

### Programming Example
```assembly
; Wait for key press
wait_key:
    LDR R0, [R1, #4]      ; Load status
    TSTI R0, #1           ; Check output buffer full
    BEQ wait_key          ; Wait if empty
    
    LDR R0, [R1, #0]      ; Read scancode
    ; Process scancode...
```

## Display Controller (VGA Style)

### Base Address: 0xFF01_0000
### Size: 64KB
### IRQ: 2 (VSync)

### Registers

| Offset | Name | Access | Description |
|--------|------|--------|-------------|
| 0x00 | Width | R | Horizontal resolution (1024) |
| 0x04 | Height | R | Vertical resolution (768) |
| 0x08 | BPP | R | Bits per pixel (32) |
| 0x0C | FB Address | R/W | Framebuffer physical address |
| 0x10 | Control | R/W | Bit 0 = swap buffers |
| 0x14 | Status | R | Bit 0 = VSync pending |
| 0x18 | IRQ Status | R/W | Interrupt status |
| 0x1C | IRQ Enable | R/W | Interrupt enable |
| 0x20 | Cursor X | R/W | Hardware cursor X position |
| 0x24 | Cursor Y | R/W | Hardware cursor Y position |

### Framebuffer
- Located at 0xFF80_0000 (configurable via FB Address register)
- 1024 × 768 × 4 bytes = 3,145,728 bytes
- Double-buffered (write to back, swap on VSync)
- Format: 32-bit BGRA (little-endian)

### VSync Interrupt
- Generated at 60Hz (every ~16.67ms)
- Automatically swaps buffers if control bit 0 set
- IRQ 2 raised on VSync

### Programming Example
```assembly
; Set pixel at (100, 100) to red (0x00FF0000)
MOVI R0, #0xFF800000    ; Framebuffer base
MOVI R1, #100           ; X
MOVI R2, #100           ; Y
MOVI R3, #0x00FF0000    ; Red color
; Calculate offset: (Y * width + X) * 4
MULI R4, R2, #1024      ; Y * width
ADD R4, R4, R1          ; + X
LSLI R4, R4, #2         ; * 4 (bytes per pixel)
ADD R0, R0, R4          ; Framebuffer + offset
STR R3, [R0, #0]        ; Write pixel

; Swap buffers
MOVI R0, #0xFF010010    ; Control register
MOVI R1, #1             ; Swap bit
STR R1, [R0, #0]
```

## Disk Controller (VirtIO Block Style)

### Base Address: 0xFF02_0000
### Size: 4KB
### IRQ: 3

### Registers

| Offset | Name | Access | Description |
|--------|------|--------|-------------|
| 0x00 | Data | R/W | PIO data port |
| 0x04 | Status | R | 0x1=busy, 0x2=ready, 0x4=error |
| 0x08 | Sector Size | R | 512 bytes |
| 0x0C | Sector Count Low | R | Total sectors (low 32) |
| 0x10 | Sector Count High | R | Total sectors (high 32) |
| 0x14 | Sector Address | R/W | Current sector number |
| 0x18 | Control | R/W | Command register |
| 0x1C | IRQ Status | R/W | Interrupt status |
| 0x20 | IRQ Enable | R/W | Interrupt enable |
| 0x24 | DMA Address | R/W | DMA buffer address |
| 0x28 | DMA Count | R/W | DMA sector count |

### Commands (write to 0x18)

| Command | Value | Description |
|---------|-------|-------------|
| Read | 0x20 | Read sector(s) |
| Write | 0x30 | Write sector(s) |
| Identify | 0xEC | Return device info |
| Flush | 0xE7 | Flush cache |

### Identify Data (after 0xEC command)
- Data port returns device signature: 0x0040 (non-magnetic)
- Full identify structure in DMA buffer (future)

### Disk Geometry
- 64MB virtual disk
- 512-byte sectors
- 131,072 sectors total
- LBA addressing

### Programming Example
```assembly
; Read sector 100 into buffer at 0x20000
MOVI R0, #0xFF020014    ; Sector address register
MOVI R1, #100           ; Sector 100
STR R1, [R0, #0]

MOVI R0, #0xFF020024    ; DMA address
MOVI R1, #0x20000       ; Buffer address
STR R1, [R0, #0]

MOVI R0, #0xFF020028    ; DMA count
MOVI R1, #1             ; 1 sector
STR R1, [R0, #0]

MOVI R0, #0xFF020018    ; Control
MOVI R1, #0x20          ; Read command
STR R1, [R0, #0]

; Wait for interrupt (IRQ 3)
```

## Network Controller (VirtIO Net Style)

### Base Address: 0xFF03_0000
### Size: 4KB
### IRQ: 4

### Registers

| Offset | Name | Access | Description |
|--------|------|--------|-------------|
| 0x00 | MAC Low | R | MAC[0:3] |
| 0x04 | MAC High | R | MAC[4:5] |
| 0x08 | IP Address | R/W | IPv4 address |
| 0x0C | Netmask | R/W | Subnet mask |
| 0x10 | Gateway | R/W | Default gateway |
| 0x14 | Status | R | Bit 0=RX ready, Bit 1=TX ready |
| 0x18 | Control | R/W | Bit 0=RX enable, Bit 1=TX enable |
| 0x1C | IRQ Status | R/W | Bit 0=RX, Bit 1=TX |
| 0x20 | IRQ Enable | R/W | Interrupt enable |
| 0x24 | RX Queue Len | R | Packets in RX queue |
| 0x28 | TX Queue Len | R | Packets in TX queue |
| 0x2C | RX Packets | R | Total RX packets |
| 0x30 | TX Packets | R | Total TX packets |
| 0x34 | RX Bytes | R | Total RX bytes |
| 0x38 | TX Bytes | R | Total TX bytes |
| 0x3C | TX Data | W | Transmit packet data |

### Default Configuration
- MAC: 52:54:00:12:34:56
- IP: 10.0.2.15
- Netmask: 255.255.255.0
- Gateway: 10.0.2.2

### Packet Format
- Simple raw Ethernet frames
- No checksum offload (software)
- Max frame size: 1518 bytes

### Programming Example
```assembly
; Send packet
MOVI R0, #0xFF030018    ; Control
MOVI R1, #2             ; Enable TX
STR R1, [R0, #0]

; Write packet data to TX Data port (0x3C)
; (In practice, use DMA)

; Wait for TX interrupt (IRQ 4)
```

## Timer (HPET Style)

### Base Address: 0xFF04_0000
### Size: 4KB
### IRQ: 5

### Registers

| Offset | Name | Access | Description |
|--------|------|--------|-------------|
| 0x00 | Counter Low | R | Current counter (low 32) |
| 0x04 | Counter High | R | Current counter (high 32) |
| 0x08 | Compare Low | R/W | Compare value (low 32) |
| 0x0C | Compare High | R/W | Compare value (high 32) |
| 0x10 | Control | R/W | Bit 0=enable, Bit 1=periodic |
| 0x14 | Status | R | Timer status |
| 0x18 | IRQ Status | R/W | Bit 0=compare match |
| 0x1C | IRQ Enable | R/W | Interrupt enable |

### Modes

#### One-Shot
```
MOVI R0, #0xFF040010    ; Control
MOVI R1, #1             ; Enable
STR R1, [R0, #0]

MOVI R0, #0xFF040008    ; Compare
MOVI R1, #1000000       ; 1 second at 1MHz
STR R1, [R0, #0]
```

#### Periodic
```
MOVI R0, #0xFF040010    ; Control
MOVI R1, #3             ; Enable + Periodic
STR R1, [R0, #0]

MOVI R0, #0xFF040008    ; Compare
MOVI R1, #100000        ; 100Hz at 1MHz
STR R1, [R0, #0]
```

### Frequency
- Default: 1 MHz (1 microsecond resolution)
- Configurable via frequency register (future)

## RTC (CMOS Style)

### Base Address: 0xFF05_0000
### Size: 256B
### IRQ: 6 (periodic, alarm, update-ended)

### Registers

| Offset | Name | Access | Description |
|--------|------|--------|-------------|
| 0x00 | Seconds | R | 0-59 |
| 0x04 | Minutes | R | 0-59 |
| 0x08 | Hours | R | 0-23 |
| 0x0C | Days | R | Days since epoch |
| 0x10 | Control | R/W | RTC control |
| 0x14 | Status | R | RTC status |

### Time Source
- Based on host system time
- Monotonic from boot (not wall clock)
- No battery backup simulation

## DMA Controller

### Channels
- 8 DMA channels (0-7)
- Each channel: source, destination, count, direction
- Memory-to-memory, device-to-memory, memory-to-device

### Channel Registers (per channel)
```
DMA_SRC_LOW    = 0x00 + ch * 0x10
DMA_SRC_HIGH   = 0x04 + ch * 0x10
DMA_DST_LOW    = 0x08 + ch * 0x10
DMA_DST_HIGH   = 0x0C + ch * 0x10
DMA_COUNT      = 0x10 + ch * 0x10
DMA_CONTROL    = 0x14 + ch * 0x10
DMA_STATUS     = 0x18 + ch * 0x10
```

### Control Bits
- Bit 0: Enable
- Bit 1: Direction (0=mem->dev, 1=dev->mem)
- Bit 2: Increment source
- Bit 3: Increment destination
- Bit 4: Interrupt on complete

## Interrupt Controller

### IRQ Assignments
| IRQ | Device | Priority |
|-----|--------|----------|
| 0 | Timer (system) | Highest |
| 1 | Keyboard | High |
| 2 | Display (VSync) | High |
| 3 | Disk | Medium |
| 4 | Network | Medium |
| 5 | Timer (user) | Low |
| 6 | RTC | Low |
| 7-15 | Reserved | - |

### Interrupt Handling Flow
1. Device raises interrupt (sets IRQ Status bit)
2. If enabled in IRQ Enable, interrupt controller raises CPU IRQ
3. CPU vectors to interrupt handler
4. Handler reads IRQ Status to determine source
5. Handler acknowledges (clears IRQ Status)
6. Handler processes device
7. Return with RETI

### Programming Interrupts
```assembly
; Enable keyboard interrupt
MOVI R0, #0xFF000010    ; Keyboard IRQ Enable
MOVI R1, #1             ; Enable bit 0
STR R1, [R0, #0]

; Enable timer interrupt
MOVI R0, #0xFF04001C    ; Timer IRQ Enable
MOVI R1, #1
STR R1, [R0, #0]

; Global interrupt enable (CPU)
; (Done via SVC or privileged instruction)
```

## Device Programming from Shell

### Device Commands
```bash
devices               # List all devices
mount                 # Show device mount points (VFS)
```

### UI Device Tab
The Devices tab in the UI shows real-time device status:
- Register values
- Interrupt counts
- Queue depths
- Statistics

## Device Files (Future /dev/)

| Device | Major | Minor | Path | Description |
|--------|-------|-------|------|-------------|
| Keyboard | 10 | 0 | /dev/kbd | Raw keyboard input |
| Display | 29 | 0 | /dev/fb0 | Framebuffer |
| Disk | 1 | 0 | /dev/vda | Virtual disk |
| Network | 1 | 0 | /dev/net0 | Network interface |
| Timer | 10 | 1 | /dev/timer0 | Timer |
| RTC | 10 | 2 | /dev/rtc0 | RTC |
| Null | 1 | 3 | /dev/null | Discard writes |
| Zero | 1 | 5 | /dev/zero | Infinite zeros |
| Random | 1 | 8 | /dev/random | Random bytes |

## Future Devices

### Planned
- **Audio**: PCM playback, mixer, MIDI
- **Mouse**: PS/2 or USB mouse
- **GPIO**: General-purpose I/O
- **USB**: USB host controller
- **PCI**: PCI bus enumeration
- **SD/MMC**: SD card controller
- **SPI/I2C**: Serial buses
- **Watchdog**: Hardware watchdog timer
- **RNG**: Hardware random number generator
- **Crypto**: AES/SHA accelerator

### Device Tree (Future)
```
/devices/
├── cpu@0
├── memory@0
├── interrupt-controller@0
├── timer@0
├── rtc@0
├── keyboard@0
├── display@0
├── disk@0
├── network@0
├── audio@0
├── gpio@0
└── pci@0
```

## Testing Devices

### Keyboard Test
```bash
# In shell tab, type keys
# Check UI Devices tab for scancodes
```

### Display Test
```bash
asm -e "
    MOVI R0, #0xFF800000
    MOVI R1, #0xFF0000
    MOVI R2, #1024*768
loop:
    STR R1, [R0], #4
    SUBI R2, R2, #1
    BNE loop
    ; Swap buffers
    MOVI R0, #0xFF010010
    MOVI R1, #1
    STR R1, [R0, #0]
"
```

### Disk Test
```bash
asm -e "
    ; Write sector 0
    MOVI R0, #0xFF020014
    MOVI R1, #0
    STR R1, [R0, #0]
    MOVI R0, #0xFF020024
    MOVI R1, #0x20000
    STR R1, [R0, #0]
    MOVI R0, #0xFF020028
    MOVI R1, #1
    STR R1, [R0, #0]
    MOVI R0, #0xFF020018
    MOVI R1, #0x30
    STR R1, [R0, #0]
"
```

### Timer Test
```bash
asm -e "
    ; Set periodic 100Hz timer
    MOVI R0, #0xFF040010
    MOVI R1, #3
    STR R1, [R0, #0]
    MOVI R0, #0xFF040008
    MOVI R1, #10000
    STR R1, [R0, #0]
    MOVI R0, #0xFF04001C
    MOVI R1, #1
    STR R1, [R0, #0]
"
```

## Device Emulation Accuracy

| Device | Accuracy Level | Notes |
|--------|---------------|-------|
| Keyboard | High | PS/2 compatible scancodes |
| Display | Medium | VGA-like, double-buffered |
| Disk | High | VirtIO Block compatible |
| Network | Medium | VirtIO Net compatible |
| Timer | High | HPET-like, 1µs resolution |
| RTC | Low | Host-time based |

## Performance

| Operation | Latency |
|-----------|---------|
| Register read/write | <100ns |
| Interrupt delivery | <1µs |
| Disk sector read | ~10µs |
| Network packet | ~50µs |
| Timer interrupt | Exact (1µs) |
| VSync | 16.67ms (60Hz) |

## Security

- Devices isolated in kernel space
- No direct host hardware access
- Network packets filtered (future)
- Disk writes contained to virtual disk
- No DMA to host memory