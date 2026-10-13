import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            { name: 'terminal', test: /node_modules[\\/]@xterm[\\/]/ },
            { name: 'tauri', test: /node_modules[\\/]@tauri-apps[\\/]/ },
          ],
        },
      },
    },
  },
  server: {
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
})
