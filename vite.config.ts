import { fileURLToPath, URL } from 'node:url'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  // GitHub Pages はリポジトリ名のサブパスで配信される。開発サーバーは / のままにしたいので分ける (mhxx-combo-scan と同じ)
  base: mode === 'production' ? '/mhxx-renkin-reader/' : '/',
  plugins: [react(), tailwindcss()],
  // 読み取りの Worker は wasm (renkin-reader/pkg) を ES モジュールとして読み込む
  worker: { format: 'es' },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
}))
