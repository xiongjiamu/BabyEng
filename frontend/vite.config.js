import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// PRD 9.2：PWA（H5 + Service Worker），Vue 3 + Vite
export default defineConfig({
  plugins: [vue()],
  base: '/',
  server: {
    host: true,
    port: 5173,
    // 本地开发：/api 代理到 Rust 后端
    proxy: {
      '/api': {
        target: process.env.VITE_API_PROXY || 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: false,
  },
})
