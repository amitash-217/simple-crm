import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// In dev, the Rust server (cargo run) serves the API on :3000 and Vite proxies to it.
export default defineConfig({
  plugins: [react()],
  build: { outDir: "dist", emptyOutDir: true },
  server: { proxy: { "/api": "http://127.0.0.1:3000" } },
});
