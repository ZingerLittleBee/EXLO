import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import viteReact from '@vitejs/plugin-react'
import dotenv from 'dotenv'
import { nitro } from 'nitro/vite'
import { defineConfig } from 'vite'
import tsconfigPaths from 'vite-tsconfig-paths'

const rootDir = path.resolve(__dirname, '../..')

// for nitro
dotenv.config({ path: path.join(rootDir, '.env') })

export default defineConfig({
  envDir: rootDir,
  // https://github.com/nitrojs/nitro/issues/3741
  plugins: [tsconfigPaths(), tailwindcss(), tanstackStart(), nitro({ preset: 'bun' }), viteReact()],
  server: {
    port: 3000
  }
})
