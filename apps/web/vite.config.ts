import tailwindcss from '@tailwindcss/vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import viteReact from '@vitejs/plugin-react'
import { nitro } from 'nitro/vite'
import { defineConfig } from 'vite'
import tsconfigPaths from 'vite-tsconfig-paths'

export default defineConfig({
  // https://github.com/nitrojs/nitro/issues/3741
  plugins: [tsconfigPaths(), tailwindcss(), tanstackStart(), nitro({ preset: 'bun' }), viteReact()],
  server: {
    port: 3000
  }
})
