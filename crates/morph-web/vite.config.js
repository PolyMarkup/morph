import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

export default defineConfig({
  resolve: {
    alias: {
      // rst-compiler only reads Shiki's language metadata when highlighting is
      // disabled. Keep the preview lazy and avoid shipping every Shiki grammar.
      shiki: fileURLToPath(new URL("./web/shiki-preview-stub.js", import.meta.url)),
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8787",
    },
  },
  test: {
    environment: "node",
  },
});
