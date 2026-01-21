# Kaku

A lightning-fast Markdown memo app. Press a hotkey, write, close — your note is saved automatically.

## Features

- **Instant Access** — Global hotkey (`Ctrl+Shift+Space`) brings up the editor instantly
- **Auto-Save** — Notes are saved automatically when you close the window
- **Live Preview** — Obsidian-style in-place formatting (no split view)
- **Memo List** — Browse and open your notes from the sidebar
- **Customizable** — Change save location, font, and color theme (Tokyo Night default)
- **Cross-Platform** — Works on Windows, macOS, and Linux

## Installation

Download the latest release for your platform from the [Releases](https://github.com/your-repo/releases) page.

## Usage

1. Press `Ctrl+Shift+Space` to open the editor
2. Start writing in Markdown
3. Close the window or press the hotkey again — your note is saved automatically

### Sidebar

Click the menu icon (top-left) to:
- View all your saved memos
- Open and edit existing notes
- Access settings

### Settings

- **Save Directory** — Where your notes are stored
- **Font** — Family, size, and line height
- **Theme** — Tokyo Night (dark) or Light

## File Format

Notes are saved as Markdown files with YAML front matter:

```markdown
---
uid: "01J1Z9P6V9WQ7H9QXGQ2K5J1ZC"
created_at: "2025-01-13T06:12:34+09:00"
updated_at: "2025-01-13T06:15:10+09:00"
---

Your note content here...
```

## Development

```bash
# Install dependencies
bun install

# Run in development mode
bun run tauri dev

# Build for production
bun run tauri build
```

## Tech Stack

- **Frontend**: SvelteKit 5 + TypeScript
- **Backend**: Rust + Tauri v2

## License

MIT
