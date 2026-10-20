// 一時的なデバッグ (公開前の確認用)。?debug を付けたときだけ App の一番下に出す
import { useState } from 'react'
import { BugIcon, CheckIcon, CopyIcon } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { environment, formatPhoto, formatSearch, type PhotoDebug, type SearchDebug } from '@/lib/debug'

/** photos は新しい順 */
export function DebugPanel({ photos, searches }: { photos: PhotoDebug[]; searches: SearchDebug[] }) {
  const [copied, setCopied] = useState(false)
  const sections: [string, string[]][] = [
    ['環境', environment()],
    ...photos.map((p, i): [string, string[]] => [`読み取り ${photos.length - i}`, formatPhoto(p)]),
    ...searches.map((s, i): [string, string[]] => [`検索 ${searches.length - i}`, formatSearch(s)]),
  ]
  const text = sections.map(([title, lines]) => `## ${title}\n${lines.join('\n')}`).join('\n\n')

  const copy = async () => {
    await navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  return (
    <Card className="border-dashed">
      <CardHeader className="flex flex-row items-center justify-between gap-3">
        <CardTitle className="flex items-center gap-2">
          <BugIcon className="size-4" />
          デバッグ
        </CardTitle>
        <Button variant="outline" size="sm" onClick={() => void copy()}>
          {copied ? <CheckIcon /> : <CopyIcon />}
          {copied ? 'コピー済み' : 'コピー'}
        </Button>
      </CardHeader>
      <CardContent className="space-y-4">
        {sections.map(([title, lines]) => (
          <section key={title} className="space-y-1">
            <h3 className="text-sm font-medium">{title}</h3>
            <pre className="overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs leading-relaxed">
              {lines.join('\n')}
            </pre>
          </section>
        ))}
      </CardContent>
    </Card>
  )
}
