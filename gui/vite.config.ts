import { defineConfig } from 'vite'
import { readFileSync } from 'fs'
import { resolve } from 'path'

function getAppVersion(): string {
  try {
    const pkg = JSON.parse(readFileSync(resolve(__dirname, 'package.json'), 'utf-8'))
    return pkg.version
  } catch {
    return '0.0.0'
  }
}

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(getAppVersion()),
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    minify: 'esbuild',
    sourcemap: false,
    target: 'es2022',
    rollupOptions: {
      output: {
        manualChunks: {
          'tauri-api': ['@tauri-apps/api'],
        },
      },
    },
  },
  optimizeDeps: {
    include: ['@tauri-apps/api', '@tauri-apps/plugin-shell', '@tauri-apps/plugin-fs'],
  },
  resolve: {
    extensions: ['.ts', '.mts', '.mjs', '.js', '.json'],
  },
})
