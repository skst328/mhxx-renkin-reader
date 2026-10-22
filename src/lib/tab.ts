// 護石のタブ 1 つ分の状態。同じ錬金で出た護石を最大 4 個まで並べ、全部が出るフレームで絞り込む
import type { Draft, Unsure } from './draft'
import type { RejectReason } from './types'

/** 1 回の錬金で出る護石は最大 4 個 */
export const MAX_TABS = 4

export const EMPTY_DRAFT: Draft = { skill1: null, pts1: null, skill2: null, pts2: null, slots: null, index: null }

/** none: 写真なし / reading: 読み取り中 / rejected: 撮り直し / edit: 読み取った (または手で入力する) */
export type Stage = 'none' | 'reading' | 'rejected' | 'edit'

export interface Tab {
  /** タブを見分ける番号 (並び順ではない) */
  id: number
  photo: { url: string; name: string; detail: string } | null
  stage: Stage
  reason: RejectReason | null
  /** 位置合わせした画面の画像 (blob: の URL)。撮り直し・読み取り中は null */
  aligned: string | null
  draft: Draft
  unsure: Unsure
  /** 写真を読み込めなかったときの文 */
  error: string | null
}

export function emptyTab(id: number): Tab {
  return { id, photo: null, stage: 'none', reason: null, aligned: null, draft: EMPTY_DRAFT, unsure: {}, error: null }
}

/** 写真も入力も無いタブ (何枚か選んだ写真の 2 枚目から埋めてよいもの) */
export function isBlank(t: Tab): boolean {
  return t.photo === null && Object.values(t.draft).every((v) => v === null)
}

/** タブが持っている blob: の URL を捨てる (タブを消すときや写真を差し替えるとき) */
export function revokeTab(t: Tab) {
  for (const url of [t.photo?.url, t.aligned]) if (url?.startsWith('blob:')) URL.revokeObjectURL(url)
}
