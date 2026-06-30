# Changelog

All notable changes to ClipnPaste are documented in this file.

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

[0.1.0]: https://github.com/LinkofHyrule89/ClipnPaste/releases/tag/v0.1.0