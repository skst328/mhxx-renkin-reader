// 一時的なデバッグ (公開前の確認用)。URL に ?debug を付けたときだけ、ページの下に計測と写真の開き方を出す。
// スマホでの速さと、iPad の HEIC がどう扱われるかを実機で確かめるためのもの
import type { Timing } from '@/lib/types'
import type { ReadDebug } from '../workers/reader.worker'

export const DEBUG = new URLSearchParams(window.location.search).has('debug')

/** 写真を開こうとした 1 回分 */
export type DecodeAttempt = { method: string; ok: boolean; ms?: number; detail?: string }

/** 写真 1 枚の読み取り */
export type PhotoDebug = {
  name: string
  type: string
  size: number
  attempts: DecodeAttempt[]
  /** Worker が測った時間 (読み取りまで進んだときだけ) */
  worker?: ReadDebug
  /** Rust が返した段階ごとの時間 */
  timings?: Timing[]
  result?: string
  error?: string
  /** 写真を選んでから結果が出るまで (ページ側で測った時間) */
  totalMs: number
}

/** フレーム探索 1 回分 */
export type SearchDebug = { frames: number; ms: number; found: number; cancelled: boolean }

/** 同じ段階を足し合わせる (試し直しなどで同じ段階が何度も出る)。出てきた順に並べる */
export function aggregate(timings: Timing[]): { stage: string; ms: number; n: number }[] {
  const sums = new Map<string, { ms: number; n: number }>()
  for (const { stage, ms } of timings) {
    const s = sums.get(stage) ?? { ms: 0, n: 0 }
    sums.set(stage, { ms: s.ms + ms, n: s.n + 1 })
  }
  return [...sums].map(([stage, s]) => ({ stage, ...s }))
}

const ms = (v: number) => `${v.toFixed(0)} ms`

/** 割合は total (Worker で測った実際の時間) に対して出す。段階は入れ子になることがあり
 * (init.reference の中で sift.* も記録される)、足すと二重に数えるため */
const table = (timings: Timing[], total: number) =>
  aggregate(timings).map(
    (s) =>
      `  ${s.stage.padEnd(20)} ${ms(s.ms).padStart(9)} ${((s.ms / total) * 100).toFixed(1).padStart(5)}%${s.n > 1 ? ` ×${s.n}` : ''}`,
  )

/** 端末の情報 */
export function environment(): string[] {
  const nav = navigator as Navigator & { deviceMemory?: number }
  return [
    `version  v${__APP_VERSION__}`,
    `UA       ${nav.userAgent}`,
    `cores    ${nav.hardwareConcurrency ?? '?'}`,
    `memory   ${nav.deviceMemory ? `${nav.deviceMemory} GB` : '?'}`,
    `screen   ${window.screen.width}×${window.screen.height} @${window.devicePixelRatio}x`,
  ]
}

/** 読み取りの計測を、そのまま貼り付けられる文字にする */
export function formatPhoto(d: PhotoDebug): string[] {
  const lines = [
    `file     ${d.name}`,
    `type     ${d.type || '(空)'}`,
    `size     ${d.size.toLocaleString()} B`,
    ...d.attempts.map(
      (a) =>
        `decode   ${a.method}: ${a.ok ? 'OK' : 'NG'}${a.ms !== undefined ? ` (${ms(a.ms)})` : ''}${a.detail ? ` ${a.detail}` : ''}`,
    ),
  ]
  const w = d.worker
  if (w) {
    lines.push(
      `image    ${w.width}×${w.height} (${((w.width * w.height) / 1e6).toFixed(1)} MP)`,
      `result   ${d.result}`,
      '',
      `decode   ${ms(w.decodeMs)}`,
      `init     ${w.initMs === null ? '(済み)' : ms(w.initMs)}`,
      `read     ${ms(w.readMs)}`,
      `png      ${ms(w.pngMs)}`,
    )
  }
  if (d.error) lines.push(`error    ${d.error}`)
  lines.push(`total    ${ms(d.totalMs)} (写真を選んでから結果まで)`)
  if (w?.initTimings && w.initMs !== null)
    lines.push('', `init timings (${ms(w.initMs)}、sift.* は init.reference の内訳)`, ...table(w.initTimings, w.initMs))
  if (w && d.timings) lines.push('', `read timings (${ms(w.readMs)})`, ...table(d.timings, w.readMs))
  return lines
}

/** 探索の計測 */
export function formatSearch(d: SearchDebug): string[] {
  return [
    `frames   ${d.frames.toLocaleString()}${d.cancelled ? ' (中止)' : ''}`,
    `time     ${ms(d.ms)}`,
    `speed    ${((d.frames / d.ms) * 1000 / 1e6).toFixed(1)} M frames/s`,
    `found    ${d.found}`,
  ]
}
