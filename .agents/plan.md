# Round 1 Implementation Plan: Package Backends + Config Loading + Tera Engine

## Overview

Implement four units in sequence:
1. Config struct + loader (dwell-core)
2. Package backends (dwell-package)  
3. Wire CLI package commands (dwell-cli)
4. Tera template engine (dwell-template)

---

## 1. Config Loading — dwell-core + dwell-cli

### 1a. Add `Config` types to dwell-core

**NEW: `crates/dwell-core/src/config.rs`** — `Config`, `MetaConfig`, `DataConfig`, `PackageConfig`, `ModuleConfig`, `SecretConfig`, `PluginConfig`, `GenerationConfig` structs. All serde Deserialize + Default. Plus `Config::load(path)` that reads TOML with `~` expansion.

**MODIFY: `crates/dwell-core/src/lib.rs`** — add `pub mod config;` and `pub use config::*;`

### 1b. Wire config loading in CLI

**MODIFY: `crates/dwell-cli/src/commands.rs`** — load config in `run()`, pass `&Config` to all command functions.

**MODIFY**: `apply.rs`, `diff.rs`, `init.rs`, `add.rs`, `status.rs`, `watch.rs` — accept `&Config` param.

---

## 2. Package Backends — dwell-package

### 2a. Create backends module

**NEW: `crates/dwell-package/src/backends/mod.rs`** — declare submodules.

### 2b. Five backends, each implements `PackageManager`

| Backend | Binary | List | Install | Remove | Search |
|---------|--------|------|---------|--------|--------|
| apt | apt-get | apt list --installed | apt-get install -y | apt-get remove -y | apt-cache search |
| pacman | pacman | pacman -Q | pacman -S --noconfirm | pacman -R --noconfirm | pacman -Ss |
| brew | brew | brew list --formula -1 | brew install | brew uninstall | brew search |
| nix | nix | nix profile list | nix profile install nixpkgs# | nix profile remove | nix search nixpkgs |
| cargo | cargo | cargo install --list | cargo install | cargo uninstall | cargo search |

All use `Command::new("binary").arg("--version")` for availability check.

### 2c. Update `PackageManagerRegistry`

**MODIFY: `crates/dwell-package/src/manager.rs`** — register all 5 backends in `new()`. Add `get(id)`, `detect()`, `all()` methods.

---

## 3. Wire Up CLI Package Commands — dwell-cli

**NEW: `crates/dwell-cli/src/commands/package.rs`** — `run()`, `cmd_install()`, `cmd_list()`, `cmd_remove()`, `cmd_diff()`.

Flow: auto-detect first available backend → read packages from `cfg.packages` → execute.

**MODIFY: `crates/dwell-cli/src/commands.rs`** — add `mod package;`, dispatch Package command to `package::run()`.

---

## 4. Tera Template Engine — dwell-template

**NEW: `crates/dwell-template/src/tera_engine.rs`** — `TeraEngine` implementing `TemplateEngine` via `tera::Tera::one_off()`.

**MODIFY: `crates/dwell-template/Cargo.toml`** — add `tera.workspace = true`.

**MODIFY: `crates/dwell-template/src/lib.rs`** — add `mod tera_engine;`, register in `TemplateRegistry::new()`.
