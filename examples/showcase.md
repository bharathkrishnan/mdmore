# Markdown examples

Open this file with `mdmore examples/showcase.md`.

## Inline formatting

Text can be **bold**, *italic*, **bold and *italic***, or ~~struck out~~.
Commands such as `cargo build --release` use inline code. A
[link to the repository](https://github.com/bharathkrishnan/mdmore) includes
its destination.

This paragraph is long enough to wrap in a narrow terminal. **Bold text can
continue onto the next row**, and *italic text can wrap too*. Unicode examples
include café, é, 世界, and 👩🏽‍💻.

## Blockquotes

> A blockquote can contain multiple paragraphs.
>
> It can also contain **formatting** and `code`.
>
> > Quotes can be nested.

## Code

```rust
fn main() {
    let numbers = [2, 3, 5, 7];
    let total: i32 = numbers.iter().sum();
    println!("sum: {total}");
}
```

```python
from pathlib import Path

for path in sorted(Path(".").glob("*.md")):
    print(path.name)
```

## Tables

| Key | Action | Rows |
| :--- | :--- | ---: |
| `j` | Scroll down | 1 |
| `k` | Scroll up | 1 |
| Mouse wheel | Scroll in the wheel's direction | 3 |

## Lists

1. Build with `cargo build --release`.
2. Open a document.
   - Scroll with `j` and `k`.
   - Page with `Space` and `b`.
   - Search with `/`, then repeat with `n`.
3. Exit with `q`.

- [x] Read chapter one
- [ ] Read chapter two

---

## References

Here is a [reference link][project] and a footnote.[^note]

[^note]: Footnote definitions can appear later in the document.

[project]: https://github.com/bharathkrishnan/mdmore
