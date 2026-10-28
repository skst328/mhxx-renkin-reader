import { useEffect, useMemo, useRef, useState } from 'react'
import {
  AlertCircleIcon,
  CameraIcon,
  CircleCheckIcon,
  ImageIcon,
  LoaderCircleIcon,
  XIcon,
  PlusIcon,
  RotateCcwIcon,
  SearchIcon,
  ShieldCheckIcon,
  TriangleAlertIcon,
} from 'lucide-react'

import { CharmForm } from '@/components/charm-form'
import { FrameResults, type ResultItem, type SearchedCharm } from '@/components/frame-results'
import { AlignedScreen, ScreenPlaceholder } from '@/components/photo-review'
import { SearchRangeFields } from '@/components/search-range-fields'
import { TabPhoto } from '@/components/tab-photo'
import { ThemeToggle } from '@/components/theme-toggle'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { isComplete, type Draft, type FieldName, type Unsure } from '@/lib/draft'
import { detectCapabilities } from '@/lib/capabilities'
import { useWindowFileDrop } from '@/lib/drop'
import { parseFrame } from '@/lib/frame'
import { formatBytes, hasFinePointer } from '@/lib/pointer'
import { loadSkills, readPhoto, searchFrames } from '@/lib/reader'
import { possible, skillName } from '@/lib/skills'
import { EMPTY_DRAFT, emptyTab, isBlank, MAX_TABS, revokeTab, type Tab } from '@/lib/tab'
import type { ReadResult, RejectReason, SkillInfo } from '@/lib/types'
import { cn } from '@/lib/utils'

/** 撮り直しの理由ごとの案内 */
const RETAKE: Record<RejectReason, { title: string; text: string }> = {
  not_aligned: { title: '鑑定画面を読み取れませんでした', text: '鑑定ボックスまで画面全体が写るように撮り直してください。' },
  too_small: { title: '画面が小さく写っています', text: 'もう少し近づいて撮り直してください。' },
  panel_not_found: { title: '護石の欄を読み取れませんでした', text: '反射やピンボケがないように撮り直してください。' },
}

/** 読み取り結果から、画面に出す値と自信のない欄 */
function fromRead(r: Extract<ReadResult, { kind: 'read' }>): { draft: Draft; unsure: Unsure } {
  const f = r.fields
  const draft: Draft = {
    skill1: f.skill1.value,
    pts1: f.pts1.value,
    skill2: f.skill2.value,
    pts2: f.pts2.value,
    slots: f.slots.value,
    // 5 以上は鑑定ボックスに何回分もの錬金の護石が並んでいるときで、1 回の錬金の中での位置はわからない
    index: f.index.value !== null && f.index.value <= 4 ? f.index.value : null,
  }
  const unsure: Unsure = {}
  for (const name of Object.keys(draft) as FieldName[]) {
    // ボックス内の位置の候補は 1〜4 番目と不明だけ (5 以上は選べない)
    if (!f[name].confident) unsure[name] = f[name].candidates.filter((c) => name !== 'index' || c.value === null || c.value <= 4)
  }
  return { draft, unsure }
}

/** 読み取りの結果を、タブの中身にする */
function applyRead(t: Tab, r: ReadResult, screen: string | null): Tab {
  if (r.kind === 'rejected') {
    // 前の写真の値が残っていると今の写真の結果に見えるので、空にして手で入力できるようにする
    return { ...t, stage: 'rejected', reason: r.reason, aligned: screen, draft: EMPTY_DRAFT, unsure: {} }
  }
  return { ...t, stage: 'edit', reason: null, aligned: screen, ...fromRead(r) }
}

/** 結果に添える、検索した護石の中身。中身は全部入っている前提 */
function describeCharm(skills: SkillInfo[], no: number, d: Draft): SearchedCharm {
  return {
    no,
    skill1: skillName(skills, d.skill1),
    pts1: d.pts1!,
    skill2: d.skill2 === null ? null : skillName(skills, d.skill2),
    pts2: d.skill2 === null ? null : d.pts2,
    slots: d.slots!,
    index: d.index,
  }
}

/** マカ錬金で出ない値があるか (探しても必ず 0 件) */
function impossibleDraft(skills: SkillInfo[], d: Draft): boolean {
  return !possible(skills, 'skill1', d.skill1, d.pts1) || !possible(skills, 'skill2', d.skill2, d.pts2)
}

export default function App() {
  const [tabs, setTabs] = useState<Tab[]>(() => [emptyTab(1)])
  const [activeId, setActiveId] = useState(1)
  /** 次に作るタブの番号 */
  const nextId = useRef(2)
  /** 今のタブ (読み取りの続きや、何枚か選んだときに、描画を待たずに見るため) */
  const tabsRef = useRef(tabs)
  const activeRef = useRef(activeId)
  useEffect(() => {
    tabsRef.current = tabs
    activeRef.current = activeId
  })
  /** 護石のタブのバー。スマホで読み取りが終わったら、ここを画面の上端に合わせる */
  const tablistRef = useRef<HTMLDivElement>(null)
  const scrollPending = useRef(false)
  useEffect(() => {
    if (!scrollPending.current) return
    scrollPending.current = false
    // 狭い幅では読み取った画面と護石の欄が縦に並ぶので、タブを切り替えられる位置から両方が見えるようにする
    if (window.matchMedia('(max-width: 639px)').matches) {
      tablistRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }
  }, [tabs])
  const [startText, setStartText] = useState('0')
  const [endText, setEndText] = useState('10000000')
  const [searching, setSearching] = useState(false)
  const [found, setFound] = useState<{
    items: ResultItem[]
    /** 検索した護石の中身 (検索した時点のもの。あとで欄を直しても変わらない) */
    charms: SearchedCharm[]
    multi: boolean
    count: number
  } | null>(null)
  /** 選んだ写真が多すぎたときなどの案内 */
  const [notice, setNotice] = useState<string | null>(null)
  const finePointer = useMemo(() => hasFinePointer(), [])
  /** マカフシギ錬金で出るスキルとポイントの範囲 (wasm から。読み取りの準備は最初の読み取りで行う) */
  const [skills, setSkills] = useState<SkillInfo[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  /** ブラウザが必要な機能 (WebAssembly の SIMD など) を備えているか。足りなければ wasm を読み込まない */
  const capabilities = useMemo(() => detectCapabilities(), [])

  useEffect(() => {
    if (!capabilities.ready) return
    loadSkills().then(setSkills, (e: Error) => setError(e.message))
  }, [capabilities.ready])

  const active = tabs.find((t) => t.id === activeId) ?? tabs[0]
  const update = (id: number, f: (t: Tab) => Tab) => setTabs((ts) => ts.map((t) => (t.id === id ? f(t) : t)))

  /** タブごとの、最後に頼んだ読み取りの番号。読み取り中に写真を選び直したり、タブを消したりしたら、前の結果は捨てる */
  const readIds = useRef(new Map<number, number>())
  const readCount = useRef(0)

  /** 写真をタブ id に読み込む */
  const readInto = async (id: number, file: File) => {
    const n = ++readCount.current
    readIds.current.set(id, n)
    const old = tabsRef.current.find((t) => t.id === id)
    if (old) revokeTab(old)
    update(id, () => ({
      ...emptyTab(id),
      photo: { url: URL.createObjectURL(file), name: file.name, detail: formatBytes(file.size) },
      stage: 'reading',
    }))
    setFound(null)
    try {
      const { result, aligned } = await readPhoto(file)
      if (readIds.current.get(id) !== n) return
      update(id, (t) => applyRead(t, result, aligned && URL.createObjectURL(aligned)))
      if (id === activeRef.current) scrollPending.current = true
    } catch (e) {
      if (readIds.current.get(id) !== n) return
      update(id, (t) => ({ ...t, stage: 'none', error: (e as Error).message }))
    }
  }

  /** タブ id に選んだ (落とした) 写真: 1 枚目は id のタブに入れ、2 枚目からは空いているタブを順に埋め、
   * 足りない分のタブを作る (合わせて MAX_TABS 個まで) */
  const selectInto = (id: number, files: File[]) => {
    const current = tabsRef.current
    const targets = [id, ...current.filter((t) => t.id !== id && isBlank(t)).map((t) => t.id)]
    const added: Tab[] = []
    while (targets.length < files.length && current.length + added.length < MAX_TABS) {
      const t = emptyTab(nextId.current++)
      added.push(t)
      targets.push(t.id)
    }
    if (added.length > 0) setTabs((ts) => [...ts, ...added])
    files.slice(0, targets.length).forEach((f, i) => void readInto(targets[i], f))
    setActiveId(id)
    const skipped = files.length - targets.length
    setNotice(skipped > 0 ? `護石は ${MAX_TABS} 個までなので、入りきらない写真 ${skipped} 枚は読み込んでいません。` : null)
  }

  const addTab = () => {
    if (tabsRef.current.length >= MAX_TABS) return
    const t = emptyTab(nextId.current++)
    setTabs((ts) => [...ts, t])
    setActiveId(t.id)
    setFound(null)
  }

  /** タブを消す。1 つしかなければ、消す代わりに中身を空にする */
  const removeTab = (id: number) => {
    const current = tabsRef.current
    const i = current.findIndex((t) => t.id === id)
    if (i < 0) return
    revokeTab(current[i])
    readIds.current.delete(id)
    setFound(null)
    setNotice(null)
    if (current.length === 1) {
      setTabs([emptyTab(id)])
      return
    }
    setTabs(current.filter((t) => t.id !== id))
    if (id === activeId) setActiveId(current[i + 1]?.id ?? current[i - 1].id)
  }

  /** 全部を最初に戻す */
  const reset = () => {
    tabsRef.current.forEach(revokeTab)
    readIds.current.clear()
    const t = emptyTab(nextId.current++)
    setTabs([t])
    setActiveId(t.id)
    setFound(null)
    setNotice(null)
  }

  // ページへのドラッグ&ドロップは、今開いているタブの写真になる (2 枚目からはほかのタブへ)
  const dragging = useWindowFileDrop(
    (files) => selectInto(active.id, files),
    !capabilities.ready || active.stage === 'reading' || searching,
  )

  const change = (id: number, field: FieldName, value: number | null) => {
    update(id, (t) => {
      // 自分で選んだ欄は確認済みにする
      const { [field]: _, ...unsure } = t.unsure
      return { ...t, draft: { ...t.draft, [field]: value }, unsure }
    })
    setFound(null)
  }

  const start = parseFrame(startText)
  const end = parseFrame(endText)
  /** 検索に使うタブ (中身が全部入力されたもの)。並び順の番号 (護石 N) と一緒に持つ */
  const filled = tabs.map((t, i) => ({ t, no: i + 1 })).filter(({ t }) => isComplete(t.draft))
  // マカ錬金で出ない値があるタブ・読み取りが曖昧な欄があるタブ (番号)
  const impossibleNos = skills ? filled.filter(({ t }) => impossibleDraft(skills, t.draft)).map(({ no }) => no) : []
  const unsureNos = filled.filter(({ t }) => Object.keys(t.unsure).length > 0).map(({ no }) => no)
  // 同じ錬金の護石は、ボックス内の位置がそれぞれ違う。同じ位置のタブがあれば、どれかの入力が間違っている
  const byPosition = new Map<number, number[]>()
  for (const { t, no } of filled) {
    if (t.draft.index !== null) byPosition.set(t.draft.index, [...(byPosition.get(t.draft.index) ?? []), no])
  }
  const samePosition = [...byPosition]
    .filter(([, nos]) => nos.length > 1)
    .map(([index, nos]) => ({ index, nos }))
  const reading = tabs.some((t) => t.stage === 'reading')
  const canSearch =
    filled.length > 0 &&
    impossibleNos.length === 0 &&
    samePosition.length === 0 &&
    start !== null &&
    end !== null &&
    start < end &&
    !searching &&
    !reading

  // 検索が終わったら結果のカードまで移る
  const resultCard = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (!found) return
    resultCard.current?.scrollIntoView({
      behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
      block: 'start',
    })
    resultCard.current?.focus({ preventScroll: true })
  }, [found])

  /** 探索の進み具合 (0〜1) と中止 */
  const [progress, setProgress] = useState(0)
  const abort = useRef<AbortController | null>(null)

  /** 中身の入ったタブの護石を全部まとめて探す (2 個以上なら、全部が (それぞれ別の位置に) 出るフレームだけ) */
  const search = async () => {
    if (!canSearch) return
    setSearching(true)
    setProgress(0)
    const controller = new AbortController()
    abort.current = controller
    try {
      const searched = filled.map(({ t: { draft: d } }) => ({
        charm: {
          skill1: d.skill1!,
          pts1: d.pts1!,
          skill2: d.skill2,
          pts2: d.skill2 === null ? null : d.pts2,
          slots: d.slots!,
        },
        index: d.index,
      }))
      const matches = await searchFrames(searched, start!, end!, {
        onProgress: (done, total) => setProgress(done / total),
        signal: controller.signal,
      })
      const charms = filled.map(({ t, no }) => describeCharm(skills!, no, t.draft))
      if (filled.length === 1) {
        // ボックス内の位置がわかっていれば全部同じなので、札は出さない
        const known = filled[0].t.draft.index !== null
        const items = matches.map((c) => ({
          frame: c.frame,
          badges: known ? [] : c.indexes[0].map((i) => `${i} 番目`),
        }))
        setFound({ items, charms, multi: false, count: 1 })
      } else {
        // 位置を入れた護石の札は上の表と同じなので、位置が不明だった護石の札だけ出す
        const items = matches.map((c) => ({
          frame: c.frame,
          badges: c.positions.flatMap((p, k) =>
            filled[k].t.draft.index === null ? [`護石 ${filled[k].no}: ${p} 番目`] : [],
          ),
        }))
        setFound({ items, charms, multi: true, count: filled.length })
      }
    } catch (e) {
      // 中止したときは何も出さない
      if ((e as Error).name !== 'AbortError') setError((e as Error).message)
    } finally {
      setSearching(false)
      abort.current = null
    }
  }

  return (
    <main className="mx-auto flex min-h-dvh w-full max-w-5xl flex-col gap-6 px-4 py-8">
      <header className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          {/* ファビコンと同じアイコンを添えて、兄弟ツール (mhxx-combo-scan) と見分けやすくする */}
          <h1 className="flex items-center gap-3">
            <img src={`${import.meta.env.BASE_URL}favicon.svg`} alt="" className="size-9 shrink-0 sm:size-10" />
            {/* 狭い幅で入りきらないときは、バージョンだけが次の行に回る */}
            <span>
              MHXX 鑑定リーダー{' '}
              <span className="text-base font-normal whitespace-nowrap text-muted-foreground tabular-nums sm:text-lg">
                v{__APP_VERSION__}
              </span>
            </span>
          </h1>
          <p className="text-sm text-muted-foreground sm:text-base">鑑定画面の写真から錬金時のフレームを特定</p>
          <p className="text-xs text-muted-foreground sm:text-sm">現在はマカ錬金 (最高ランク投入) のみ対応</p>
        </div>
        <ThemeToggle />
      </header>

      <p className="flex items-center justify-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-4 py-2.5 text-center text-sm font-medium sm:text-base">
        <ShieldCheckIcon className="size-5 shrink-0 text-primary" />
        写真は外部に送信されません
      </p>

      {dragging && (
        // 飾りなので、ドラッグのイベントを拾わせない
        <div className="pointer-events-none fixed inset-0 z-50 bg-primary/10 backdrop-blur-[2px]">
          <div className="absolute inset-3 flex flex-col items-center justify-center gap-3 rounded-2xl border-2 border-dashed border-primary bg-background/50 text-center sm:inset-6">
            <ImageIcon className="size-10 text-primary" />
            <p className="font-medium">
              ここにドロップ (護石 {tabs.indexOf(active) + 1} の写真にする。何枚かなら残りはほかのタブへ)
            </p>
          </div>
        </div>
      )}

      {!capabilities.ready && (
        <Alert variant="destructive">
          <AlertCircleIcon />
          <AlertTitle>このブラウザでは動きません</AlertTitle>
          <AlertDescription>
            <ul className="list-inside list-disc">
              {capabilities.missing.map((m) => (
                <li key={m}>{m}</li>
              ))}
            </ul>
          </AlertDescription>
        </Alert>
      )}

      {notice && (
        <Alert variant="warning">
          <AlertCircleIcon />
          <AlertDescription>{notice}</AlertDescription>
        </Alert>
      )}

      {error && (
        <Alert variant="destructive">
          <AlertCircleIcon />
          <AlertTitle>読み込めませんでした</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* 全部のタブを最初に戻す。何か入っているときだけ押せる */}
      <div className="-mb-3 flex justify-end">
        <Button
          variant="outline"
          size="sm"
          onClick={reset}
          disabled={!capabilities.ready || searching || (tabs.length === 1 && isBlank(tabs[0]) && found === null)}
        >
          <RotateCcwIcon />
          リセット
        </Button>
      </div>

      {/* 写真がなくても、手で入力して探せるように最初から出しておく */}
      {skills && (
        // 狭い幅では、読み取った画面を少しでも大きく見せるため、カードの内側の余白を詰める
        <Card className="[--card-spacing:--spacing(3)] sm:[--card-spacing:--spacing(4)]">
          <CardContent className="space-y-4">
            {/* 護石のタブ。護石は最大 4 個なので、横にはみ出させずに幅を分け合う
                (開いているタブは、下の線に 1px 重ねて線を隠す) */}
            <div ref={tablistRef} role="tablist" className="flex scroll-mt-2 items-end gap-1 border-b">
              {tabs.map((t, i) => {
                const selected = t.id === active.id
                const closable = tabs.length > 1
                // 中身の要約は出さず、読み取りの状態だけを印で示す (読み取り中・撮り直し・確かめる欄あり・読み取り済み。
                // 写真なしで手入力したタブには付けない)。読み取れたタブは、自信のない欄が残っていれば警告、なければ成功
                // (欄を確かめて直すと成功になる)
                const status =
                  t.stage === 'reading' ? (
                    <LoaderCircleIcon role="img" aria-label="読み取り中" className="size-3.5 shrink-0 animate-spin text-primary" />
                  ) : t.stage === 'rejected' || t.error ? (
                    <AlertCircleIcon role="img" aria-label="撮り直し" className="size-3.5 shrink-0 text-destructive" />
                  ) : t.stage === 'edit' ? (
                    Object.keys(t.unsure).length > 0 ? (
                      <TriangleAlertIcon role="img" aria-label="確かめる欄あり" className="size-3.5 shrink-0 text-warning" />
                    ) : (
                      <CircleCheckIcon role="img" aria-label="読み取り済み" className="size-3.5 shrink-0 text-success" />
                    )
                  ) : null
                return (
                  // 見出しは番号と印だけ (狭い幅でタブが 4 つ並ぶと 1 つ 70px ほどしかないので「護石」は付けない。
                  // 読み上げ用の名前には付ける)。消す × はタブの右端に重ねる (ボタンの中にボタンは置けないので、外側で包む)。
                  // VS Code のように、開いているタブでは常に、ほかのタブではマウスを乗せたときだけ出す。タブが 1 つなら出さない
                  <div key={t.id} className="group relative -mb-px flex min-w-0 flex-1 sm:max-w-28">
                    <button
                      type="button"
                      role="tab"
                      aria-selected={selected}
                      onClick={() => setActiveId(t.id)}
                      className={cn(
                        'flex min-w-0 flex-1 items-center gap-1.5 rounded-t-lg border border-transparent py-2 pl-3 text-left text-sm font-medium tabular-nums',
                        closable ? 'pr-7 sm:pr-9' : 'pr-3',
                        selected ? 'border-border border-b-card bg-card' : 'text-muted-foreground hover:bg-muted/50',
                      )}
                    >
                      <span>
                        <span className="sr-only">護石 </span>
                        {i + 1}
                      </span>
                      {status}
                    </button>
                    {closable && (
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        onClick={() => removeTab(t.id)}
                        disabled={t.stage === 'reading' || searching}
                        aria-label={`護石 ${i + 1} のタブを消す`}
                        className={cn(
                          // タブの縦の真ん中に置く。PC では押しやすい大きさにする
                          'absolute top-1/2 right-1 -translate-y-1/2 text-muted-foreground sm:size-7',
                          !selected && 'invisible group-hover:visible focus-visible:visible',
                        )}
                      >
                        <XIcon className="size-3 sm:size-4" />
                      </Button>
                    )}
                  </div>
                )
              })}
              {tabs.length < MAX_TABS && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="mb-1.5 ml-1 text-primary"
                  onClick={addTab}
                  disabled={searching}
                  aria-label="護石のタブを追加"
                >
                  <PlusIcon />
                </Button>
              )}
            </div>

            {/* 開いているタブの中身: そのタブの写真、撮り直しの案内、読み取った画面と護石の欄 */}
            <div role="tabpanel" className="space-y-3 sm:space-y-4">
              <TabPhoto
                key={active.id}
                photo={active.photo}
                reading={active.stage === 'reading'}
                disabled={!capabilities.ready || searching}
                showDropHint={finePointer}
                onSelect={(files) => selectInto(active.id, files)}
              />
              {active.error && (
                <Alert variant="destructive">
                  <AlertCircleIcon />
                  <AlertTitle>読み込めませんでした</AlertTitle>
                  <AlertDescription>{active.error}</AlertDescription>
                </Alert>
              )}
              {active.stage === 'rejected' && active.reason && (
                <Alert variant="destructive">
                  <CameraIcon />
                  <AlertTitle>{RETAKE[active.reason].title}</AlertTitle>
                  <AlertDescription>{RETAKE[active.reason].text}</AlertDescription>
                </Alert>
              )}
              {/* 読み取った画面と護石の欄を並べて、見比べながら直せるようにする (狭い幅では縦に並ぶ) */}
              {/* 画像の列は少し狭めて、高さを護石の欄に近づける (右の下に余白が出ないように) */}
              <div className="grid gap-6 md:grid-cols-[5fr_6fr]">
                {/* 写真を読むまでは同じ大きさの空の枠 (スマホでは場所を取るので、読み取り中のときだけ出す) */}
                {active.aligned ? (
                  <AlignedScreen src={active.aligned} />
                ) : (
                  <ScreenPlaceholder
                    failed={active.stage === 'rejected'}
                    reading={active.stage === 'reading'}
                    className={active.stage === 'reading' ? undefined : 'hidden md:block'}
                  />
                )}
                {/* 横並びのときは、左の枠の内側の余白 (12px + 枠線) だけ下げて見出しの高さを揃え、左に薄い区切り線を引く */}
                <section className="space-y-3 md:border-l md:pt-[13px] md:pl-6">
                  <h3>護石 {tabs.indexOf(active) + 1}</h3>
                  <CharmForm
                    skills={skills}
                    draft={active.draft}
                    unsure={active.unsure}
                    onChange={(field, value) => change(active.id, field, value)}
                  />
                </section>
              </div>
            </div>

            <section className="space-y-2 border-t pt-4">
              <h3>探索範囲</h3>
              <SearchRangeFields
                start={startText}
                end={endText}
                onStartChange={setStartText}
                onEndChange={setEndText}
                disabled={searching}
              />
            </section>
            <div className="border-t pt-4">
              {searching ? (
                // 検索中は進み具合と中止
                <div className="space-y-2">
                  <div className="flex items-center justify-between gap-3">
                    <p className="flex items-center gap-2 text-sm text-muted-foreground">
                      <LoaderCircleIcon className="size-4 animate-spin" />
                      フレームを検索しています… {Math.floor(progress * 100)}%
                    </p>
                    <Button variant="outline" size="sm" onClick={() => abort.current?.abort()}>
                      中止
                    </Button>
                  </div>
                  <div className="h-1.5 overflow-hidden rounded-full bg-muted">
                    <div
                      className="h-full rounded-full bg-primary transition-[width]"
                      style={{ width: `${Math.min(100, progress * 100)}%` }}
                    />
                  </div>
                </div>
              ) : (
                <div className="flex flex-wrap items-center gap-3">
                  <Button onClick={search} disabled={!canSearch}>
                    <SearchIcon />
                    フレーム検索{filled.length > 1 && ` (護石 ${filled.length} 個)`}
                  </Button>
                  {impossibleNos.length > 0 ? (
                    <span className="text-sm text-destructive">
                      マカ錬金では出ない値あり (護石 {impossibleNos.join('・')})
                    </span>
                  ) : samePosition.length > 0 ? (
                    <span className="text-sm text-destructive">
                      ボックス内の位置が重複 (
                      {samePosition.map(({ index, nos }) => `護石 ${nos.join('・')} が ${index} 番目`).join('、')})
                    </span>
                  ) : (
                    unsureNos.length > 0 && (
                      <span className="text-sm text-warning">読み取りが曖昧な欄あり (護石 {unsureNos.join('・')})</span>
                    )
                  )}
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      )}

      <div ref={resultCard} tabIndex={-1} className="scroll-mt-4 outline-none">
        {found && (
          <FrameResults
            key={found.items.length}
            found={found.items}
            charms={found.charms}
            badgesBelow={found.multi}
            note={found.multi ? `護石 ${found.count} 個で絞り込み` : undefined}
          />
        )}
      </div>
    </main>
  )
}
