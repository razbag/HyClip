# HyClip

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

Produces a `.app` (and `.dmg`) under `src-tauri/target/release/bundle/`.

The build is unsigned and not notarized, so macOS Gatekeeper will flag it as
being from an unidentified developer. Right-click the app (or the mounted
`.dmg`) and choose **Open** the first time to bypass this.

## Notes

- History is stored in memory only and resets when the app quits. Preferences
  (history size) are also in-memory for now.
- HyClip does not currently filter out clipboard entries marked "concealed" by
  password managers (e.g. 1Password, Bitwarden) — anything copied gets stored
  like any other text. Keep this in mind if you copy sensitive data often.

## License

MIT — see [LICENSE](LICENSE).
