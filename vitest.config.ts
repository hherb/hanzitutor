import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config.ts";

/**
 * The interface's own test run.
 *
 * It exists because the app had no frontend test runner at all: the stroke
 * animation's geometry is arithmetic over arrays, and the only thing standing
 * between a wrong prefix or a wrong sample and a learner seeing it was the
 * compiler. The tests are pure TypeScript — no component, no canvas, no DOM —
 * so `environment: "node"` is deliberate rather than a default left standing;
 * it keeps a jsdom dependency out of the project.
 *
 * The Vite config is merged in so that everything the app builds with applies
 * here too: a future test that imports a `.svelte` file gets the Svelte
 * compiler, and any resolve alias added there reaches these tests as well.
 */
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      include: ["src/**/*.test.ts"],
      environment: "node",
    },
  }),
);
