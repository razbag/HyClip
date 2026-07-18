# HayClip

A minimal macOS menu bar clipboard manager. No dock icon, no clutter — just a
quick way to search and reuse what you've recently copied.

## Features

- Lives entirely in the menu bar — no dock icon, no main window
- Keeps your last N copied text items in memory (default 50, adjustable up to 250)
- Global shortcut **⌃⌘V** opens a small search popup near the menu bar icon
- Type to filter, arrow keys to navigate, **Return** or click to copy an item back
- Dark-mode aware, minimal UI
- Tray menu: About, Show, Preferences, Clear History, Quit

## Requirements

- macOS 11+
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI v2](https://v2.tauri.app/): `cargo install tauri-cli --version "^2.0.0"`

## Development

```sh
cargo tauri dev
```

## Building

```sh
cargo tauri build
```

Produces a signed-for-local-use `.app` (and `.dmg`) under
`src-tauri/target/release/bundle/`.

## Notes

History is stored in memory only and resets when the app quits. Preferences
(history size) are also in-memory for now.

## License

MIT — see [LICENSE](LICENSE).
