import { useState } from 'react'
import { AlertTriangleIcon } from 'lucide-react'

import { Alert, AlertTitle, AlertDescription } from '@/components/ui/alert'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { frameToTime } from '@/lib/frame'
import { cn } from '@/lib/utils'

/** 一度に出す数。広い範囲で探して候補が多くても畳まずに並べ、続きは「さらに表示」で出す */
const PAGE = 100

/** 候補 1 つ: フレームと、そのフレームに付ける札 (何番目に出るかなど。無ければ空) */
export interface ResultItem {
  frame: number
  badges: string[]
}

/** 検索した護石 1 個の中身 (名前に直したもの)。skill2 が null なら第 2 スキルなし、index が null なら位置不明 */
export interface SearchedCharm {
  no: number
  skill1: string
  pts1: number
  skill2: string | null
  pts2: number | null
  slots: number
  index: number | null
}

const showPoints = (p: number) => (p > 0 ? `+${p}` : String(p))

/** 検索した護石の一覧。行ごとに文字の位置がそろうように表にする (スキル名とポイントは別の列で、ポイントは右寄せ) */
function CharmTable({ charms }: { charms: SearchedCharm[] }) {
  return (
    <table className="text-sm text-muted-foreground tabular-nums [&_td]:py-0.5 [&_td]:pr-2 sm:[&_td]:pr-3 [&_th]:pr-2 [&_th]:text-left [&_th]:font-normal sm:[&_th]:pr-3">
      <thead className="sr-only">
        <tr>
          <th>護石</th>
          <th>第1スキル</th>
          <th>ポイント</th>
          <th>第2スキル</th>
          <th>ポイント</th>
          <th>スロット</th>
          <th>ボックス内の位置</th>
        </tr>
      </thead>
      <tbody>
        {charms.map((c) => (
          <tr key={c.no}>
            <td className="whitespace-nowrap">護石 {c.no}</td>
            <td>{c.skill1}</td>
            <td className="text-right">{showPoints(c.pts1)}</td>
            <td>{c.skill2 ?? '―'}</td>
            <td className="text-right">{c.pts2 === null ? '' : showPoints(c.pts2)}</td>
            <td className="whitespace-nowrap">{c.slots}スロ</td>
            <td className="whitespace-nowrap">{c.index === null ? '位置不明' : `${c.index} 番目`}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

/** 錬金時のフレームの候補。フレームを大きく出し、ゲーム開始からの時間を添える。
 * 札は、護石 1 個なら右上に縦に (短い「N 番目」だけ)、複数の護石で絞り込んだときはフレームの下に並べる */
export function FrameResults({
  found,
  charms,
  badgesBelow = false,
  note,
}: {
  found: ResultItem[]
  /** 検索した護石の中身 (1 個 1 行)。見つからなかったときも、何で探したか分かるように出す */
  charms: SearchedCharm[]
  badgesBelow?: boolean
  /** 件数の横に添える文 (「護石 2 個で絞り込み」など) */
  note?: string
}) {
  const [shown, setShown] = useState(PAGE)
  const unique = found.length === 1

  const list = (
    <div className="space-y-3">
      {/* 候補が多いときは詰めて並べる (スマホ 2 列、640px 以上 3 列、1024px 以上 4 列) */}
      <ul className={cn('grid gap-2', !unique && 'grid-cols-2 sm:grid-cols-3 lg:grid-cols-4')}>
        {found.slice(0, shown).map((c) => (
          <li
            key={c.frame}
            className={cn('relative space-y-0.5 rounded-lg border p-3', unique && 'border-primary/40 bg-primary/5')}
          >
            <p
              className={cn(
                'font-mono tabular-nums select-all',
                unique ? 'text-3xl' : 'text-lg sm:text-2xl',
                // 右上の札の分だけ空ける
                !badgesBelow && c.badges.length > 0 && 'pr-14',
              )}
            >
              {c.frame}
            </p>
            <p className="text-xs text-muted-foreground sm:text-sm">{frameToTime(c.frame)}</p>
            {c.badges.length > 0 &&
              (badgesBelow ? (
                <div className="flex flex-wrap gap-1 pt-1.5">
                  {c.badges.map((b) => (
                    <Badge key={b} variant="secondary">
                      {b}
                    </Badge>
                  ))}
                </div>
              ) : (
                <div className="absolute top-2.5 right-2.5 flex flex-col items-end gap-1">
                  {c.badges.map((b) => (
                    <Badge key={b} variant="secondary">
                      {b}
                    </Badge>
                  ))}
                </div>
              ))}
          </li>
        ))}
      </ul>
      {shown < found.length && (
        <Button variant="outline" onClick={() => setShown(shown + PAGE)}>
          さらに表示 (残り {(found.length - shown).toLocaleString()} 件)
        </Button>
      )}
    </div>
  )

  return (
    <Card>
      <CardHeader>
        <CardTitle>錬金時のフレーム</CardTitle>
        {found.length > 0 && (
          <CardDescription>
            {unique ? '一意に特定' : `候補 ${found.length.toLocaleString()} 件`}
            {note && ` (${note})`}
          </CardDescription>
        )}
        {/* 狭い幅で長いスキル名が来ても、ページごと横にはみ出さないようにする */}
        <div className="overflow-x-auto">
          <CharmTable charms={charms} />
        </div>
      </CardHeader>
      <CardContent>
        {found.length === 0 ? (
          <Alert variant="warning">
            <AlertTriangleIcon />
            <AlertTitle>この探索範囲では見つかりませんでした</AlertTitle>
            <AlertDescription>
              護石の中身と探索範囲を確認してください。
            </AlertDescription>
          </Alert>
        ) : (
          list
        )}
      </CardContent>
    </Card>
  )
}
