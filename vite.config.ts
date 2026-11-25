import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const DEV_SERVER_PORT = 1420;

// Tauri expects a fixed dev-server port and must keep control of the terminal output.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: DEV_SERVER_PORT,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    // WebView2 is evergreen Chromium on Windows.
    target: "chrome120",
    sourcemap: false,
  },
});
