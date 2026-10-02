import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// The same two lines as the other two apps' configs: `vitePreprocess` is what
// lets a `<script lang="ts">` block be TypeScript.
export default {
  preprocess: vitePreprocess(),
};
