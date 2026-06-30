# Changelog

All notable changes to ClipnPaste are documented in this file.

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

[0.2.1]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.2.1
[0.2.0]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.2.0
[0.1.0]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.1.0