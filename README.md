# dwell — unify your dotfiles

[![CI](https://github.com/grave0x/dwell/actions/workflows/ci.yml/badge.svg)](https://github.com/grave0x/dwell/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange)](https://www.rust-lang.org)

A chezmoi-inspired dotfile manager written in Rust. Type-safe modules, cross-platform packages, and a plugin system.

## Install

# With Curl
```bash
# One-liner (Linux x86_64/arm64, macOS)
curl -fsSL https://github.com/grave0x/dwell/raw/main/scripts/install.sh | sh
```
# Or with Cargo
``` bash
cargo install --git https://github.com/grave0x/dwell
```
# Or pick your package manager
# Arch Linux:     ``yay -S dwell``
# Debian/Ubuntu:  ``sudo dpkg -i dwell_*.deb``
# Homebrew:      ``brew install grave0x/tap/dwell``


## Quick start

```bash
# Initialize
dwell init

# Add your dotfiles
dwell add ~/.bashrc
dwell add ~/.config/nvim/init.lua

# Deploy
dwell apply
```

## Why dwell?

| Feature | dwell | chezmoi |
|---------|-------|---------|
| Template engines | Handlebars, Rhai, Tera | Go templates |
| Package management | Built-in (apt, pacman, brew, nix, cargo) | External |
| Plugin system | WASM + Lua | None |
| Module system | TOML-defined generators | None |
| Secret backend | age | age, GPG, Vault |
| Migration from chezmoi | Auto-detection (`dot_` prefix) | — |

## Docs

```
dwell init       # set up ~/.local/share/dwell + git repo
dwell add <path> # add a dotfile to source
dwell apply      # render templates → $HOME
dwell diff       # preview changes
dwell status     # check sync state
dwell watch      # auto-apply on file changes
dwell doctor     # health check
dwell package    # manage system packages
```

## Config (`~/.config/dwell/dwell.toml`)

```toml
[meta]
name = "dotfiles"

[data]
home = "/home/user"
hostname = "my-machine"

[packages]
system.apt = ["build-essential", "git"]
system.cargo = ["bat", "ripgrep"]
```

## Related

- **Dotfiles repo**: [github.com/grave-configs/dotfiles](https://github.com/grave-configs/dotfiles) — my personal dotfiles managed by dwell
