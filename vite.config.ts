import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri serves the dev build from a fixed port and owns the terminal, so vite
// must not clear the screen or pick a different port when 1420 is busy.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Watching the Rust build tree makes the dev server thrash; the Tauri CLI
    // restarts it on Rust changes instead.
    watch: {
      ignored: ["**/src-tauri/**", "**/.cargo-home/**", "**/.cargo-target/**", "**/data/**"],
    },
  },
  build: {
    // The app only ever runs inside the platform webview.
    target: "safari15",
    sourcemap: true,
  },
});
