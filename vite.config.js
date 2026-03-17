import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: process.env.TAURI_PLATFORM == "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('node_modules')) {
            if (id.includes('react-dom')) return 'vendor-react';
            if (id.includes('react')) return 'vendor-react';
            if (id.includes('zustand')) return 'vendor-state';
            if (id.includes('@tauri-apps')) return 'vendor-tauri';
            if (id.includes('leaflet') || id.includes('react-leaflet')) return 'vendor-map';
            if (id.includes('video.js') || id.includes('videojs')) return 'vendor-video';
            if (id.includes('hls.js')) return 'vendor-hls';
            if (id.includes('mpegts')) return 'vendor-mpegts';
            return 'vendor';
          }
        }
      }
    }
  },
});
