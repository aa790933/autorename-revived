import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { readFileSync } from 'fs'

function getAppVersion(): string {
  try {
    const pkg = JSON.parse(readFileSync(path.resolve(__dirname, 'package.json'), 'utf-8'))
    return pkg.version
  } catch {
    return '0.0.0'
  }
}

export default defineConfig({
  plugins: [react()],
  define: {
    __APP_VERSION__: JSON.stringify(getAppVersion()),
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    minify: 'esbuild',
    sourcemap: false,
    target: 'es2022',
    rollupOptions: {
      output: {
        manualChunks: {
          'tauri-api': ['@tauri-apps/api'],
          'react-vendor': ['react', 'react-dom', 'zustand'],
          'i18n': ['i18next', 'react-i18next'],
        },
      },
    },
  },
  optimizeDeps: {
    include: [
      '@tauri-apps/api',
      '@tauri-apps/plugin-dialog',
      '@tauri-apps/plugin-fs',
      '@tauri-apps/plugin-opener',
      'react',
      'react-dom',
    ],
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})