import { useId, useRef } from 'react'
import { ImagePlusIcon, LoaderCircleIcon, UploadIcon } from 'lucide-react'

import { buttonVariants } from '@/components/ui/button'
import { MAX_TABS } from '@/lib/tab'
import { cn } from '@/lib/utils'

/** 写真を選ぶ input。同じ写真を選び直せるように、選んだら空に戻す */
function FileInput({
  id,
  disabled,
  multiple = false,
  onSelect,
}: {
  id: string
  disabled: boolean
  multiple?: boolean
  onSelect: (files: File[]) => void
}) {
  const ref = useRef<HTMLInputElement>(null)
  return (
    <input
      id={id}
      ref={ref}
      type="file"
      multiple={multiple}
      accept="image/*,.heic,.heif"
      className="sr-only"
      disabled={disabled}
      onChange={(e) => {
        const files = [...(e.target.files ?? [])]
        if (files.length > 0) onSelect(files)
        if (ref.current) ref.current.value = ''
      }}
    />
  )
}

/**
 * タブの中の、そのタブの写真の行。写真が無ければ選ぶ欄、あれば小さな写真とファイル名と「写真を選び直す」。
 * 読み取り中は選び直せない。
 * 選ぶ欄では写真を何枚か選べる (2 枚目からは呼ぶ側がほかのタブに入れる)。選び直しは 1 枚だけ
 */
export function TabPhoto({
  photo,
  reading,
  disabled,
  showDropHint,
  onSelect,
}: {
  photo: { url: string; name: string; detail: string } | null
  reading: boolean
  disabled: boolean
  /** ドラッグ&ドロップの案内を出すか (タッチだけの端末では出さない) */
  showDropHint: boolean
  onSelect: (files: File[]) => void
}) {
  const inputId = useId()
  const locked = disabled || reading

  if (!photo) {
    return (
      <label
        htmlFor={inputId}
        className={cn(
          'flex flex-col items-center gap-2 rounded-xl border-2 border-dashed px-4 py-8 text-center text-sm',
          locked ? 'opacity-50' : 'cursor-pointer hover:bg-muted/50',
        )}
      >
        <span className="mb-1 flex size-12 items-center justify-center rounded-full bg-muted">
          <UploadIcon className="size-5 text-muted-foreground" />
        </span>
        <span className="font-medium">
          {showDropHint ? '写真をドラッグ&ドロップ、またはクリックして選択' : 'タップして写真を選択'}
        </span>
        <span className="text-muted-foreground">
          同じ錬金の護石の写真を何枚か選ぶと、タブに分かれます ({MAX_TABS} 枚まで)
        </span>
        <FileInput id={inputId} disabled={locked} multiple onSelect={onSelect} />
      </label>
    )
  }

  return (
    <div className="flex items-center gap-3">
      <img src={photo.url} alt="" decoding="async" className="size-12 shrink-0 rounded-md border bg-muted object-cover" />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{photo.name}</p>
        <p className="flex items-center gap-1.5 truncate text-sm text-muted-foreground">
          {reading && <LoaderCircleIcon className="size-3.5 animate-spin text-primary" />}
          {reading ? '読み取り中…' : photo.detail}
        </p>
      </div>
      <label
        htmlFor={inputId}
        className={cn(
          buttonVariants({ variant: 'ghost', size: 'sm' }),
          locked ? 'pointer-events-none opacity-50' : 'cursor-pointer',
        )}
      >
        <ImagePlusIcon />
        写真を選び直す
        <FileInput id={inputId} disabled={locked} onSelect={onSelect} />
      </label>
    </div>
  )
}
