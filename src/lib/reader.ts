// ページ側から Worker (src/workers/reader.worker.ts) に読み取りとフレーム探索を頼む。
// Worker はページを開いている間 1 つだけ。頼んだ順に 1 つずつ処理される
import type { Charm, FrameCandidate, ReadResult, SkillInfo } from '../../renkin-reader/pkg/renkin_reader.js'
import type { PhotoDebug } from '@/lib/debug'
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

/** wasm の中で落ちたときなどに出す文 (元のエラーは Worker 側でコンソールに出す) */
const BROKEN = '処理中に問題が起きました。もう一度試すと、読み取りの準備からやり直します'

/** Worker で写真を開けなかった。detail は元のエラー (デバッグ用) */
class DecodeFailure extends Error {
  readonly detail: string
  readonly ms: number
  constructor(message: string, detail: string, ms: number) {
    super(message)
    this.detail = detail
    this.ms = ms
  }
}

/** Worker を捨てる。待っている頼みごとは全部失敗にし、次の頼みごとで作り直す */
function discard(error: Error) {
  worker?.terminate()
  worker = null
  skillsReady = null
  for (const p of pending.values()) p.reject(error)
  pending.clear()
}

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
      if (message.type === 'failed') {
        const error = new Error(`読み取りの準備に失敗しました (${message.message})`)
        rejectSkills(error)
        discard(error)
        return
      }
      const p = pending.get(message.id)
      if (!p) return
      if (message.type === 'progress') {
        p.onProgress?.(message.done, message.total)
        return
      }
      pending.delete(message.id)
      if (message.type === 'error' && message.fatal) discard(new Error(BROKEN))
      if (message.type === 'error' && message.decode)
        p.reject(new DecodeFailure(message.message, message.decode.detail, message.decode.ms))
      else if (message.type === 'error') p.reject(new Error(message.fatal ? BROKEN : message.message))
      else if (message.type === 'cancelled') p.reject(new DOMException('中止しました', 'AbortError'))
      else p.resolve(message)
    }
    worker.onerror = (event) => {
      const error = new Error(event.message || '読み取りの準備に失敗しました')
      rejectSkills(error)
      discard(error)
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

/** ページ側で写真を開いて画素にする。Worker の createImageBitmap で開けない形式 (iPad の HEIC など) でも、
 * ブラウザが <img> で表示できるなら開ける */
async function decodeWithImg(file: Blob): Promise<ImageData> {
  const url = URL.createObjectURL(file)
  try {
    const img = new Image()
    img.src = url
    await img.decode()
    const canvas = document.createElement('canvas')
    canvas.width = img.naturalWidth
    canvas.height = img.naturalHeight
    const context = canvas.getContext('2d')
    if (!context) throw new Error('canvas の 2d が使えない')
    context.drawImage(img, 0, 0)
    return context.getImageData(0, 0, canvas.width, canvas.height)
  } finally {
    URL.revokeObjectURL(url)
  }
}

const describe = (e: unknown) => (e instanceof Error ? `${e.name}: ${e.message}` : String(e))

/** 写真を読む。位置合わせ後の画面は PNG の Blob (撮り直しなら null)。
 * onDebug には、成功しても失敗しても、写真の開き方と各段階の時間が届く (一時的なデバッグ用) */
export async function readPhoto(
  file: File,
  onDebug?: (debug: PhotoDebug) => void,
): Promise<{ result: ReadResult; aligned: Blob | null }> {
  const t0 = performance.now()
  const debug: PhotoDebug = { name: file.name, type: file.type, size: file.size, attempts: [], totalMs: 0 }
  const done = () => onDebug?.({ ...debug, totalMs: performance.now() - t0 })
  try {
    let r: WorkerResponse
    try {
      r = await call({ type: 'read', input: file })
      debug.attempts.push({ method: 'worker createImageBitmap', ok: true })
    } catch (e) {
      if (!(e instanceof DecodeFailure)) throw e
      debug.attempts.push({ method: 'worker createImageBitmap', ok: false, ms: e.ms, detail: e.detail })
      // Worker で開けなければ、ページ側の <img> で開き直す
      const t = performance.now()
      let image: ImageData
      try {
        image = await decodeWithImg(file)
      } catch (e2) {
        debug.attempts.push({ method: 'page <img>', ok: false, ms: performance.now() - t, detail: describe(e2) })
        throw new Error(e.message)
      }
      debug.attempts.push({ method: 'page <img>', ok: true, ms: performance.now() - t })
      r = await call({ type: 'read', input: image })
    }
    if (r.type !== 'read') throw new Error('読み取りの応答が不正')
    debug.worker = r.debug
    debug.timings = r.result.timings
    debug.result = r.result.kind === 'read' ? 'read' : `rejected (${r.result.reason})`
    return { result: r.result, aligned: r.aligned }
  } catch (e) {
    debug.error = describe(e)
    throw e
  } finally {
    done()
  }
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
