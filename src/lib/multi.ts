// 同じ錬金で出た複数の護石から、錬金時のフレームを絞り込む。
// 1 回の錬金で出た護石は、どれも同じフレームから作られる。護石ごとに探した候補 (searchFrames の結果) を突き合わせ、
// 全部の護石が、それぞれ別の位置 (何番目) に出るフレームだけを残す
import type { FrameCandidate } from './types'

/** 複数の護石で絞り込んだ候補: フレームと、護石ごとの位置 (渡した順。1〜4) */
export interface MultiCandidate {
  frame: number
  positions: number[]
}

/** 護石ごとに、その位置の候補 (options[k] は k 個目の護石が出うる位置) から、全部が別の位置になる選び方を 1 つ返す。
 * 無ければ null。護石は 4 個まで、位置も 4 つまでなので、全部試しても軽い */
function distinctPositions(options: number[][]): number[] | null {
  const chosen: number[] = []
  const pick = (k: number): boolean => {
    if (k === options.length) return true
    for (const p of options[k]) {
      if (chosen.includes(p)) continue
      chosen.push(p)
      if (pick(k + 1)) return true
      chosen.pop()
    }
    return false
  }
  return pick(0) ? [...chosen] : null
}

/** 護石ごとの候補 (lists[k] は k 個目の護石の候補。フレームの小さい順) を突き合わせる。
 * 全部の護石の候補にあり、しかも全部が別の位置に出るフレームだけを、フレームの小さい順に返す */
export function intersectCandidates(lists: FrameCandidate[][]): MultiCandidate[] {
  if (lists.length === 0) return []
  const rest = lists.slice(1).map((list) => new Map(list.map((c) => [c.frame, c.indexes])))
  const out: MultiCandidate[] = []
  for (const first of lists[0]) {
    const options = [first.indexes]
    for (const m of rest) {
      const indexes = m.get(first.frame)
      if (!indexes) break
      options.push(indexes)
    }
    if (options.length !== lists.length) continue
    const positions = distinctPositions(options)
    if (positions) out.push({ frame: first.frame, positions })
  }
  return out
}
