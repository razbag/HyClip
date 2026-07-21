# HyClip
<img width="64" height="64" alt="AppIcon-64" src="https://github.com/user-attachments/assets/b07deab4-893e-420c-9e21-ca885a89dcf9" />

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

The build is ad-hoc signed but not notarized (no paid Apple Developer
account), so macOS Gatekeeper will still flag it once downloaded. If
right-click → **Open** doesn't get you past the warning, or macOS says the
app **"is damaged and can't be opened"** (a misleading message — it isn't
actually damaged, Gatekeeper just refuses unnotarized downloads by default),
open Terminal and run:

```sh
xattr -cr /path/to/HyClip.app
```

(e.g. `xattr -cr /Applications/HyClip.app` if you moved it there, or
`xattr -cr ~/Downloads/HyClip.app` otherwise). This removes the quarantine
flag macOS attaches to anything downloaded from a browser and always
resolves the "damaged" message.

## Notes

- History is stored in memory only and resets when the app quits. Preferences
  (history size) are also in-memory for now.
- HyClip does not currently filter out clipboard entries marked "concealed" by
  password managers (e.g. 1Password, Bitwarden) — anything copied gets stored
  like any other text. Keep this in mind if you copy sensitive data often.

## License

MIT — see [LICENSE](LICENSE).
