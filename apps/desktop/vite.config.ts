/// <reference types="vitest/config" />
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri 用的設定：固定 port、相對於 dist 的路徑（spec/06 §2.4 桌面殼）。
export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "esnext",
    sourcemap: true,
  },
  test: {
    environment: "node",
  },
});
