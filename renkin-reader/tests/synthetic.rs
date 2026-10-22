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

/// 読み取りからフレーム探索まで: 読める画像を読み、読んだ護石と何個目でフレームを探して、frames.json の候補
/// (正解の護石から Python 版で求めたもの) と同じかを見る。素材の 4 枚は 1 回のマカフシギ錬金の 1〜4 個目なので、
/// 全部の画像の候補に共通するフレームは、その錬金のフレームただ 1 つになる
#[test]
fn synthetic_images_to_frames() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic");
    let want: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("frames.json")).unwrap()).unwrap();
    let (start, end) = (
        want["start"].as_u64().unwrap() as u32,
        want["end"].as_u64().unwrap() as u32,
    );
    let mut reader = renkin_reader::Reader::new();
    let mut common: Option<Vec<u32>> = None;
    for (file, frames) in want["images"].as_object().unwrap() {
        let (rgba, w, h) = e2e::load(&dir.join(file));
        let renkin_reader::ReadResult::Read { fields, .. } = reader.read(&rgba, w, h).unwrap()
        else {
            panic!("{file}: 読めなかった");
        };
        let charm = renkin_reader::Charm {
            skill1: fields.skill1.value,
            pts1: fields.pts1.value,
            skill2: fields.skill2.value,
            pts2: fields.pts2.value,
            slots: fields.slots.value,
        };
        // 画面と同じく、鑑定ボックスで読んだ位置が 5 以上なら何個目の指定なしにする
        let charms = [renkin_reader::SearchCharm {
            charm: charm.clone(),
            index: fields.index.value.filter(|&i| i <= 4),
        }];
        let got: Vec<u32> = renkin_reader::search_frames(&charms, start, end)
            .unwrap()
            .iter()
            .map(|c| c.frame)
            .collect();
        let want: Vec<u32> = frames
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        println!(
            "{file}: {charm:?} {} 個目 → 候補 {} 個",
            fields.index.value.unwrap_or(0),
            got.len()
        );
        assert_eq!(got, want, "{file}");
        common = Some(match common {
            None => got,
            Some(c) => c.into_iter().filter(|f| got.contains(f)).collect(),
        });
    }
    assert_eq!(
        common.unwrap(),
        vec![want["melding"].as_u64().unwrap() as u32],
        "全部の画像に共通するフレーム"
    );
}
