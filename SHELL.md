# ZEROPOINT Shell

## Overview

The ZEROPOINT shell (`zsh`) is a feature-rich command-line interpreter designed for the ZEROPOINT virtual computer. It provides 40+ built-in commands, command history, aliases, variables, and integration with all subsystems.

## Features

- **Built-in Commands**: 40+ commands covering filesystem, process, memory, device, and system management
- **Command History**: Up to 1000 entries with up/down navigation
- **Aliases**: Command shortcuts with argument support
- **Variables**: Shell variables with export capability
- **Tab Completion**: (Future) File and command completion
- **Scripting**: (Future) Shell script execution
- **Pipeline**: (Future) Command piping and redirection

## Command Reference

### Filesystem Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `ls` | `ls [path]` | List directory contents |
| `cd` | `cd [path]` | Change directory |
| `pwd` | `pwd` | Print working directory |
| `cat` | `cat <file>` | Display file contents |
| `mkdir` | `mkdir <path>` | Create directory |
| `rmdir` | `rmdir <path>` | Remove empty directory |
| `rm` | `rm <path>` | Remove file |
| `touch` | `touch <path>` | Create empty file |
| `cp` | `cp <src> <dst>` | Copy file |
| `mv` | `mv <src> <dst>` | Move/rename file |
| `stat` | `stat <path>` | Show file metadata |
| `find` | `find <name> [path]` | Find file by name |

### Process Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `ps` | `ps` | List processes |
| `kill` | `kill <pid>` | Kill process |

### Memory Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `mem` | `mem` | Show memory statistics |
| `cpu` | `cpu` | Show CPU state |
| `regs` | `regs` | Show CPU registers |
| `free` | `free` | Show free memory (alias for mem) |

### Device Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `devices` | `devices` | List virtual devices |
| `mount` | `mount` | Show mount points |

### System Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `reboot` | `reboot` | Reboot virtual system |
| `shutdown` | `shutdown` | Shutdown virtual system |
| `version` | `version` | Show ZEROPOINT version |
| `uptime` | `uptime` | Show system uptime |
| `date` | `date` | Show current date/time |
| `whoami` | `whoami` | Show current user |
| `df` | `df [path]` | Show disk usage |

### Environment Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `set` | `set [name[=value]]` | Set/show shell variables |
| `unset` | `unset <name>` | Unset shell variable |
| `export` | `export <name>[=value]` | Export variable |
| `env` | `env` | Show environment variables |
| `alias` | `alias [name[=value]]` | Create/show aliases |
| `unalias` | `unalias <name>` | Remove alias |
| `history` | `history [-c]` | Show/clear command history |
| `help` | `help [command]` | Show help |
| `clear` | `clear` | Clear screen |
| `echo` | `echo [args...]` | Print arguments |

### Assembly Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `asm` | `asm <file.asm>` | Assemble and run file |
| `asm` | `asm -e "code"` | Assemble and run inline |

## Default Aliases

```bash
alias ll='ls -la'
alias la='ls -a'
alias l='ls -CF'
alias ..='cd ..'
alias ...='cd ../..'
alias h='history'
alias cls='clear'
```

## Variables

### Setting Variables
```bash
set VAR=value        # Set variable
set VAR              # Show variable
set                  # Show all variables
```

### Exporting Variables
```bash
export VAR=value     # Set and export
export VAR           # Export existing variable
```

### Special Variables
- `PS1` - Primary prompt (default: `zeropoint> `)
- `HOME` - Home directory
- `PATH` - Command search path (future)
- `TERM` - Terminal type

## Command History

### Navigation
- `Up` / `Down` - Navigate history
- `history` - Show all history
- `history -c` - Clear history

### History Expansion (Future)
- `!!` - Previous command
- `!n` - Command n
- `!string` - Last command starting with string
- `^old^new^` - Substitute in previous command

## Prompt Customization

```bash
set PS1='[ZEROPOINT] $ '
set PS1='\u@\h:\w$ '    # Future: username@host:path
```

## Input/Output

### Standard Streams
- `stdin` (fd 0) - Keyboard input
- `stdout` (fd 1) - Display output
- `stderr` (fd 2) - Display output (errors)

### Redirection (Future)
```bash
cmd > file        # Redirect stdout
cmd 2> file       # Redirect stderr
cmd >> file       # Append stdout
cmd < file        # Redirect stdin
cmd1 | cmd2       # Pipe
```

## Assembly Integration

### Running Assembly Files
```bash
asm hello.asm           # Run assembly file
asm program.s           # .s extension also works
```

### Inline Assembly
```bash
asm -e "MOVI R0, #42; HLT"    # Direct code execution
asm -e "
    MOVI R0, #1
    MOVI R1, #msg
    MOVI R2, #13
    MOVI R7, #1
    SVC
    MOVI R7, #0
    SVC
    msg: .ascii \"Hello\n\"
"
```

## Built-in Command Details

### help
```
help                    # Show all commands
help <command>          # Show help for specific command
help alias              # Show alias help
```

### cd
```bash
cd                      # Go to home directory
cd /path/to/dir         # Go to absolute path
cd ../dir               # Relative path
cd -                    # Previous directory (future)
```

### ls
```bash
ls                      # Current directory
ls /path                # Specific path
ls -l                   # Long format (future)
ls -a                   # Show hidden (future)
```

### cat
```bash
cat file.txt            # Display file
cat file1 file2         # Concatenate multiple
```

### mkdir
```bash
mkdir dirname           # Create single directory
mkdir -p a/b/c          # Create parents (future)
```

### cp/mv
```bash
cp src dst              # Copy file
cp -r src dst           # Copy directory (future)
mv old new              # Move/rename
```

### ps
```bash
ps                      # List all processes
```
Output:
```
PID    STATE       NAME
1      Running     init
2      Ready       scheduler
3      Running     cpu
4      Ready       shell
```

### kill
```bash
kill 123              # Send SIGTERM
kill -9 123           # Send SIGKILL (future)
```

### mem/free/cpu/regs
```bash
mem                   # Memory statistics
free                  # Same as mem
cpu                   # CPU state (PC, SP, LR, cycles)
regs                  # All registers + flags
```

### devices
```bash
devices               # List all virtual devices
```
Output:
```
Virtual Devices:
Name                 Type         Base       Size       State
Keyboard             Keyboard     0xFF000000 0x00001000 Ready
Display              Display      0xFF010000 0x00010000 Ready
Disk                 Disk         0xFF020000 0x00001000 Ready
Network              Network      0xFF030000 0x00001000 Ready
Timer                Timer        0xFF040000 0x00001000 Ready
RTC                  RTC          0xFF050000 0x00000100 Ready
```

### mount
```bash
mount                 # Show mount points
```
Output:
```
Mount points:
inmem on / (1)
```

### version/uptime/date/whoami
```bash
version               # ZEROPOINT v0.1.0
uptime                # Uptime: 12345 cycles
date                  # Mon Nov 15 10:30:45 2026
whoami                # root
```

### env/set/unset/export
```bash
env                   # Show all environment variables
set FOO=bar           # Set shell variable
unset FOO             # Remove variable
export FOO            # Export to environment
```

### alias/unalias
```bash
alias                 # Show all aliases
alias ll='ls -la'     # Create alias
unalias ll            # Remove alias
```

### history
```bash
history               # Show command history
history -c            # Clear history
```

### reboot/shutdown
```bash
reboot                # Restart virtual system
shutdown              # Shutdown (exit ZEROPOINT)
```

## Keyboard Shortcuts (in Shell Tab)

| Key | Action |
|-----|--------|
| `Enter` | Execute command |
| `Up`/`Down` | History navigation |
| `Left`/`Right` | Cursor movement |
| `Home`/`End` | Line start/end |
| `Backspace` | Delete previous char |
| `Delete` | Delete current char |
| `Ctrl+C` | Exit ZEROPOINT |
| `Tab` | Next tab (global) |
| `F1`-`F8` | Jump to tab |

## Scripting (Future)

### Shebang
```bash
#!/usr/bin/env zsh
# ZEROPOINT shell script
```

### Control Structures
```bash
if [ condition ]; then
    commands
elif [ condition ]; then
    commands
else
    commands
fi

for var in list; do
    commands
done

while [ condition ]; do
    commands
done
```

### Functions
```bash
function name {
    commands
}
```

## Integration with Subsystems

### CPU Control
```bash
cpu                   # View CPU state
regs                  # View registers
# Future: cpu step, cpu run, cpu break
```

### Memory Control
```bash
mem                   # View statistics
# Future: mem map, mem dump, mem fill
```

### Filesystem Control
```bash
# All filesystem operations through standard commands
# Future: fsck, mount, umount, format
```

### Device Control
```bash
devices               # View all devices
# Future: device read, device write, device reset
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid syntax |
| 3 | Command not found |
| 4 | Permission denied |
| 5 | Not found |
| 126 | Not executable |
| 127 | Command not found |
| 128+n | Killed by signal n |

## Configuration (Future)

### Config File
`~/.zshrc` - User configuration
`/etc/zshrc` - System configuration

### Settings
- History size
- Prompt format
- Default aliases
- Key bindings
- Completion behavior

## Differences from POSIX Shells

| Feature | ZEROPOINT | POSIX |
|---------|-----------|-------|
| Built-in commands | 40+ | ~30 |
| Arrays | No | Yes (bash) |
| Associative arrays | No | Yes (bash) |
| Arithmetic | Limited | Full |
| Process substitution | No | Yes (bash) |
| Coprocesses | No | Yes (bash) |
| Job control | No | Yes |
| Signal handling | Basic | Full |

## Future Enhancements

- Command-line editing (readline-style)
- Tab completion
- Command substitution `$(cmd)`
- Arithmetic expansion `$((expr))`
- Brace expansion `{a,b,c}`
- Globbing patterns `*.txt`
- Here documents `<<EOF`
- Functions with arguments
- Local variables in functions
- Signal traps
- Job control (bg, fg, jobs)
- Shell scripts with .zsh extension
- Profile/rc file loading
- Custom key bindings
- Colored output (ls --color)
- Man pages