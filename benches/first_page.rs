use std::hint::black_box;
use std::time::Instant;

use mdmore::document::Document;

fn main() {
    let paragraph =
        "## A section\n\nSome **bold** and *italic* text, with a [link](https://example.com).\n\n";
    for count in [100, 10_000, 100_000] {
        let source = paragraph.repeat(count);
        let mut samples = Vec::new();
        for _ in 0..20 {
            let start = Instant::now();
            let mut doc = Document::new(black_box(&source), 80, true);
            doc.ensure(25);
            black_box(&doc.lines);
            samples.push(start.elapsed());
        }
        samples.sort();
        let median = (samples[9] + samples[10]) / 2;
        println!(
            "{:>8} bytes: first 25 rows median {:?}, p95 {:?}",
            source.len(),
            median,
            samples[18]
        );
    }
    let source = format!("```rust\n{}\n```\n", "let answer = 42;\n".repeat(100_000));
    let cold_start = Instant::now();
    Document::new(&source, 80, true).ensure(25);
    println!(
        "{:>8} bytes: first 25 code rows, cold syntax load {:?}",
        source.len(),
        cold_start.elapsed()
    );
    let start = Instant::now();
    let mut doc = Document::new(&source, 80, true);
    doc.ensure(25);
    println!(
        "{:>8} bytes: first 25 code rows {:?}; {} cached rows",
        source.len(),
        start.elapsed(),
        doc.lines.len()
    );
}
