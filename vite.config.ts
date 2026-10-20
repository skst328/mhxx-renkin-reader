import { readFileSync } from 'node:fs'
import { fileURLToPath, URL } from 'node:url'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig } from 'vite'

// バージョンは package.json の version だけで管理し、ビルド時にアプリへ埋め込む (mhxx-combo-scan と同じ)
const { version } = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8')) as { version: string }

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  // GitHub Pages はリポジトリ名のサブパスで配信される。開発サーバーは / のままにしたいので分ける (mhxx-combo-scan と同じ)
  base: mode === 'production' ? '/mhxx-renkin-reader/' : '/',
  plugins: [react(), tailwindcss()],
  define: {
    __APP_VERSION__: JSON.stringify(version),
  },
  // 読み取りの Worker は wasm (renkin-reader/pkg) を ES モジュールとして読み込む
  worker: { format: 'es' },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
}))
