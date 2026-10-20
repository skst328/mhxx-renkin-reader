// ページ側から Worker (src/workers/reader.worker.ts) に読み取りとフレーム探索を頼む。
// Worker はページを開いている間 1 つだけ。頼んだ順に 1 つずつ処理される
import type { Charm, FrameCandidate, ReadResult, SkillInfo } from '../../renkin-reader/pkg/renkin_reader.js'
import type { WorkerRequest, WorkerResponse } from '../workers/reader.worker'

/** 頼む内容 (id は call が付ける)。ユニオンの各要素から id を外す */
type Request = WorkerRequest extends infer R ? (R extends unknown ? Omit<R, 'id'> : never) : never

type Pending = {
  resolve: (value: WorkerResponse) => void
  reject: (error: Error) => void
  /** 探索の進み具合 (探し終えたフレームの数, 全体) */
  onProgress?: (done: number, total: number) => void
}

let worker: Worker | null = null
let skillsReady: Promise<SkillInfo[]> | null = null
const pending = new Map<number, Pending>()
let nextId = 1

function start(): Promise<SkillInfo[]> {
  if (skillsReady) return skillsReady
  skillsReady = new Promise((resolveSkills, rejectSkills) => {
    worker = new Worker(new URL('../workers/reader.worker.ts', import.meta.url), { type: 'module' })
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const message = event.data
      if (message.type === 'ready') {
        resolveSkills(message.skills)
        return
      }
      const p = pending.get(message.id)
      if (!p) return
      if (message.type === 'progress') {
        p.onProgress?.(message.done, message.total)
        return
      }
      pending.delete(message.id)
      if (message.type === 'error') p.reject(new Error(message.message))
      else if (message.type === 'cancelled') p.reject(new DOMException('中止しました', 'AbortError'))
      else p.resolve(message)
    }
    worker.onerror = (event) => {
      const error = new Error(event.message || '読み取りの準備に失敗しました')
      rejectSkills(error)
      for (const p of pending.values()) p.reject(error)
      pending.clear()
    }
  })
  return skillsReady
}

function call(
  request: Request,
  options: { onProgress?: Pending['onProgress']; signal?: AbortSignal } = {},
): Promise<WorkerResponse> {
  return start().then(
    () =>
      new Promise((resolve, reject) => {
        const id = nextId++
        pending.set(id, { resolve, reject, onProgress: options.onProgress })
        // 中止は Worker に頼み、区切りの合間に止まったら cancelled が返る
        options.signal?.addEventListener('abort', () => worker!.postMessage({ id: nextId++, type: 'cancel', target: id }), {
          once: true,
        })
        worker!.postMessage({ ...request, id } as WorkerRequest)
      }),
  )
}

/** wasm を読み込み、マカフシギ錬金で出るスキルとポイントの範囲を返す (軽い。重い準備は最初の読み取りで行う) */
export function loadSkills(): Promise<SkillInfo[]> {
  return start()
}

/** 写真を読む。位置合わせ後の画面は PNG の Blob (撮り直しなら null) */
export async function readPhoto(file: Blob): Promise<{ result: ReadResult; aligned: Blob | null }> {
  const r = await call({ type: 'read', file })
  if (r.type !== 'read') throw new Error('読み取りの応答が不正')
  return { result: r.result, aligned: r.aligned }
}

/** 錬金時のフレームを探す。onProgress に進み具合 (探し終えたフレームの数, 全体) が届く。signal で中止できる
 * (中止したら AbortError で失敗する) */
export async function searchFrames(
  charm: Charm,
  index: number | null,
  start: number,
  end: number,
  options: { onProgress?: (done: number, total: number) => void; signal?: AbortSignal } = {},
): Promise<FrameCandidate[]> {
  const r = await call({ type: 'search', charm, index, start, end }, options)
  if (r.type !== 'search') throw new Error('探索の応答が不正')
  return r.found
}
