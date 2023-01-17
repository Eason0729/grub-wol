import { defineConfig } from 'vite'
export default defineConfig({
    server: {
      proxy: {
        '/api': 'http://localhost:8080/api',
        '/proxy/5173': {
          target: 'http://localhost:5173',
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/proxy\/5173/, ''),
        },
      },
    },
  })
  