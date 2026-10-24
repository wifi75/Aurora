import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

// In sviluppo l'interfaccia fa da proxy verso il core (porta 8787), WebSocket compreso.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      // xfwd: il core vede che la richiesta e' passata da un proxy e rifiuta di dare codici di associazione
      '/api': { target: 'http://127.0.0.1:8787', ws: true, changeOrigin: false, xfwd: true },
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    css: false,
  },
});
