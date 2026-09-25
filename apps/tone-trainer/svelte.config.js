import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// The same two lines as the full app's config: `vitePreprocess` is what lets a
// `<script lang="ts">` block be TypeScript, and without it vite falls back to the
// default configuration and warns on every build.
export default {
  preprocess: vitePreprocess(),
};
