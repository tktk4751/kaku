# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

z_memo is a fast Markdown memo application built with Tauri v2. It's designed for quick capture via global hotkey (Ctrl+Shift+Space) with automatic save on close. The UI aims for Obsidian-style live preview (in-place formatting without split view).

## Development Commands

```bash
# Development (starts both frontend and Tauri)
bun run tauri dev

# Build for production
bun run tauri build

# Frontend only (Vite dev server)
bun run dev

# Type checking
bun run check
bun run check:watch   # watch mode
```

## Architecture

**Tauri v2 + SvelteKit 5 + Rust**

- **Frontend** (`src/`): SvelteKit 5 with Svelte 5 runes (`$state`, etc.), compiled as static site
- **Backend** (`src-tauri/`): Rust application handling file I/O, hotkeys, window management, and system tray

**Key Configuration:**
- `src-tauri/tauri.conf.json` - Tauri app config (uses bun as package manager)
- `svelte.config.js` - SvelteKit with static adapter
- `vite.config.js` - Vite bundler config

**Planned Rust Modules** (per README spec):
- `core/note` - ULID generation, YAML front matter parsing
- `core/storage` - Atomic file saves (`tmp` → `rename`)
- `core/index` - Memo listing from directory
- `core/settings` - Config persistence (save path, font, theme, window geometry)
- `platform/hotkey` - OS-specific global hotkey registration
- `platform/window` - Show/hide toggle, focus, geometry restore

## Key Requirements

- **File format**: Markdown with YAML front matter (`uid`, `created_at`, `updated_at`)
- **File naming**: `{ulid}.md`
- **Save behavior**: Atomic writes with guaranteed completion before window hide
- **Theme**: Tokyo Night color scheme (dark default)
- **Window**: Remember position/size, auto-correct if off-screen

## Rust-Frontend Communication

Use Tauri commands via `@tauri-apps/api/core`:
```typescript
import { invoke } from "@tauri-apps/api/core";
const result = await invoke("command_name", { arg: value });
```
