# doot

A simple dotfile manager that copies or symlinks files between your system and a dotfiles repository.

## Installation

```bash
cargo binstall --git https://github.com/onbjerg/doot doot
```

## Quick Start

1. Create a `doot.yaml` in your dotfiles repo:

```yaml
version: v1
mode: file  # or "link" for symlinks

groups:
  bash:
    nux: "~"
    mac: "$HOME"
  vim:
    nux: "~"
    mac: "~"

plans:
  all:  # empty = all groups
  minimal: [bash]
```

2. Create group directories with `.dootignore` files:

```
dotfiles/
├── bash/
│   ├── .dootignore
│   ├── .bashrc
│   └── .bash_profile
├── vim/
│   ├── .dootignore
│   └── .vimrc
└── doot.yaml
```

3. Use gitignore syntax in `.dootignore` to control which files are tracked:

```gitignore
*            # ignore everything
!.bashrc     # except these
!.profile
```

## Usage

```bash
# Import files from system to repo
doot import group bash nux
doot import plan all mac

# Export files from repo to system
doot export group vim nux
doot export plan minimal mac

# Skip confirmation
doot -y import group bash nux

# Custom config path
doot -c ~/.dotfiles/doot.yaml export plan all nux
```

## Example Workflow

**Initial setup** - import your existing dotfiles:

```bash
$ doot import group bash nux

[+] create  .bashrc
[+] create  .bash_profile

Apply 2 changes? [y/N] y
Done.
```

**On a new machine** - export your dotfiles:

```bash
$ doot export plan all mac

[+] create  .bashrc
[+] create  .bash_profile
[✓] same    .vimrc
[~] overwrite .gitconfig

Apply 3 changes? [y/N/d] d

--- bash/.bashrc (destination)
+++ bash/.bashrc (source)
──────────────────────────────────────────────────────────
   1   export PATH="$HOME/bin:$PATH"
   2 - export EDITOR=nano
   2 + export EDITOR=vim
   3   alias ll='ls -la'

Apply 3 changes? [y/N/d] y
Done.
```

## Concepts

| Concept | Description |
|---------|-------------|
| **Group** | A directory of related files (e.g., `bash/`, `vim/`) |
| **Resolver** | A named path mapping (e.g., `nux: "~"`, `mac: "$HOME"`) |
| **Plan** | A collection of groups for batch operations |
| **Mode** | `file` (copy) or `link` (symlink) |

## Confirmation Prompt

Before applying changes, doot shows a confirmation prompt:

- `y` - proceed with the changes
- `n` or Enter - abort
- `d` - show syntax-highlighted diffs for all files that would be created or overwritten

The diff view shows line numbers and uses red/green coloring for deletions/additions.

## Path Expansion

- `~` expands to home directory
- `$VAR` or `${VAR}` expands environment variables

## Acknowledgements

doot is inspired by [dotato](https://github.com/msisdev/dotato), a similar dotfile manager written in Go. doot is a Rust rewrite with additional features like diff previews.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
