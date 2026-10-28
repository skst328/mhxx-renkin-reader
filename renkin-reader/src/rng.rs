//! お守りが出る錬金のフレームを探す (マカフシギ錬金。投入は最高ランク)。乱数とお守りの作り方は mhxx-rand
//! (apmnnn/mhxx-rng の移植) を使い、ここでは画面とやり取りする型との間を受け渡すだけにする

use mhxx_rand as rand;

use crate::types::{Charm, FrameMatch, InputError, PointRange, SearchCharm, SkillInfo};

/// 1 回の錬金で出る護石の最大の数 (風化 2 個 + 古びた 2 個)
const MAX_CHARMS: u8 = 4;

/// マカフシギ錬金で出るスキル (番号の小さい順) と、第 1・第 2 スキルとしてのポイントの範囲。名前は正式名称
/// (鑑定画面の表記)。フレームの探索と同じ表から求めるので、画面の選択肢と食い違わない
pub fn skill_info() -> Vec<SkillInfo> {
    let range = |r: &Option<std::ops::RangeInclusive<i8>>| {
        r.as_ref().map(|r| PointRange {
            min: *r.start(),
            max: *r.end(),
        })
    };
    rand::skill_info(rand::Melding::Juju)
        .iter()
        .map(|i| SkillInfo {
            id: i.skill.id(),
            name: i.skill.full_name(),
            skill1: range(&i.pts1),
            skill2: range(&i.pts2),
        })
        .collect()
}

/// 画面の護石を mhxx-rand の探す条件にする。マカフシギ錬金で出ない護石 (お守りに付かないスキル、
/// 第 2 スキルとポイントの片方だけ、表にない組み合わせ) なら None
fn query(c: &Charm) -> Option<rand::Query> {
    let skill1 = rand::Skill::from_id(c.skill1)?;
    let skill2 = match (c.skill2, c.pts2) {
        (Some(s), Some(p)) => Some((rand::Skill::from_id(s)?, p)),
        (None, _) => None,
        (Some(_), None) => return None,
    };
    let charm = rand::Charm {
        skill1,
        pts1: c.pts1,
        skill2,
        slots: c.slots,
    };
    rand::Query::new(rand::Melding::Juju, rand::Rank::MAX, None, &charm).ok()
}

/// [start, end) のフレームのうち、charms が全部 (index があればその何個目に、全部が別の位置に) 出るものを、
/// 護石ごとの何個目と一緒に返す (フレームの小さい順)。charms が空なら空。どれか 1 つでもマカフシギ錬金で
/// 出ない護石なら、どのフレームにも出ないので空。
///
/// index は 1 回の錬金の中で何個目か (1〜4)。錬金を予約してまとめて行うと、鑑定ボックスには何回分もの護石が
/// 続けて並ぶので、画面で読める何個目は 5 以上になりうる。そのときは 1 回の錬金の中での位置がわからないので、
/// 呼ぶ側で指定なし (None) にして渡す (候補は増えるが、取りこぼさない)。1〜4 以外の index はエラー
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
    let Some(targets) = charms
        .iter()
        .map(|c| {
            query(&c.charm).map(|query| rand::MeldingTarget {
                query,
                index: c.index,
            })
        })
        .collect::<Option<Vec<_>>>()
    else {
        return Ok(Vec::new());
    };
    // 何個目と錬金のやり方は確かめてあるので、エラーにはならない
    let found = rand::search_meldings(&targets, u64::from(start)..u64::from(end))
        .expect("何個目と錬金のやり方は確かめてある");
    Ok(found
        .into_iter()
        .map(|m| FrameMatch {
            frame: m.frame as u32,
            indexes: m.indexes,
            positions: m.positions,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::json_fixture;

    /// 錬金したフレームから、最後の護石の最初の乱数までの最大の間 (護石は最大 4 個で、1 個につき乱数を最大 6 個使う)
    const MAX_OFFSET: u32 = 3 * 6;

    /// フレーム frame で錬金したときに出る護石 (鑑定ボックスの並び順) と、その種類 (0 = 風化、1 = 古びた)
    fn melding(frame: u32) -> Vec<(u8, rand::Generated)> {
        rand::meld(rand::Melding::Juju, rand::Rank::MAX, u64::from(frame))
            .into_iter()
            .map(|(k, c)| (u8::from(k == rand::Kind::Worn), c))
            .collect()
    }

    /// 護石ごとの位置の候補 (options[k] は k 個目の護石が出る何個目) から、全部が別の位置になる選び方を 1 つ返す
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
    fn shown(c: &rand::Generated) -> Charm {
        let s = c.shown();
        Charm {
            skill1: s.skill1.id(),
            pts1: s.pts1,
            skill2: s.skill2.map(|(k, _)| k.id()),
            pts2: s.skill2.map(|(_, p)| p),
            slots: s.slots,
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
                        c.skill1.id() as i64,
                        c.pts1 as i64,
                        c.skill2.map_or(-1, |(s, _)| s.id() as i64),
                        c.skill2.map_or(0, |(_, p)| p as i64),
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
        let all: Vec<Vec<(u8, rand::Generated)>> = (start..end + 1).map(melding).collect();
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
                .filter(|&&g| g + MAX_OFFSET >= pos && g <= pos)
                .count();
            assert_eq!(
                near as u64,
                case["hits"].as_u64().unwrap(),
                "{name}: 当たりフレームの数"
            );
        }
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
        assert_eq!(info.len(), 134);
        assert!(info.windows(2).all(|w| w[0].id < w[1].id));
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
    fn places(made: &[(u8, rand::Generated)], want: &SearchCharm) -> Vec<u8> {
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
            let all: Vec<Vec<(u8, rand::Generated)>> = (start..end).map(melding).collect();
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
