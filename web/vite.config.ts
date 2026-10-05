/// <reference types="vitest/config" />
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

export default defineConfig({
  base: './',
  plugins: [vue()],
  server: {
    proxy: {
      '/api': { target: 'http://127.0.0.1:7480', changeOrigin: true },
    },
  },
  test: {
    environment: 'jsdom',
  },
})
