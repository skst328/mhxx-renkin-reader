//! 写真から最後まで読むテスト (tests/synthetic.rs と tests/charm_pics.rs) で共通の道具: answers.csv の読み込み、
//! 画像を開く、読んで期待と比べる (experimental-charm-reader の evaluate と同じ判定)、結果の表示。
//!
//! answers.csv の列: file, index (何個目か。`-` は写っていない、空欄は不明), skill1, pts1, skill2, pts2, slot,
//! expect (`retake` なら撮り直しになるのが正しい), note。スキルはゲームの表示名で書く。
//!
//! 環境変数 (調べるとき用): RENKIN_ONLY (名前にこの文字列を含む画像だけ)、RENKIN_LIMIT (先頭の n 枚だけ)、
//! RENKIN_DUMP (位置合わせ後の画面を、このディレクトリに PNG で書き出す)

#![allow(dead_code)] // テストごとに使う道具が違う

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use renkin_reader::{Field, ReadResult, Reader, skills};

pub struct Answer {
    /// 表示用の名前 (answers.csv を探し始めた場所からの相対パス)
    pub name: String,
    pub path: PathBuf,
    /// None = わからない、Some(None) = 写っていない、Some(Some(n)) = n 個目
    pub index: Option<Option<u8>>,
    pub skill1: u16,
    pub pts1: i8,
    pub skill2: Option<u16>,
    pub pts2: Option<i8>,
    pub slot: Option<u8>,
    pub retake: bool,
}

/// root の下の dirs (の下全部) にある answers.csv を読む
pub fn answers(root: &Path, dirs: &[&str]) -> Vec<Answer> {
    let mut found = Vec::new();
    let mut stack: Vec<PathBuf> = dirs.iter().map(|d| root.join(d)).collect();
    while let Some(d) = stack.pop() {
        if d.join("answers.csv").is_file() {
            found.push(d.clone());
        }
        for e in std::fs::read_dir(&d).unwrap_or_else(|e| panic!("{}: {e}", d.display())) {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            }
        }
    }
    found.sort();
    let id = |name: &str| {
        skills()
            .iter()
            .find(|(_, n)| *n == name)
            .map(|(id, _)| *id)
            .unwrap_or_else(|| panic!("スキル名が表に無い: {name}"))
    };
    let mut out = Vec::new();
    for d in found {
        let text = std::fs::read_to_string(d.join("answers.csv")).unwrap();
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        let col = |row: &[&str], k: &str| {
            header
                .iter()
                .position(|h| *h == k)
                .and_then(|i| row.get(i).copied())
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let rel = d
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        for line in lines.filter(|l| !l.trim().is_empty()) {
            let row: Vec<&str> = line.split(',').collect();
            let file = col(&row, "file");
            let index = match col(&row, "index").as_str() {
                "" => None,
                "-" => Some(None),
                n => Some(Some(n.parse().unwrap())),
            };
            let opt = |k: &str| {
                let v = col(&row, k);
                (!v.is_empty()).then_some(v)
            };
            out.push(Answer {
                name: if rel.is_empty() {
                    file.clone()
                } else {
                    format!("{rel}/{file}")
                },
                path: d.join(&file),
                index,
                skill1: id(&col(&row, "skill1")),
                pts1: col(&row, "pts1").parse().unwrap(),
                skill2: opt("skill2").map(|s| id(&s)),
                pts2: opt("pts2").map(|s| s.parse().unwrap()),
                slot: opt("slot").map(|s| s.parse().unwrap()),
                retake: col(&row, "expect") == "retake",
            });
        }
    }
    out
}

/// 画像を RGBA で開く (写真の向き (EXIF) も反映する。ブラウザで開いたときと同じ)
pub fn load(path: &Path) -> (Vec<u8>, u32, u32) {
    let mut decoder = image::ImageReader::open(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .with_guessed_format()
        .unwrap()
        .into_decoder()
        .unwrap();
    let orientation = image::ImageDecoder::orientation(&mut decoder).unwrap();
    let mut img = image::DynamicImage::from_decoder(decoder)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    img.apply_orientation(orientation);
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    (rgba.into_raw(), w, h)
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Ok,
    Retake(String),
    Wrong(String),
}

/// 1 枚の結果: (何番目の答えか, 結果, 何個目かが合ったか, 自信ありの欄のうち一番小さい 2 位との差 (欄の名前, 差))
pub type Judged = (usize, Outcome, Option<bool>, Option<(&'static str, f32)>);

pub fn judge(a: &Answer, r: &ReadResult) -> (Outcome, Option<bool>, Option<(&'static str, f32)>) {
    let fields = match r {
        ReadResult::Rejected {
            reason,
            diagnostics,
            ..
        } => {
            return (
                Outcome::Retake(format!("{reason:?} {diagnostics:?}")),
                None,
                None,
            );
        }
        ReadResult::Read { fields, .. } => fields,
    };
    let unsure = |f: bool, k: &str, s: f32, m: f32| (!f).then(|| format!("{k}({s:.2}/{m:.2})"));
    let reasons: Vec<String> = [
        unsure(
            fields.skill1.confident,
            "skill1",
            fields.skill1.score,
            fields.skill1.margin,
        ),
        unsure(
            fields.pts1.confident,
            "pts1",
            fields.pts1.score,
            fields.pts1.margin,
        ),
        unsure(
            fields.skill2.confident,
            "skill2",
            fields.skill2.score,
            fields.skill2.margin,
        ),
        unsure(
            fields.pts2.confident,
            "pts2",
            fields.pts2.score,
            fields.pts2.margin,
        ),
        unsure(
            fields.slots.confident,
            "slot",
            fields.slots.score,
            fields.slots.margin,
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    let index_ok = match a.index {
        None => None,
        Some(None) => Some(fields.index.value.is_none()),
        Some(Some(n)) => fields.index.value.map(|v| v == n),
    };
    // スキル名の 2 位との差 (一番の読み違いの元)。少しずつ痩せていく劣化を見るために表示する
    let margin = [
        ("skill1", fields.skill1.margin),
        ("skill2", fields.skill2.margin),
    ]
    .into_iter()
    .min_by(|a, b| a.1.total_cmp(&b.1));
    let got = (
        fields.skill1.value,
        fields.pts1.value,
        fields.skill2.value,
        fields.pts2.value,
    );
    let want = (a.skill1, a.pts1, a.skill2, a.pts2);
    let content_ok = got == want && a.slot.is_none_or(|s| s == fields.slots.value);
    let show = |f: &Field<u8>| f.value;
    if !reasons.is_empty() {
        return (Outcome::Retake(reasons.join(" ")), index_ok, None);
    }
    if content_ok {
        (Outcome::Ok, index_ok, margin)
    } else {
        (
            Outcome::Wrong(format!(
                "{got:?} S{} / 正解 {want:?} S{:?}",
                show(&fields.slots),
                a.slot
            )),
            index_ok,
            None,
        )
    }
}

/// 全部の画像を読んで判定する (環境変数 RENKIN_ONLY・RENKIN_LIMIT で絞れる)。open は画像を RGBA で開く関数。
/// 3 スレッドで読む。wasm (wasm32-wasip1) ではスレッドを作れないので 1 本で順に読む
pub fn run(
    mut all: Vec<Answer>,
    open: impl Fn(&Answer) -> (Vec<u8>, u32, u32) + Sync,
) -> (Vec<Answer>, Vec<Judged>) {
    if let Ok(n) = std::env::var("RENKIN_LIMIT") {
        all.truncate(n.parse().unwrap());
    }
    if let Ok(only) = std::env::var("RENKIN_ONLY") {
        all.retain(|a| a.name.contains(&only));
    }
    let next = Mutex::new(0usize);
    let results = Mutex::new(Vec::new());
    let work = || {
        let mut reader = Reader::new();
        loop {
            let i = {
                let mut n = next.lock().unwrap();
                let i = *n;
                *n += 1;
                i
            };
            let Some(a) = all.get(i) else { break };
            let (rgba, w, h) = open(a);
            let t = std::time::Instant::now();
            let r = reader.read(&rgba, w, h).unwrap();
            let (outcome, index_ok, margin) = judge(a, &r);
            if let (Ok(dir), Some(screen)) = (std::env::var("RENKIN_DUMP"), reader.aligned_image())
            {
                let path = Path::new(&dir).join(a.name.replace('/', "__") + ".png");
                image::save_buffer(&path, screen, 1280, 720, image::ColorType::Rgba8).unwrap();
            }
            println!(
                "{:60} {:5.1}s {:?}",
                a.name,
                t.elapsed().as_secs_f32(),
                outcome
            );
            results.lock().unwrap().push((i, outcome, index_ok, margin));
        }
    };
    if cfg!(target_os = "wasi") {
        work();
    } else {
        std::thread::scope(|s| {
            for _ in 0..3 {
                s.spawn(work);
            }
        });
    }
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|r| r.0);
    (all, results)
}

/// 結果をまとめて表示し、期待どおりでないもの (known に挙げた既知のものを除く) を返す。
/// known: (名前, 理由)。既知のものは期待と違っても失敗にしないが、表示はする (直ったら一覧から消す)
pub fn summarize(all: &[Answer], results: &[Judged], known: &[(&str, &str)]) -> Vec<String> {
    let (mut ok, mut retake, mut wrong) = (0, 0, 0);
    let (mut unexpected, mut known_hit) = (Vec::new(), Vec::new());
    let (mut idx_ok, mut idx_ng, mut idx_none) = (0, 0, 0);
    let mut tightest: Option<(&str, &str, f32)> = None;
    for (i, outcome, index_ok, margin) in results {
        let a = &all[*i];
        match outcome {
            Outcome::Ok => ok += 1,
            Outcome::Retake(_) => retake += 1,
            Outcome::Wrong(_) => wrong += 1,
        }
        let expected = if a.retake {
            matches!(outcome, Outcome::Retake(_))
        } else {
            *outcome == Outcome::Ok
        };
        let line = format!(
            "{}: {outcome:?} / 期待: {}",
            a.name,
            if a.retake { "retake" } else { "OK" }
        );
        match known.iter().find(|(n, _)| *n == a.name) {
            Some((_, why)) if !expected => known_hit.push(format!("{line} (既知: {why})")),
            Some(_) => known_hit.push(format!(
                "{}: 既知の外れが期待どおりになった (known から消してよい)",
                a.name
            )),
            None if !expected => unexpected.push(line),
            None => {}
        }
        if a.index.is_some() {
            match index_ok {
                Some(true) => idx_ok += 1,
                Some(false) => idx_ng += 1,
                None => idx_none += 1,
            }
        }
        if let Some((field, m)) = margin
            && tightest.is_none_or(|t| *m < t.2)
        {
            tightest = Some((&a.name, field, *m));
        }
    }
    println!(
        "期待どおり {}/{}  (OK {ok}、撮り直し {retake}、読み間違い {wrong})",
        results.len()
            - unexpected.len()
            - known_hit.iter().filter(|l| l.contains("(既知: ")).count(),
        results.len()
    );
    println!("何個目か: 合った {idx_ok}、違った {idx_ng}、読めなかった {idx_none}");
    if let Some((name, field, m)) = tightest {
        println!("スキル名の 2 位との差が一番小さかったもの: {name} の {field} ({m:.3})");
    }
    for l in &known_hit {
        println!("  {l}");
    }
    for u in &unexpected {
        println!("  {u}");
    }
    unexpected
}
