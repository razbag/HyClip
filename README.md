# HyClip
<img width="64" height="64" alt="AppIcon-64" src="https://github.com/user-attachments/assets/b07deab4-893e-420c-9e21-ca885a89dcf9" />

A minimal macOS menu bar clipboard manager. No dock icon, no clutter — just a
quick way to search and reuse what you've recently copied.

## Features

- Lives entirely in the menu bar — no dock icon, no main window
- Keeps your last N copied text items, saved to disk (default 50, adjustable up to 250)
- Retention policy — auto-expire items after 1 week, 1 month, or keep forever
- Global shortcut **⌃⌘V** opens a small search popup near the menu bar icon
- Type to filter, arrow keys to navigate, **Return** or click to copy an item back
- Dark-mode aware, minimal UI
- In-app auto-update — checks on launch and every 24 hours, always asks
  before installing
- Tray menu: About, Check for Updates, Show, Preferences, Clear History, Quit

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

## Releasing a new version

1. Bump `version` in `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
2. Build with the updater signing key set (private key lives outside the
   repo, never commit it):
   ```sh
   TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/hyclip.key)" \
   TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
   cargo tauri build
   ```
3. This produces, under `src-tauri/target/release/bundle/`:
   - `dmg/HyClip_<version>_aarch64.dmg` — what people download to install
   - `macos/HyClip.app.tar.gz` + `.sig` — the updater artifact + signature
4. Build a `latest.json` manifest (see `tauri-plugin-updater` docs for the
   schema) using the contents of the `.sig` file, and a `pub_date` in RFC3339
   format.
5. Tag the commit (`git tag -a vX.Y.Z`), push it, then create a GitHub
   release for that tag with three assets attached: the `.dmg`, the
   `.app.tar.gz`, and `latest.json`. The updater's endpoint always points at
   `.../releases/latest/download/latest.json`, so it just needs to exist on
   whichever release GitHub currently considers "latest".

## Notes

- History and preferences are saved as plain JSON in
  `~/Library/Application Support/HyClip/` (`history.json`, `preferences.json`)
  and persist across restarts. Use Preferences → **Delete All History…** to
  wipe it (asks for confirmation first).
- HyClip does not currently filter out clipboard entries marked "concealed" by
  password managers (e.g. 1Password, Bitwarden) — anything copied gets stored
  like any other text, on disk, indefinitely if retention is set to Forever.
  Keep this in mind if you copy sensitive data often.

## License

MIT — see [LICENSE](LICENSE).

---

Built with the help of [Claude](https://claude.ai).
