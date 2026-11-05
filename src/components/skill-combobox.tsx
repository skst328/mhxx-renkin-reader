import { useRef } from 'react'

import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
} from '@/components/ui/combobox'
import type { SkillInfo } from '@/lib/types'
import { cn } from '@/lib/utils'

/** 一覧の先頭に置く「なし」(第 2 スキルなし)。選ぶと値は null になる */
const NONE: SkillInfo = { id: -1, name: 'なし', skill1: undefined, skill2: undefined }

/** 名前の比べ方: 前後の空白を除き、全角と半角の違いは無視する (「ＫＯ」と「KO」を同じに) */
const normalize = (s: string) => s.normalize('NFKC').trim()

/** スキルを名前で絞り込んで選ぶ。allowNone なら一覧の先頭に「なし」を置く
 * (なしのあいだは入力欄を空にして「なし」を薄く出す。押してすぐ打ち始められるように)。
 * 絞り込んだ一覧の先頭は選択中にしておき、Enter で決められる。一覧から選ばずに欄を離れても、打った文字が
 * スキル名とぴったり同じなら、そのスキルを選んだことにする。attention なら要確認の色にする */
export function SkillCombobox({
  skills,
  value,
  onChange,
  allowNone = false,
  placeholder,
  attention = false,
  id,
}: {
  /** 選べるスキル */
  skills: SkillInfo[]
  value: number | null
  onChange: (id: number | null) => void
  allowNone?: boolean
  placeholder?: string
  attention?: boolean
  id?: string
}) {
  const items = allowNone ? [NONE, ...skills] : skills
  const selected = skills.find((s) => s.id === value) ?? null
  /** ユーザーが入力欄に打った文字 (選ばずに離れたときに、名前とぴったり同じかを見る) */
  const typed = useRef('')
  const pickTyped = () => {
    const text = normalize(typed.current)
    typed.current = ''
    if (text === '') return
    const match = items.find((s) => normalize(s.name) === text)
    if (match && match !== selected) onChange(match.id === NONE.id ? null : match.id)
  }
  return (
    <Combobox
      items={items}
      value={selected}
      onValueChange={(s: SkillInfo | null) => {
        typed.current = ''
        onChange(s === null || s.id === NONE.id ? null : s.id)
      }}
      onInputValueChange={(text, details) => {
        // 覚えるのは打った文字だけ。選ばずに閉じたことで入力が空にされるとき (ほかの欄を押したときなど。
        // 欄を離れるより先にこちらが来る) は、空にされる前に打った文字で選ぶ
        if (details.reason === 'input-change') typed.current = text
        else if (['input-clear', 'outside-press', 'focus-out'].includes(details.reason)) pickTyped()
      }}
      itemToStringLabel={(s: SkillInfo) => s.name}
      filter={(s: SkillInfo, query: string) => normalize(s.name).includes(normalize(query))}
      isItemEqualToValue={(a: SkillInfo, b: SkillInfo) => a.id === b.id}
      autoHighlight
    >
      <ComboboxInput
        id={id}
        className={cn('w-full', attention && 'border-warning ring-3 ring-warning/20')}
        placeholder={selected === null && allowNone ? NONE.name : placeholder}
        onBlur={pickTyped}
      />
      <ComboboxContent>
        <ComboboxEmpty>見つかりません</ComboboxEmpty>
        <ComboboxList>
          {(s: SkillInfo) => (
            <ComboboxItem key={s.id} value={s}>
              {s.name}
            </ComboboxItem>
          )}
        </ComboboxList>
      </ComboboxContent>
    </Combobox>
  )
}
