//! リポジトリに入っている人工の写真 (tests/synthetic/) を、写真から最後まで読み、answers.csv の期待どおりかを確かめる。
//! キャプチャ (ゲーム画面) を斜めに置いて劣化させたもので、tools/make_synthetic.py が作る (作り方と中身はそちら)。
//!
//! リポジトリだけで動く (charm-pics は要らない)。wasm の中でも流せる:
//! `cargo test --release --target wasm32-wasip1 --test synthetic`

#[path = "support/e2e.rs"]
mod e2e;

use std::path::Path;

#[test]
fn synthetic_images_as_expected() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let all = e2e::answers(&root, &["synthetic"]);
    assert!(!all.is_empty(), "tests/synthetic/answers.csv が無い");
    let (all, results) = e2e::run(all, |a| e2e::load(&a.path));
    let unexpected = e2e::summarize(&all, &results, &[]);
    assert!(unexpected.is_empty());
}
