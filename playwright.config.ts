import { defineConfig, devices } from "@playwright/test";

const baseURL = "http://127.0.0.1:4173";
const isCI = ["1", "true"].includes((process.env.CI ?? "").toLowerCase());

export default defineConfig({
    testDir: "./tests/e2e",
    fullyParallel: true,
    forbidOnly: isCI,
    retries: isCI ? 2 : 0,
    workers: isCI ? 1 : 4,
    timeout: 60_000,
    expect: {
        timeout: 10_000,
    },
    outputDir: "test-results/artifacts",
    reporter: [
        ["list"],
        ["html", { outputFolder: "playwright-report", open: "never" }],
        ["junit", { outputFile: "test-results/junit.xml" }],
    ],
    use: {
        baseURL,
        channel: "chromium",
        actionTimeout: 15_000,
        navigationTimeout: 30_000,
        // Records every attempt and retains traces for failed attempts, including retries.
        trace: "retain-on-failure",
        screenshot: "only-on-failure",
        video: "retain-on-failure",
    },
    projects: [
        {
            name: "chromium",
            use: { ...devices["Desktop Chrome"] },
        },
    ],
});
