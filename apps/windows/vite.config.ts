import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
    plugins: [react(), tailwindcss()],
    clearScreen: false,
    envPrefix: ["VITE_"],
    build: {
        target: "es2021",
        sourcemap: false,
    },
    test: {
        environment: "jsdom",
        setupFiles: "./src/test/setup.ts",
        restoreMocks: true,
    },
});
