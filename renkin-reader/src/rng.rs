//! 乱数とフレームの探索: お守りが出る錬金のフレームを探す (マカフシギ錬金。投入は最高ランク)。
//!
//! apmnnn/mhxx-rng (https://github.com/apmnnn/mhxx-rng) の jujupush (meldingpush と juju_colors) と同じ手順で、
//! 錬金したフレーム F ごとに:
//!   1. F の 1 つ前の乱数から、風化した護石と古びた護石が何個ずつ出るかを決める
//!   2. F から順に護石を作る。1 個につき乱数を 4 個 (第 2 スキルなし) か 6 個 (あり) 使う
//!
//! ライセンスは [`xorshift`] の記載を参照 (apmnnn/mhxx-rng, MIT)。

mod charm;
mod xorshift;

use charm::{Generated, Table, WEATHERED, WORN};
use xorshift::{SEED, ascend, descend, jump};

use crate::types::{
    Charm, FrameMatch, InputError, PointRange, SearchCharm, SkillInfo, union_range,
};

/// 1 回の錬金で出る護石の種類の数 (投入は最高ランク): 風化 (乱数 % 1 + 2 個)、古びた (乱数 % 2 + 1 個)。
/// mhxx-rng の juju_colors の rank 4 の値 (幅, 最小)
const WEATHERED_COUNT: (u32, u32) = (1, 2);
const WORN_COUNT: (u32, u32) = (2, 1);
/// 1 回の錬金で出る護石の最大の数 (風化 2 個 + 古びた 2 個)
const MAX_CHARMS: u8 = 4;
/// 錬金したフレームから、最後の護石の最初の乱数までの最大の間 (護石は最大 4 個で、1 個につき乱数を最大 6 個使う)
const MAX_OFFSET: usize = 3 * 6;
/// 手元に持つ乱数の数: 錬金の 1 つ前 + MAX_OFFSET + 最後の護石の 7 個より大きい 2 のべき
const WINDOW: usize = 64;

/// mhxx-rng の rand: 下位 16 bit を初期値に、0 なら 1 にして x176 mod 65363 を n 回
fn rand(seed: u32, n: u32) -> u32 {
    let mut v = seed & 0xFFFF;
    for _ in 0..n {
        v = v.max(1) * 176 % 65363;
    }
    v
}

/// 錬金で出る風化した護石と古びた護石の数 (mhxx-rng の juju_colors。seed は錬金の 1 つ前のフレームの乱数)
fn counts(seed: u32) -> (usize, usize) {
    let n1 = rand(seed, 1) % WEATHERED_COUNT.0 + WEATHERED_COUNT.1;
    let n2 = rand(seed, 2) % WORN_COUNT.0 + WORN_COUNT.1;
    (n1 as usize, n2 as usize)
}

/// フレーム f の乱数の状態 (f = -1 は初期値の 1 つ前)
fn state(f: i64) -> [u32; 4] {
    if f < 0 { descend(SEED) } else { jump(f as u64) }
}

/// 錬金で出る護石 (鑑定ボックスの並び順) と、その種類 (0 = 風化した護石、1 = 古びた護石)。
/// v(d) は錬金したフレームの d - 1 個先の乱数 (v(0) が 1 つ前、v(1) が錬金したフレーム)。d は MAX_OFFSET + 7 まで使う
fn meld(v: impl Fn(usize) -> u32) -> Vec<(u8, Generated)> {
    let (n1, n2) = counts(v(0));
    let tables = [&WEATHERED, &WORN];
    let mut pos = 1;
    (0..n1 + n2)
        .map(|j| {
            let k = usize::from(j >= n1);
            let c = tables[k].make(&std::array::from_fn(|d| v(pos + d)));
            pos += tables[k].stride(v(pos + 2));
            (k as u8, c)
        })
        .collect()
}

/// frame で錬金したときに出る護石 (jump で frame へ飛んで作る)。テストで、Python 版や総当たりと比べるのに使う
#[cfg(test)]
fn melding(frame: u32) -> Vec<(u8, Generated)> {
    // v[k] はフレーム frame - 1 + k の乱数
    let mut s = state(frame as i64 - 1);
    let v: Vec<u32> = (0..WINDOW)
        .map(|_| {
            let w = s[3];
            s = ascend(s);
            w
        })
        .collect();
    meld(|d| v[d])
}

/// 作った護石が、鑑定画面で読んだ中身と同じか
fn matches(c: &Generated, want: &Charm) -> bool {
    if c.skill1 != want.skill1 || c.pts1 != want.pts1 || c.slots != want.slots {
        return false;
    }
    match (want.skill2, want.pts2) {
        (Some(s2), Some(p2)) => c.shows_skill2() && c.skill2 == Some(s2) && c.pts2 == p2,
        (None, _) => !c.shows_skill2(),
        (Some(_), None) => false,
    }
}

/// 種類ごとの、読んだ護石になりうるかを護石を作る前に手早く見るための値
struct Quick {
    /// 第 1 スキルが表の何番目か (r0 % 表の長さ がこれなら第 1 スキルが合う)
    loc1: u32,
    /// r1 % width1 == rem1 なら第 1 スキルのポイントが合う
    width1: u32,
    rem1: u32,
    /// 第 2 スキルが表の何番目か (読んだ護石に第 2 スキルがあるとき)
    loc2: Option<u32>,
}

impl Quick {
    /// t の種類で charm が出うるなら Some
    fn new(t: &Table, charm: &Charm) -> Option<Self> {
        let id1 = t.skill1.iter().position(|&s| s == charm.skill1)?;
        let (lo, hi) = t.pts1[id1];
        if !(lo as i8..=hi as i8).contains(&charm.pts1) {
            return None;
        }
        let loc2 = match charm.skill2 {
            Some(s2) => Some(t.skill2.iter().position(|&s| s == s2)? as u32),
            None => None,
        };
        Some(Self {
            loc1: id1 as u32,
            width1: (hi - lo + 1) as u32,
            rem1: (charm.pts1 - lo as i8) as u32,
            loc2,
        })
    }
}

/// マカフシギ錬金で出るスキルと、第 1・第 2 スキルとしてのポイントの範囲 (スキル番号の小さい順)。
/// 錬金の表 (風化した護石と古びた護石) から求める。フレームの探索と同じ表なので、画面の選択肢と食い違わない
pub fn skill_info() -> Vec<SkillInfo> {
    crate::skills()
        .iter()
        .map(|&(id, name)| {
            let mut skill1 = None;
            let mut skill2 = None;
            for t in [&WEATHERED, &WORN] {
                if let Some(i) = t.skill1.iter().position(|&s| s == id) {
                    let (lo, hi) = t.pts1[i];
                    skill1 = union_range(
                        skill1,
                        Some(PointRange {
                            min: lo as i8,
                            max: hi as i8,
                        }),
                    );
                }
                if let Some(i) = t.skill2.iter().position(|&s| s == id) {
                    skill2 = union_range(
                        skill2,
                        Some(PointRange {
                            min: 1,
                            max: t.pts2[i].1 as i8,
                        }),
                    );
                }
            }
            SkillInfo {
                id,
                name,
                skill1,
                skill2,
            }
        })
        .collect()
}

// 割る数を定数にして、割り算を掛け算に置き換えられるようにする
const LEN1: [u32; 2] = [WEATHERED.skill1.len() as u32, WORN.skill1.len() as u32];
const LEN2: [u32; 2] = [WEATHERED.skill2.len() as u32, WORN.skill2.len() as u32];
const NO_SKILL2: [u32; 2] = [WEATHERED.no_skill2, WORN.no_skill2];

/// 護石が乱数の列のある位置から作られる見込み (錬金の表からの大まかな値。スロットは見ない)。
/// 小さいほど、乱数の列を流したときに見つかる位置が少ない
fn density(quick: &[Option<Quick>; 2]) -> f64 {
    (0..2)
        .filter_map(|k| {
            let q = quick[k].as_ref()?;
            let skill2 = q.loc2.map_or(1.0, |_| {
                f64::from(100 - NO_SKILL2[k]) / 100.0 / f64::from(LEN2[k])
            });
            Some(skill2 / f64::from(LEN1[k]) / f64::from(q.width1))
        })
        .sum()
}

/// 護石ごとの位置の候補 (options[k] は k 個目の護石が出る何個目) から、全部が別の位置になる選び方を 1 つ返す。
/// 護石も位置も 4 つまでなので、全部試しても軽い
fn distinct_positions(options: &[Vec<u8>]) -> Option<Vec<u8>> {
    fn pick(options: &[Vec<u8>], chosen: &mut Vec<u8>) -> bool {
        let Some((first, rest)) = options.split_first() else {
            return true;
        };
        for &p in first {
            if chosen.contains(&p) {
                continue;
            }
            chosen.push(p);
            if pick(rest, chosen) {
                return true;
            }
            chosen.pop();
        }
        false
    }
    let mut chosen = Vec::new();
    pick(options, &mut chosen).then_some(chosen)
}

/// [start, end) のフレームのうち、charms が全部 (index があればその何個目に、全部が別の位置に) 出るものを、
/// 護石ごとの何個目と一緒に返す (フレームの小さい順)。charms が空なら空。
///
/// index は 1 回の錬金の中で何個目か (1〜4)。錬金を予約してまとめて行うと、鑑定ボックスには何回分もの護石が
/// 続けて並ぶので、画面で読める何個目は 5 以上になりうる。そのときは 1 回の錬金の中での位置がわからないので、
/// 呼ぶ側で指定なし (None) にして渡す (候補は増えるが、取りこぼさない)。1〜4 以外の index はエラー
///
/// フレームごとに護石の並びをたどると重いので、逆からたどる: 乱数の列を 1 回だけ流して「この位置の乱数から作る
/// 護石が手がかりの護石になる」位置 p を探し (第 1 スキルが合うだけで 1/60 程度なので、まれ)、見つかったら p を
/// 通りうる錬金のフレーム (p の 0〜18 個手前) を挙げる。挙げたフレームは、錬金の乱数がそろったところで護石を
/// 作り直し、全部の護石が出るかを確かめる。手がかりには、いちばん出にくそうな護石を使う
pub fn search_frames(
    charms: &[SearchCharm],
    start: u32,
    end: u32,
) -> Result<Vec<FrameMatch>, InputError> {
    if start > end {
        return Err(InputError::FrameRange { start, end });
    }
    if let Some(index) = charms
        .iter()
        .filter_map(|c| c.index)
        .find(|n| !(1..=MAX_CHARMS).contains(n))
    {
        return Err(InputError::Index { index });
    }
    let wants: Vec<(&Charm, Option<u8>)> = charms.iter().map(|c| (&c.charm, c.index)).collect();
    let tables = [&WEATHERED, &WORN];
    let quicks: Vec<[Option<Quick>; 2]> = wants
        .iter()
        .map(|(c, _)| tables.map(|t| Quick::new(t, c)))
        .collect();
    // どれか 1 つでもマカ錬金で出ない護石なら、どのフレームにも出ない
    if wants.is_empty() || quicks.iter().any(|q| q.iter().all(Option::is_none)) || start == end {
        return Ok(Vec::new());
    }
    let lead = (0..wants.len())
        .min_by(|&a, &b| density(&quicks[a]).total_cmp(&density(&quicks[b])))
        .unwrap();
    let (charm, index) = wants[lead];
    let quick = &quicks[lead];
    let loc1 = quick
        .each_ref()
        .map(|q| q.as_ref().map_or(u32::MAX, |q| q.loc1));
    // 位置は u = フレーム - (start - 1) で数える。v[u % WINDOW] がその位置の乱数。
    // 錬金したフレーム F (u = 1 ..= n) の護石は u = F .. F + 18 から始まり、その乱数は 7 個先まで使う
    let n = (end - start) as usize;
    let last_p = n + MAX_OFFSET;
    let mut v = [0u32; WINDOW];
    let mut s = state(start as i64 - 1);
    // 手がかりの護石が出うる錬金のフレーム (u)。作り直すのに要る乱数 (u - 1 .. u + MAX_OFFSET + 6) がそろったら
    // 確かめる。p から挙げるフレームは p - 18 以上なので、確かめ終えたフレームより前のものが後から来ることはない
    let mut pending = std::collections::BTreeSet::new();
    let mut found = Vec::new();
    for q in 0..=last_p + 6 {
        v[q % WINDOW] = s[3];
        s = ascend(s);
        let at = |u: usize| v[u % WINDOW];
        // 乱数が 7 個そろった位置 p から作る護石を見る
        if let Some(p) = q.checked_sub(6).filter(|&p| p >= 1) {
            for k in 0..2 {
                let Some(qk) = &quick[k] else { continue };
                if at(p) % LEN1[k] != loc1[k]
                    || at(p + 1) % qk.width1 != qk.rem1
                    || qk
                        .loc2
                        .is_some_and(|l| at(p + 2) % 100 < NO_SKILL2[k] || at(p + 3) % LEN2[k] != l)
                {
                    continue;
                }
                let r: [u32; 7] = std::array::from_fn(|d| at(p + d));
                if !matches(&tables[k].make(&r), charm) {
                    continue;
                }
                // p を k の種類の護石として通る錬金のフレームを探す
                for f in p.saturating_sub(MAX_OFFSET).max(1)..=p.min(n) {
                    let (n1, n2) = counts(at(f - 1));
                    let mut pos = f;
                    for j in 0..n1 + n2 {
                        if pos >= p {
                            if pos == p
                                && usize::from(j >= n1) == k
                                && index.is_none_or(|i| j + 1 == i as usize)
                            {
                                pending.insert(f);
                            }
                            break;
                        }
                        pos += tables[usize::from(j >= n1)].stride(at(pos + 2));
                    }
                }
            }
        }
        while let Some(&u) = pending.first() {
            if u + MAX_OFFSET + 6 > q {
                break;
            }
            pending.pop_first();
            let made = meld(|d| at(u - 1 + d));
            // 護石ごとに、作った護石のうち何個目に出るか
            let indexes: Vec<Vec<u8>> = wants
                .iter()
                .map(|(c, index)| {
                    (0..made.len())
                        .filter(|&j| {
                            index.is_none_or(|i| j + 1 == i as usize) && matches(&made[j].1, c)
                        })
                        .map(|j| j as u8 + 1)
                        .collect()
                })
                .collect();
            if let Some(positions) = distinct_positions(&indexes) {
                found.push(FrameMatch {
                    frame: start + (u - 1) as u32,
                    indexes,
                    positions,
                });
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::json_fixture;

    /// mhxx-rng の番号の並び (-1 = なし) から
    fn charm(v: &serde_json::Value) -> Charm {
        let n = |i: usize| v[i].as_i64().unwrap();
        Charm {
            skill1: n(0) as u16,
            pts1: n(1) as i8,
            skill2: (n(2) >= 0).then(|| n(2) as u16),
            pts2: (n(2) >= 0).then(|| n(3) as i8),
            slots: n(4) as u8,
        }
    }

    /// 護石 1 個で探した結果 (フレームと、何個目の一覧)
    type Found = Vec<(u32, Vec<u8>)>;

    /// 護石 1 個で探す
    fn search_one(c: &Charm, index: Option<u8>, start: u32, end: u32) -> Found {
        let charms = [SearchCharm {
            charm: c.clone(),
            index,
        }];
        search_frames(&charms, start, end)
            .unwrap()
            .into_iter()
            .map(|m| {
                assert_eq!(m.positions, [m.indexes[0][0]], "{m:?}");
                (m.frame, m.indexes.into_iter().next().unwrap())
            })
            .collect()
    }

    /// 画面に出る中身 (第 1 スキルと同じ第 2 スキルは出ない)
    fn shown(c: &Generated) -> Charm {
        Charm {
            skill1: c.skill1,
            pts1: c.pts1,
            skill2: c.skill2.filter(|_| c.shows_skill2()),
            pts2: c.shows_skill2().then_some(c.pts2),
            slots: c.slots,
        }
    }

    /// 錬金で出る護石が、mhxx-rng (Python) と同じ。錬金するフレームは 0〜3 と 1 億の手前、乱数で選んだ 60 個と、
    /// 全スキルを通すために足した 82 個 (合わせて 147 回の錬金、護石 526 個)
    #[test]
    fn melding_matches_mhxx_rng() {
        let f = json_fixture("rng/melding.json");
        for m in f["meldings"].as_array().unwrap() {
            let frame = m["frame"].as_u64().unwrap() as u32;
            let got: Vec<[i64; 6]> = melding(frame)
                .iter()
                .map(|(k, c)| {
                    [
                        *k as i64,
                        c.skill1 as i64,
                        c.pts1 as i64,
                        c.skill2.map_or(-1, |s| s as i64),
                        c.pts2 as i64,
                        c.slots as i64,
                    ]
                })
                .collect();
            let want: Vec<[i64; 6]> = m["charms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| std::array::from_fn(|i| c[i].as_i64().unwrap()))
                .collect();
            assert_eq!(got, want, "フレーム {frame}");
        }
    }

    /// 上のテストの錬金の結果に、2 種類の表の第 1 スキルと第 2 スキルがどれも最低 1 回は出てくる (表のどの行も
    /// 一度は通っている)。正解を作り直して足りなくなったら、ここで気づく
    #[test]
    fn melding_cases_cover_every_skill() {
        let f = json_fixture("rng/melding.json");
        let mut seen = std::collections::HashSet::new();
        for m in f["meldings"].as_array().unwrap() {
            for c in m["charms"].as_array().unwrap() {
                let n = |i: usize| c[i].as_i64().unwrap();
                seen.insert((n(0), 1, n(1)));
                if n(3) >= 0 {
                    seen.insert((n(0), 2, n(3)));
                }
            }
        }
        for (k, t) in [&WEATHERED, &WORN].iter().enumerate() {
            let k = k as i64;
            let miss1: Vec<u16> = t
                .skill1
                .iter()
                .copied()
                .filter(|&s| !seen.contains(&(k, 1, s as i64)))
                .collect();
            let miss2: Vec<u16> = t
                .skill2
                .iter()
                .copied()
                .filter(|&s| !seen.contains(&(k, 2, s as i64)))
                .collect();
            assert!(
                miss1.is_empty() && miss2.is_empty(),
                "種類 {k}: 第 1 スキルで出ていない {miss1:?}、第 2 スキルで出ていない {miss2:?}"
            );
        }
    }

    /// フレームの探索が、experimental-charm-reader の frames.py (mhxx-rng の関数を使う) と同じ
    #[test]
    fn search_matches_python() {
        let f = json_fixture("rng/melding.json");
        for case in f["searches"].as_array().unwrap() {
            let c = charm(&case["charm"]);
            let index = case["index"].as_u64().map(|i| i as u8);
            let (start, end) = (
                case["start"].as_u64().unwrap() as u32,
                case["end"].as_u64().unwrap() as u32,
            );
            // 正解は [[フレーム, [何個目, ...]], ...]
            let want: Found = case["found"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    let indexes = v[1]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|i| i.as_u64().unwrap() as u8)
                        .collect();
                    (v[0].as_u64().unwrap() as u32, indexes)
                })
                .collect();
            let got = search_one(&c, index, start, end);
            assert_eq!(got, want, "{c:?} 何個目 {index:?}");
        }
    }

    /// 錬金で出る護石の作り方と探索が食い違わない: 出た護石を探すと、そのフレームが見つかる
    #[test]
    fn search_finds_generated_charms() {
        for frame in [0, 1, 5, 1000, 123_456] {
            for (j, (_, c)) in melding(frame).iter().enumerate() {
                let want = shown(c);
                let lo = frame.saturating_sub(3);
                let got = search_one(&want, Some(j as u8 + 1), lo, frame + 3);
                assert!(
                    got.iter()
                        .any(|(g, indexes)| *g == frame && indexes == &[j as u8 + 1]),
                    "フレーム {frame} の {} 個目 {want:?}: {got:?}",
                    j + 1
                );
            }
        }
    }

    /// 1 フレームずつ素直に護石を作って探したものと同じ (範囲の端、何個目の指定あり・なし、第 2 スキルなしを含む)
    #[test]
    fn search_matches_brute_force() {
        let (start, end) = (0u32, 30_000u32);
        let all: Vec<Vec<(u8, Generated)>> = (start..end + 1).map(melding).collect();
        // よく出る護石 (第 2 スキルなし) と、まれな護石を、いくつかのフレームから選ぶ
        let picks = [0usize, 1, 17, 4321, 29_999];
        for &f in &picks {
            for (_, c) in &all[f] {
                let want = shown(c);
                for index in [None, Some(1), Some(2), Some(3), Some(4)] {
                    // フレームごとに、want が出る何個目を全部集める
                    let brute: Found = (start..end)
                        .filter_map(|g| {
                            let indexes: Vec<u8> = all[g as usize]
                                .iter()
                                .enumerate()
                                .filter(|(j, (_, x))| {
                                    index.is_none_or(|i| j + 1 == i as usize) && shown(x) == want
                                })
                                .map(|(j, _)| j as u8 + 1)
                                .collect();
                            (!indexes.is_empty()).then_some((g, indexes))
                        })
                        .collect();
                    assert_eq!(
                        search_one(&want, index, start, end),
                        brute,
                        "{want:?} 何個目 {index:?}"
                    );
                    let (a, b) = (start + 7, end - 13);
                    let sub: Found = brute
                        .iter()
                        .filter(|(g, _)| (a..b).contains(g))
                        .cloned()
                        .collect();
                    assert_eq!(
                        search_one(&want, index, a, b),
                        sub,
                        "{want:?} 何個目 {index:?} 範囲 {a}..{b}"
                    );
                }
            }
        }
    }

    /// mhxx-rng の「有名なお守り」(docs/charms.md) を 0〜10 億フレームで探す。答えが Python 版 (frames.py) と同じで、
    /// 一覧のフレーム (護石が始まる位置) の 0〜18 個手前にある答えの数が、一覧の「当たりフレーム」と同じ (6 件)
    #[test]
    fn famous_charms_in_a_billion_frames() {
        let f = json_fixture("rng/famous.json");
        for case in f.as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let want: Vec<u32> = case["frames"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect();
            let t = std::time::Instant::now();
            // 正解はフレームだけ (何個目は Python 版との比較と総当たりのテストで見ている)
            let got: Vec<u32> = search_one(&charm(&case["charm"]), None, 0, 1_000_000_000)
                .iter()
                .map(|(g, _)| *g)
                .collect();
            println!(
                "{name}: {} 件、{:.1} 秒",
                got.len(),
                t.elapsed().as_secs_f64()
            );
            assert_eq!(got, want, "{name}");
            let pos = case["frame"].as_u64().unwrap() as u32;
            let near = got
                .iter()
                .filter(|&&g| g + MAX_OFFSET as u32 >= pos && g <= pos)
                .count();
            assert_eq!(
                near as u64,
                case["hits"].as_u64().unwrap(),
                "{name}: 当たりフレームの数"
            );
        }
    }

    /// 錬金の表に出てくるスキルが、スキルの一覧 (skills.rs) とちょうど同じ
    #[test]
    fn tables_cover_the_skill_list() {
        let mut ids: Vec<u16> = [&WEATHERED, &WORN]
            .iter()
            .flat_map(|t| t.skill1.iter().chain(t.skill2))
            .copied()
            .collect();
        ids.sort();
        ids.dedup();
        let list: Vec<u16> = crate::skills().iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, list);
    }

    /// 何個目が 1〜4 でない (0 や、鑑定ボックスで読んだ 5 以上の位置をそのまま渡した) ならエラー。
    /// 複数の護石のうち 1 個だけ外れていてもエラー
    #[test]
    fn index_outside_one_melding_is_an_error() {
        let c = Charm {
            skill1: 106,
            pts1: 4,
            skill2: None,
            pts2: None,
            slots: 1,
        }; // 跳躍4 S1
        for index in [0, 5, 12, 40] {
            let charms = [
                SearchCharm {
                    charm: c.clone(),
                    index: Some(1),
                },
                SearchCharm {
                    charm: c.clone(),
                    index: Some(index),
                },
            ];
            assert_eq!(
                search_frames(&charms, 0, 200_000),
                Err(InputError::Index { index }),
                "何個目 {index}"
            );
            assert_eq!(
                search_frames(&charms[1..], 0, 200_000),
                Err(InputError::Index { index }),
                "何個目 {index}"
            );
        }
    }

    /// スキルごとのポイントの範囲: どのスキルも第 1 か第 2 のどちらかにはなれ、錬金の結果 (526 個の護石) の
    /// ポイントはどれも範囲に入る。例として、ガード性能の第 1 スキルは、風化した護石の 3〜7 と古びた護石の 1〜6 を合わせた 1〜7
    #[test]
    fn skill_ranges_cover_generated_charms() {
        let info = skill_info();
        assert_eq!(info.len(), crate::skills().len());
        assert!(
            info.iter()
                .all(|s| s.skill1.is_some() || s.skill2.is_some())
        );
        let by_id = |id: u16| info.iter().find(|s| s.id == id).unwrap();
        assert_eq!(by_id(84).skill1, Some(PointRange { min: 1, max: 7 }));
        let f = json_fixture("rng/melding.json");
        for m in f["meldings"].as_array().unwrap() {
            for c in m["charms"].as_array().unwrap() {
                let n = |i: usize| c[i].as_i64().unwrap();
                let s1 = by_id(n(1) as u16);
                assert!(s1.skill1.is_some_and(|r| r.contains(n(2) as i8)), "{c}");
                if n(3) >= 0 {
                    let s2 = by_id(n(3) as u16);
                    assert!(s2.skill2.is_some_and(|r| r.contains(n(4) as i8)), "{c}");
                }
            }
        }
    }

    #[test]
    fn reversed_range_is_an_error() {
        let c = Charm {
            skill1: 4,
            pts1: 3,
            skill2: None,
            pts2: None,
            slots: 0,
        };
        let charms = [SearchCharm {
            charm: c,
            index: None,
        }];
        assert_eq!(
            search_frames(&charms, 10, 5),
            Err(InputError::FrameRange { start: 10, end: 5 })
        );
        assert_eq!(search_frames(&charms, 5, 5), Ok(Vec::new()));
        assert_eq!(search_frames(&[], 0, 100), Ok(Vec::new()));
    }

    /// 錬金で出た護石 made のうち、want が何個目に出るか (index があればその位置だけ)
    fn places(made: &[(u8, Generated)], want: &SearchCharm) -> Vec<u8> {
        (0..made.len())
            .filter(|&j| {
                want.index.is_none_or(|i| j + 1 == i as usize) && shown(&made[j].1) == want.charm
            })
            .map(|j| j as u8 + 1)
            .collect()
    }

    /// 複数の護石で探した 1 件が、そのフレームの錬金を作り直したものと合う: 何個目が護石ごとに合い、
    /// 位置の選び方がそれぞれの護石の出る位置で、全部が別
    fn assert_match_is_consistent(m: &FrameMatch, charms: &[SearchCharm]) {
        let made = melding(m.frame);
        let want: Vec<Vec<u8>> = charms.iter().map(|c| places(&made, c)).collect();
        assert_eq!(m.indexes, want, "{m:?}");
        assert!(
            m.positions
                .iter()
                .zip(&m.indexes)
                .all(|(p, ix)| ix.contains(p)),
            "{m:?}"
        );
        assert!(
            (1..m.positions.len()).all(|i| !m.positions[..i].contains(&m.positions[i])),
            "{m:?}"
        );
    }

    /// 複数の護石で探したものが、1 フレームずつ素直に護石を作って、全部が別の位置に出るか確かめたものと同じ
    /// (同じ錬金の護石の組・ほかの錬金の護石との組・同じ中身の護石を 2 個・何個目の指定あり・なしを含む)。
    /// 範囲は 0 からと、10 億の手前 (大きなフレームでも、作り直す錬金のフレームの数え方がずれない)
    #[test]
    fn multi_search_matches_brute_force() {
        for start in [0u32, 999_970_000] {
            let end = start + 30_000;
            let all: Vec<Vec<(u8, Generated)>> = (start..end).map(melding).collect();
            // f は start からの何フレーム目か
            let pick = |f: usize, j: usize, index: bool| SearchCharm {
                charm: shown(&all[f][j].1),
                index: index.then_some(j as u8 + 1),
            };
            let cases: Vec<Vec<SearchCharm>> = vec![
                vec![pick(17, 0, true), pick(17, 1, true)],
                vec![pick(17, 0, false), pick(17, 1, false), pick(17, 2, false)],
                vec![pick(4321, 1, false), pick(4321, 2, true)],
                vec![pick(29_000, 0, false), pick(123, 1, false)],
                vec![pick(500, 0, false), pick(500, 0, false)],
                (0..all[2000].len()).map(|j| pick(2000, j, false)).collect(),
            ];
            for charms in &cases {
                let brute: Vec<(u32, Vec<Vec<u8>>)> = (start..end)
                    .filter_map(|g| {
                        let made = &all[(g - start) as usize];
                        let indexes: Vec<Vec<u8>> =
                            charms.iter().map(|c| places(made, c)).collect();
                        distinct_positions(&indexes).map(|_| (g, indexes))
                    })
                    .collect();
                let got = search_frames(charms, start, end).unwrap();
                for m in &got {
                    assert_match_is_consistent(m, charms);
                }
                let got: Vec<(u32, Vec<Vec<u8>>)> =
                    got.into_iter().map(|m| (m.frame, m.indexes)).collect();
                assert_eq!(got, brute, "{start}..{end} {charms:?}");
            }
        }
    }

    /// 0〜10 億フレームを通しで、複数の護石で探す。10 億の手前で錬金した護石を全部 (何個目の指定あり) 渡すと、
    /// そのフレームが見つかり、見つかったどのフレームも錬金を作り直したものと合う。
    /// 範囲全体を流すのは護石の数によらず 1 回 (10 億フレームの単独のテストの 1 件分)
    #[test]
    fn multi_charms_in_a_billion_frames() {
        let frame = 999_999_000;
        let charms: Vec<SearchCharm> = melding(frame)
            .iter()
            .enumerate()
            .map(|(j, (_, c))| SearchCharm {
                charm: shown(c),
                index: Some(j as u8 + 1),
            })
            .collect();
        let got = search_frames(&charms, 0, 1_000_000_000).unwrap();
        let hit = got.iter().find(|m| m.frame == frame);
        assert!(hit.is_some(), "フレーム {frame} が見つからない: {got:?}");
        let order: Vec<u8> = (1..=charms.len() as u8).collect();
        assert_eq!(hit.unwrap().positions, order);
        for m in &got {
            assert_match_is_consistent(m, &charms);
        }
    }
}
