# mdmore

mdmore is more or less a fast markdown pager written in Rust. It renders and highlights markdown text as you scroll,
and keeps colors and formatting across page boundaries. 

![mdmore paging through ripgrep's README](demo/mdmore.gif)

[Watch the video](https://github.com/bharathkrishnan/mdmore/raw/refs/heads/main/demo/mdmore.mp4)

## Performance history

![Performance improvements from v0.1.0 through v0.1.2](docs/performance.svg)

Measured CPU speed across three releases on the same machine and compiler.
Higher is faster; gains depend on the workload. See the [timings and methodology](docs/performance.md).

## Install

### Homebrew

```sh
brew tap bharathkrishnan/mdmore https://github.com/bharathkrishnan/mdmore.git
brew install bharathkrishnan/mdmore/mdmore
```

Installs a release binary for macOS or Linux, on ARM64 or x86-64.
macOS requires version 13 or later.

### From source

Requires Rust 1.85 or later and a C compiler for the bundled syntax-highlighting
engine. From a checkout of this repository:

```sh
cargo install --path . --locked
```

## Usage

```sh
mdmore README.md
mdmore examples/showcase.md
cat notes.md | mdmore
```

mdmore supports headings, nested lists, task lists, blockquotes, tables,
strikethrough, footnotes, and reference links. Code blocks have syntax
highlighting. Text wraps to the terminal width, including Unicode characters.
Resizing reflows the text around your position in the source.

### Keys

| Key | Action |
| --- | --- |
| `Space`, `f`, `PageDown` | Next page |
| `b`, `PageUp` | Previous page |
| `j`, `Down`, `Enter` | Down one row |
| `k`, `Up` | Up one row |
| `Ctrl-D`, `Ctrl-U` | Down / up half a page |
| `g`, `Home` | Beginning |
| `G`, `End` | End |
| `/` | Search; `Enter` submits |
| `n`, `N` | Next / previous matching row |
| `Esc` | Cancel search input or clear highlighting |
| Mouse wheel | Scroll |
| `?`, `h` | Help |
| `q`, `Ctrl-C` | Exit |

`Esc` also cancels a search or jump to the end while it is rendering.
Search is case-sensitive, operates on rendered rows, and stops at the beginning
or end of the document. A match split across two rows does not match.

### Options

```sh
mdmore --no-pager notes.md                 # Print without paging
mdmore --plain notes.md                    # Print without ANSI styling
mdmore --no-highlight notes.md             # Skip code syntax highlighting
mdmore --width 100 notes.md                # Limit content width
mdmore --no-mouse notes.md                 # Disable mouse wheel handling
mdmore --color always --no-pager notes.md | less -R
mdmore --help
```

Redirected output omits ANSI styling by default. `--color auto` respects
`NO_COLOR` and `TERM=dumb`; `--color always` overrides `NO_COLOR`.
`--color never` disables styling. `--plain` also disables the pager.

## Rendering and limits

The full UTF-8 source is loaded into memory and indexed by pulldown-cmark before
the first page. This lets reference links resolve definitions later in the file.
Wrapping, styling, and code highlighting happen as rows are requested. Piped
input waits for EOF.

Rendered rows are cached for backward scrolling, so memory use grows as you
read. Jumping to the end renders and caches the remaining rows. Resizing
rebuilds rows up to the saved source position and fills the viewport.
The position shown in the status bar is approximate.

Code highlighting runs one source line at a time. A line over 16 KiB disables
syntax highlighting for the rest of that code block. Tables buffer one row
to align cells; narrow or nested tables use a flowing layout. Raw HTML is
displayed as text. Images are shown as their alt text and URL.

## Development

```sh
cargo build --release --locked
./target/release/mdmore examples/showcase.md

cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
python3 tests/pty_smoke.py target/debug/mdmore   # macOS / Linux
cargo bench --locked --bench first_page
```
## License

[MIT](LICENSE).
