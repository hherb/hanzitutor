import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri serves the dev build from a fixed port and owns the terminal, so vite
// must not clear the screen or pick a different port when it is busy. **1421,
// not 1420**: the full app owns 1420, and the two are expected to be developed
// side by side — a shared port would have one of them silently attach to the
// other's dev server and render the wrong interface.
//
// `TAURI_DEV_HOST` is set when Tauri builds for a device on the network, and
// replaces the dev URL's host with a LAN address; left unset — the desktop case
// — the server stays on localhost, which is all the webview needs. Declared
// rather than pulled in because this project has no `@types/node`.
declare const process: { env: Record<string, string | undefined> };
const devHost = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    host: devHost || undefined,
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
