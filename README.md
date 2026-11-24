<p align="center">
  <img src="static/brand/zeroone-horizontal-fundo-escuro.svg" alt="ZeroOne" height="44" />
</p>

# ZeroOne Agents Tools

[![Build Status](https://github.com/Felipenogue91/zeroone-agents-tools/actions/workflows/build.yml/badge.svg)](https://github.com/Felipenogue91/zeroone-agents-tools/actions/workflows/build.yml)
[![Release](https://img.shields.io/github/v/release/Felipenogue91/zeroone-agents-tools)](https://github.com/Felipenogue91/zeroone-agents-tools/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Felipenogue91/zeroone-agents-tools/total)](https://github.com/Felipenogue91/zeroone-agents-tools/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-7c3aed.svg)](LICENSE)

Desktop control plane for managing **MCP servers, Skills, Sub-Agents, Slash Commands, Hooks, Rules, and Session Analytics** across AI coding agents — with filesystem reconciliation, global inheritance visibility, and **ZeroOne** visual identity.

> **Fork Notice:** ZeroOne Agents Tools is an MIT-licensed fork based on [`tylergraydev/claude-code-tool-manager`](https://github.com/tylergraydev/claude-code-tool-manager), rebuilt with scanner reconciliation fixes, multi-agent discovery, `pt-BR` localization, and ZeroOne branding.

---

## Download & Install

Grab the prebuilt installer for your OS from the [**Latest Release**](https://github.com/Felipenogue91/zeroone-agents-tools/releases/latest):

### Windows (10 / 11 x64)

- **Recommended:** Download `ZeroOne Agents Tools_1.0.0_x64-setup.exe` (NSIS installer) or `ZeroOne Agents Tools_1.0.0_x64_en-US.msi` and run it.
- Note: Since the Windows binary is community-distributed without an Authenticode EV certificate, Windows SmartScreen may ask for confirmation (`More info` → `Run anyway`).

### Linux (x64)

- **Ubuntu / Debian / Linux Mint / Pop!_OS (`.deb`):**
  ```bash
  sudo apt install ./ZeroOne.Agents.Tools_1.0.0_amd64.deb
  ```
- **Universal (`.AppImage`):**
  ```bash
  chmod +x ZeroOne*.AppImage
  ./ZeroOne*.AppImage
  ```
- **Fedora / RHEL / openSUSE (`.rpm`):**
  ```bash
  sudo dnf install ./ZeroOne*.rpm
  ```

### macOS (Apple Silicon & Intel)

- Download the `.dmg` (`aarch64` or `x64`), drag **ZeroOne Agents Tools.app** to **Applications**, and if Gatekeeper blocks an unsigned build run:
  ```bash
  xattr -cr "/Applications/ZeroOne Agents Tools.app"
  ```

---

## What's Improved in ZeroOne Agents Tools

- **True Filesystem Reconciliation:** Automatically updates modified MCPs, skills, and hooks on re-scan and prunes deleted auto-detected entries and stale project paths instead of accumulating ghosts in SQLite.
- **Global Inheritance Per Project:** Project detail views clearly separate **Global (Inherited)** MCPs, Skills, Sub-Agents, Commands, and Hooks from local overrides, and exclude already-active global tools from the "Available" list.
- **Home Directory Guard:** Never treats `~` (`$HOME`) as a project entry — global `~/.claude` configurations stay strictly in Global Scope.
- **Cross-Platform Multi-Agent Discovery:** Scans `~/.agents/skills`, Claude Code (`~/.claude.json`, `~/.claude/`), Antigravity CLI (`~/.gemini/antigravity-cli/mcp_config.json`), Gemini CLI, OpenCode, Codex CLI, Cursor, and Copilot CLI.
- **MCP Server Accuracy:** Built-in MCP server (`list_projects`) returns real `assignedMcps` per project.
- **ZeroOne Design System & Languages:** Obsidian dark/light UI (`ZEROONE.GG` tokens) and localization in **English (`EN`)**, **Português BR (`PT`)**, **简体中文 (`简`)**, and **繁體中文 (`繁`)**.

---

## Supported Editors & Config Locations

| Editor / Agent | Global Config Location | Format |
|----------------|------------------------|--------|
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code) | `~/.claude.json` & `~/.claude/` | JSON / MD |
| Antigravity CLI | `~/.gemini/antigravity-cli/mcp_config.json` & `~/.agents/skills` | JSON / MD |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli) | `~/.gemini/settings.json` | JSON |
| [OpenCode](https://opencode.ai) | `~/.config/opencode/opencode.json` | JSON |
| [Codex CLI](https://github.com/openai/codex) | `~/.codex/config.toml` | TOML |
| [Cursor](https://cursor.com) | `~/.cursor/mcp.json` | JSON |
| [GitHub Copilot CLI](https://githubnext.com/projects/copilot-cli) | `~/.copilot/mcp-config.json` | JSON |

---

## Build from Source

Requires [Node.js](https://nodejs.org/) 20+, [Rust](https://www.rust-lang.org/tools/install) stable, and [Tauri v2 prerequisites](https://tauri.app/start/prerequisites/).

```bash
git clone https://github.com/Felipenogue91/zeroone-agents-tools.git
cd zeroone-agents-tools
npm ci
npm run tauri dev     # Development mode
npm run tauri build   # Release installers in src-tauri/target/release/bundle/
```

On Ubuntu 24.04+, install the Tauri system dependencies before building:

```bash
sudo apt-get update && sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libxdo-dev patchelf xdg-utils file
```

---

## Tech Stack

Svelte 5 + SvelteKit • Tauri 2 + Rust • SQLite • Tailwind CSS v4

## License & Attribution

Licensed under the [MIT License](LICENSE).
Original upstream project: [`tylergraydev/claude-code-tool-manager`](https://github.com/tylergraydev/claude-code-tool-manager).
