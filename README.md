<!-- [![progress-banner](https://backend.codecrafters.io/progress/shell/e1ba7798-5b8a-4e92-93c9-5fbb4c7f5745)](https://app.codecrafters.io/users/ExlipseVerse?r=2qF) -->

# $ tsh

A minimal POSIX-compliant command interpreter written from scratch in Rust.

Built as part of the CodeCrafters "Build Your Own Shell" challenge to explore low-level systems programming, tokenization, process spawning, and I/O streams.

## Features

- **Builtins:** `echo`, `exit`, `pwd`, `cd` (including `~` expansion), and `type`.
- **Executable Spawning:** Searches `PATH` to launch external binaries natively.
- **Redirection:** Supports standard output (`>` or `1>`) and standard error (`2>`) redirection to files.
- **Append Mode:** Supports standard output appending (`>>` or `1>>`).
- **Background Jobs:** Append `&` to run processes asynchronously in the background.
- **Tab Completion:** Interactive tab-completion for builtins, `PATH` binaries, and file/directory paths via `rustyline`.

## Quick Start

### Prerequisites

Ensure you have the Rust toolchain installed:
```bash
rustc --version
```

### Installation & Running

Clone the repository and run the binary using Cargo:

```bash
git clone https://github.com
cd tsh
cargo run --release
```

## Vibe Check

```text
\$ pwd
/home/hacker/tsh
\$ echo "hello world" > out.txt
\$ cat out.txt
hello world
\$ nonexistent-command 2> error.log
\$ cat error.log
nonexistent-command: command not found
\(sleep 10 & [1] 42069\) jobs
[1]+ Running      sleep 10 &
```

## License

MIT / Educational Open Source.