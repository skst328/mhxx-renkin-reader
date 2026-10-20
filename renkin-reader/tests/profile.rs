//! 段階ごとの時間を表にする (読み取りの結果に付いてくる timings を使う。ネイティブ。wasm は tools/bench.mjs)。
//! `cargo test --release --test profile -- --ignored --nocapture`。
//! 写真は tests/golden/bench/ (tools/export_golden.py が charm-pics の写真から書き出す。無ければ何もせずに通る)。
//! 別の写真で測るときは RENKIN_BENCH=<ディレクトリ> (meta.json ([{name, file, width, height}]) と、file の名前の
//! RGBA の画素のファイルを置く)

use renkin_reader::{ReadResult, Reader, Timing};

fn print_table(title: &str, total: f64, timings: &[Timing]) {
    println!("\n{title} (合計 {total:.0} ms)");
    let mut sums: Vec<(&str, f64, usize)> = Vec::new();
    for t in timings {
        match sums.iter_mut().find(|s| s.0 == t.stage) {
            Some(s) => (s.1, s.2) = (s.1 + t.ms, s.2 + 1),
            None => sums.push((t.stage, t.ms, 1)),
        }
    }
    for (stage, ms, n) in sums {
        println!(
            "  {stage:20} {ms:7.0} ms {:5.1}%{}",
            ms / total * 100.0,
            if n > 1 {
                format!(" ({n} 回)")
            } else {
                String::new()
            }
        );
    }
}

#[test]
#[ignore]
fn stages() {
    let dir = std::env::var("RENKIN_BENCH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/bench")
        });
    if !dir.join("meta.json").is_file() {
        println!(
            "{} が無いので飛ばす (tools/export_golden.py で作る)",
            dir.display()
        );
        return;
    }
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("meta.json")).unwrap()).unwrap();
    let t = std::time::Instant::now();
    let mut reader = Reader::new();
    print_table(
        "準備 (new Reader)",
        t.elapsed().as_secs_f64() * 1000.0,
        reader.init_timings(),
    );
    for m in meta.as_array().unwrap() {
        let rgba = std::fs::read(dir.join(m["file"].as_str().unwrap())).unwrap();
        let t = std::time::Instant::now();
        let r = reader
            .read(
                &rgba,
                m["width"].as_u64().unwrap() as u32,
                m["height"].as_u64().unwrap() as u32,
            )
            .unwrap();
        let total = t.elapsed().as_secs_f64() * 1000.0;
        let (ReadResult::Read { timings, .. } | ReadResult::Rejected { timings, .. }) = &r;
        print_table(m["name"].as_str().unwrap(), total, timings);
    }
}
