# Changelog

All notable changes to ClipnPaste are documented in this file.

## [0.2.6] - 2026-07-10

### Fixed

- Mint / GNOME application menu icon: install full Freedesktop hicolor sizes (16–512) and a proper desktop entry (`Icon=clipnpaste`, Utility categories) so the menu no longer falls back to a generic icon

## [0.2.5] - 2026-07-10

### Fixed

- After **Clear all** (or delete), the list stays empty; the live system clipboard is marked “seen” so it is not re-inserted, while **new** copies still appear without restarting
- Clipboard monitor tracks **text and image hashes separately** so Ctrl+C/X text is not ignored when X11 still exposes a stale image target
- New text copies win over stale images; image is preferred only when the accompanying text looks incidental (URL/path)
- Monitor only stamps hashes after a successful history insert
- History text wraps in the content column instead of clipping under edit/delete/pin buttons

### Tests

- Expanded unit tests for clear-all (no flash), copy-after-clear, pin, edit, promote-on-select, emoji text, and text-vs-image capture policy

## [0.2.4] - 2026-07-10

### Added

- Edit text clipboard items from history (✎ next to delete/pin): load full text, Save/Cancel, updated preview returns to the list
- `clipnpaste-cli settings` opens the settings window
- README screenshots and demo seed/capture scripts under `scripts/` and `docs/screenshots/`

### Note

Also includes 0.2.2–0.2.3 work shipped in this release train: live history sync, image-first capture, promote-on-select for Ctrl+V, unit tests, and documentation.

## [0.2.3] - 2026-07-10

### Added

- Rust unit tests for history DB, previews, types, and settings (`npm test` / `cargo test`)

### Fixed

- Selecting an item from Super+V history now **promotes** it to the top and sets the system clipboard, so **Ctrl+V** pastes that item again (Windows 11-style)
- Snip editor **Copy** updates both the system clipboard and clipboard history

### Changed

- Documented Super+V vs Ctrl+V: ClipnPaste never intercepts Ctrl+V; history selection only updates the OS clipboard first

## [0.2.2] - 2026-07-10

### Fixed

- Clipboard history now updates live while the panel is open (`history-changed` events)
- Super+V always reloads history when reusing the existing panel window
- Image copies are captured even when the clipboard also has text (image preferred)
- Re-copying the same content bumps it to the top instead of silently dropping it
- App clipboard writes (paste/snip) coordinate with the monitor hash gate to avoid thrashing
- History list no longer loads full item content blobs (summaries + image thumbnails)

### Changed

- Image history previews use downscaled thumbnails (full image still used for paste)

## [0.2.1] - 2026-06-30

### Added

- Settings window (tray menu and clipboard panel gear icon)
- Toggles to show or hide Emoji and GIF tabs (persisted in `settings.json`)
- **Keyboard shortcuts…** link opens Cinnamon Keyboard → Shortcuts (Custom Shortcuts)

### Changed

- `Super+;` does nothing when the Emoji tab is disabled
- Clipboard tab bar hides when only History is enabled
- Selecting history items or emoji inserts into the previously focused app (not clipboard-only)

### Fixed

- Emoji and clipboard history now paste into text fields across apps (`xdotool` type / Ctrl+V)
- Focus target captured in `clipnpaste-cli` when the hotkey fires (before the panel steals focus)
- Settings window close button and Escape key (missing Tauri window capability)

## [0.2.0] - 2026-06-30

### Added

- Tabbed clipboard panel: **History**, **Emoji**, and **GIF** (stub)
- `Super+;` opens the panel on the Emoji tab (Cinnamon custom shortcut + X11 fallback)
- Offline emoji picker with search and category filters (Microsoft Fluent UI Emoji visuals, Unicode paste)
- `clipnpaste-cli emoji` IPC command for the emoji hotkey
- `copy_text_to_clipboard` for emoji selection

### Planned

- GIF search via Klipy API (app-embedded key, no user configuration) in a future release

## [0.1.0] - 2026-06-30

First public release for Linux Mint / Cinnamon (X11).

### Added

- Clipboard history panel with text and image previews (`Super+V`)
- Pin, delete, and clear-unpinned clipboard items
- Snipping toolbar with fullscreen, window, and region capture (`Super+Shift+S`)
- Snip toast preview and annotation editor entry point
- System tray icon with quick actions
- Cinnamon custom shortcut registration (Mint Menu can keep Super)
- Local-only UI and data (embedded assets, strict CSP, no external hosting)
- Debian package (`.deb`) for amd64
- Local install script (`scripts/install-local.sh`) and system install script (`scripts/install-system.sh`)

### Changed

- Clipboard panel opens in the bottom-right corner by default
- Clipboard panel is draggable from its header
- Snip toolbar is compact, transparent, and draggable

### Fixed

- Production builds embed the frontend correctly (no `localhost:1420` white screen)
- `Super+V` works with Mint Menu still bound to Super (Cinnamon keybindings + menu dismiss)
- Normal `s` key typing no longer blocked while the app is running
- Cinnamon Settings shows ClipnPaste under Custom Shortcuts

[0.2.6]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.2.6
[0.2.5]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.2.5
[0.2.4]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.2.4
[0.2.3]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.2.3
[0.2.2]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.2.2
[0.2.1]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.2.1
[0.2.0]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.2.0
[0.1.0]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.1.0