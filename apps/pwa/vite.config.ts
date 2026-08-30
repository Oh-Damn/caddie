import { readFileSync } from 'node:fs';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { VitePWA } from 'vite-plugin-pwa';

const version = JSON.parse(readFileSync('./package.json', 'utf8')).version;

export default defineConfig({
  define: { __APP_VERSION__: JSON.stringify(version) },
  plugins: [
    react(),
    tailwindcss(),
    VitePWA({
      registerType: 'autoUpdate',
      // src/lib/swUpdate.ts owns registration, so the injected one-liner would
      // only be a second, weaker registration of the same worker.
      injectRegister: null,
      manifestFilename: 'manifest.json',
      includeAssets: [
        'favicon.ico',
        'icon.svg',
        'icon-32.png',
        'icon-192.png',
        'icon-512.png',
        'icon-maskable-192.png',
        'icon-maskable-512.png',
        'apple-touch-icon.png',
        'oh-damn-logo.svg',
        'click.mp3',
      ],
      workbox: {
        navigateFallbackDenylist: [/^\/ws/, /^\/api\//],
      },
      manifest: {
        id: '/',
        name: 'Caddie',
        short_name: 'Caddie',
        description: 'Context-aware controls for your desktop',
        theme_color: '#121416',
        background_color: '#121416',
        display: 'standalone',
        orientation: 'any',
        start_url: '/',
        scope: '/',
        icons: [
          {
            src: 'icon-192.png',
            sizes: '192x192',
            type: 'image/png',
            purpose: 'any',
          },
          {
            src: 'icon-512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'any',
          },
          {
            src: 'icon-maskable-192.png',
            sizes: '192x192',
            type: 'image/png',
            purpose: 'maskable',
          },
          {
            src: 'icon-maskable-512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'maskable',
          },
        ],
      },
    }),
  ],
  server: {
    host: true,
    port: 5173,
  },
});
