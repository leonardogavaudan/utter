import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"] },
  },
  build: {
    target: ["es2022", "chrome110", "safari16"],
    rolldownOptions: {
      input: { settings: "index.html", overlay: "overlay.html" },
    },
  },
});
