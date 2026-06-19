# poneglyph

A tiny, beautiful terminal markdown editor for ancient texts and modern notes.

`poneglyph` is built for fast markdown reading and editing directly in the terminal: preview-first, themeable, keyboard-driven, and small enough to disappear into your workflow.

<img width="1023" height="1190" alt="image" src="https://github.com/user-attachments/assets/c3ae878d-383c-449e-b9d3-92489d438060" />


Current local release build:

| Artifact | Size / memory |
| --- | ---: |
| Release binary | ~955 KB |
| Runtime peak RSS | ~17 MB |

## Features

- Preview-first markdown reading.
- In-place edit mode.
- Outline sidebar and file browser.
- Theme picker with bundled themes.
- Boxed or smooth chrome.
- Configurable cursor style and theme swatches.
- Rich preview rendering for headings, blockquotes, nested lists, code blocks, tables, links, and images.
- Inline image rendering from markdown via [ratatui-image](https://github.com/ratatui/ratatui-image), using the terminal's native graphics protocol (kitty/sixel/iTerm2) with an aspect-correct fit; terminals without graphics support show a short hint instead.
- Save, undo/redo, Delete/Backspace editing, and file opening.
- Debug commands for fixtures, snapshots, and automation.

## Install

See [docs/install.md](docs/install.md).

From source:

```bash
git clone https://github.com/ShamanicArts/poneglyph.git
cd poneglyph
cargo install --path .
```

Once installed:

```bash
poneglyph README.md
```

## Usage

```bash
cargo run -- README.md
cargo run --release -- fixtures/large.md

# after release build
./target/release/poneglyph README.md
```

Direct mode keys:

- `Ctrl+E` edit mode
- `Ctrl+V` view mode
- `Ctrl+F` files mode
- `Ctrl+Q` quit anywhere
- `Ctrl+S` save anywhere
- `Ctrl+Z` / `Ctrl+Y` undo / redo

View mode commands after `Ctrl+V`:

- `o` outline
- `r` collapse/expand sidebar
- `t` theme picker
- `b` boxed/smooth chrome
- `c` cursor style

Edit mode:

- Type to insert text.
- Arrows/Home/End/PageUp/PageDown move.
- `Enter` newline.
- `Backspace` delete backward.
- `Delete` delete forward.
- `Esc` exits back to preview.

Legacy `Ctrl+X` commands remain available for compatibility.

## Configuration

Default config path:

```text
~/.config/poneglyph/config.toml
```

Project-local config is also supported:

```text
.poneglyph.toml
```

Example:

```toml
[ui]
theme = "tokyo-night"
cursorStyle = "block"      # brackets | block | bar | underline | box
boxedChrome = true
themeSwatches = "square"   # off | circle | square
themeSwatchSpacing = 0      # 0..8
```

You can override config path for testing:

```bash
PONEGLYPH_CONFIG=/tmp/poneglyph.toml poneglyph README.md
```

## Debug helpers

```bash
cargo run -- outline fixtures/large.md
cargo run -- stats fixtures/large.md
cargo run -- classify fixtures/small.md
cargo run -- preview-lines fixtures/small.md --width 96 --height 32
cargo run -- sidebar-lines fixtures/small.md --files
cargo run -- state-after-keys fixtures/small.md 'ctrl+e,right,delete'
```

## Development checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
cargo package --locked --no-verify
```
