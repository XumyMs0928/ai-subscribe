import { spawn } from "node:child_process";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "..",
);
const windowsApp = path.join(projectRoot, "apps", "windows");
const node = path.join(projectRoot, ".toolchains", "node", "node.exe");
const typescriptCli = path.join(
    windowsApp,
    "node_modules",
    "typescript",
    "bin",
    "tsc",
);
const viteCli = path.join(windowsApp, "node_modules", "vite", "bin", "vite.js");
const playwrightCli = path.join(
    projectRoot,
    "node_modules",
    "@playwright",
    "test",
    "cli.js",
);
const browserRoot = path.join(
    projectRoot,
    ".toolchains",
    "playwright-browsers",
);
const previewHost = "127.0.0.1";
const previewPort = 4173;
const previewUrl = `http://${previewHost}:${previewPort}`;
const startupTimeoutMs = 15_000;
const buildTimeoutMs = 10 * 60_000;
const playwrightTimeoutMs = 40 * 60_000;
const cleanupTimeoutMs = 5_000;
const playwrightArguments = process.argv.slice(2);
if (playwrightArguments[0] === "--") playwrightArguments.shift();

const activeChildren = new Set();
let shuttingDown = false;

function waitForExit(child) {
    return new Promise((resolve, reject) => {
        let settled = false;
        child.once("error", (error) => {
            if (!settled) {
                settled = true;
                reject(error);
            }
        });
        child.once("exit", (code, signal) => {
            if (!settled) {
                settled = true;
                resolve({ code, signal });
            }
        });
    });
}

async function terminateTree(child) {
    if (!child || child.exitCode !== null || child.signalCode !== null) return;

    if (process.platform === "win32") {
        const killer = spawn(
            "taskkill.exe",
            ["/PID", String(child.pid), "/T", "/F"],
            { stdio: "ignore", windowsHide: true },
        );
        let timer;
        const result = await Promise.race([
            waitForExit(killer).then((exit) => ({ kind: "exited", exit })),
            new Promise((resolve) => {
                timer = setTimeout(
                    () => resolve({ kind: "timed-out" }),
                    cleanupTimeoutMs,
                );
            }),
        ]);
        clearTimeout(timer);
        if (result.kind === "timed-out") {
            killer.kill();
            throw new Error(`taskkill timed out for process tree ${child.pid}`);
        }
        if (result.exit.code !== 0 && child.exitCode === null) {
            let timer;
            await Promise.race([
                waitForExit(child).catch(() => undefined),
                new Promise((resolve) => {
                    timer = setTimeout(resolve, 1_000);
                }),
            ]);
            clearTimeout(timer);
            if (child.exitCode === null && child.signalCode === null) {
                child.kill();
                let fallbackTimer;
                await Promise.race([
                    waitForExit(child).catch(() => undefined),
                    new Promise((resolve) => {
                        fallbackTimer = setTimeout(resolve, 1_000);
                    }),
                ]);
                clearTimeout(fallbackTimer);
                if (child.exitCode === null && child.signalCode === null) {
                    throw new Error(
                        `taskkill failed for process tree ${child.pid}`,
                    );
                }
            }
        }
        return;
    }

    try {
        process.kill(-child.pid, "SIGTERM");
    } catch {
        return;
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
    if (child.exitCode === null && child.signalCode === null) {
        try {
            process.kill(-child.pid, "SIGKILL");
        } catch {
            // The process tree already exited between the check and signal.
        }
    }
}

async function stopAllChildren() {
    const children = [...activeChildren];
    const results = await Promise.allSettled(
        children.map((child) => terminateTree(child)),
    );
    const failures = results.filter((result) => result.status === "rejected");
    if (failures.length > 0) {
        throw new AggregateError(
            failures.map((failure) => failure.reason),
            "One or more owned process trees could not be terminated.",
        );
    }
}

function spawnOwned(command, args, options = {}) {
    const child = spawn(command, args, {
        cwd: projectRoot,
        stdio: "inherit",
        windowsHide: true,
        detached: process.platform !== "win32",
        ...options,
    });
    activeChildren.add(child);
    child.once("exit", () => activeChildren.delete(child));
    child.once("error", () => activeChildren.delete(child));
    return child;
}

async function runBounded(command, args, options, timeoutMs, label) {
    const child = spawnOwned(command, args, options);
    let timer;
    const timeout = new Promise((resolve) => {
        timer = setTimeout(() => resolve({ timedOut: true }), timeoutMs);
    });
    const result = await Promise.race([
        waitForExit(child).then((exit) => ({ timedOut: false, ...exit })),
        timeout,
    ]);
    clearTimeout(timer);

    if (result.timedOut) {
        await terminateTree(child);
        throw new Error(
            `${label} exceeded its ${timeoutMs}ms wall-clock limit.`,
        );
    }
    if (result.code !== 0) {
        throw new Error(
            `${label} failed with ${result.signal ? `signal ${result.signal}` : `exit code ${result.code}`}.`,
        );
    }
}

function isPreviewPortInUse() {
    return new Promise((resolve) => {
        const socket = net.createConnection({
            host: previewHost,
            port: previewPort,
        });
        const finish = (inUse) => {
            socket.destroy();
            resolve(inUse);
        };
        socket.setTimeout(500);
        socket.once("connect", () => finish(true));
        socket.once("timeout", () => finish(false));
        socket.once("error", () => finish(false));
    });
}

async function waitForPreview(server) {
    const deadline = Date.now() + startupTimeoutMs;
    while (Date.now() < deadline) {
        if (server.exitCode !== null || server.signalCode !== null) {
            throw new Error(
                "The project preview server exited before it became ready.",
            );
        }
        if (await isPreviewPortInUse()) return;
        await new Promise((resolve) => setTimeout(resolve, 200));
    }
    throw new Error("The project preview server did not become ready in time.");
}

async function runPlaywright() {
    const child = spawnOwned(
        node,
        [playwrightCli, "test", ...playwrightArguments],
        {
            env: {
                ...process.env,
                BASE_URL: previewUrl,
                PLAYWRIGHT_BROWSERS_PATH: browserRoot,
            },
        },
    );
    let timer;
    const timeout = new Promise((resolve) => {
        timer = setTimeout(
            () => resolve({ timedOut: true }),
            playwrightTimeoutMs,
        );
    });
    const result = await Promise.race([
        waitForExit(child).then((exit) => ({ timedOut: false, ...exit })),
        timeout,
    ]);
    clearTimeout(timer);

    if (result.timedOut) {
        await terminateTree(child);
        throw new Error(
            `Playwright exceeded its ${playwrightTimeoutMs}ms wall-clock limit.`,
        );
    }
    return result.code ?? 1;
}

async function handleSignal(signal) {
    if (shuttingDown) return;
    shuttingDown = true;
    await stopAllChildren();
    process.exit(signal === "SIGINT" ? 130 : 143);
}

process.once("SIGINT", () => void handleSignal("SIGINT"));
process.once("SIGTERM", () => void handleSignal("SIGTERM"));

let exitCode = 1;
try {
    if (await isPreviewPortInUse()) {
        throw new Error(
            `Port ${previewPort} is already in use. Refusing to test an unmanaged server.`,
        );
    }

    await runBounded(
        node,
        [typescriptCli, "-b"],
        { cwd: windowsApp },
        buildTimeoutMs,
        "TypeScript build",
    );
    await runBounded(
        node,
        [viteCli, "build"],
        { cwd: windowsApp },
        buildTimeoutMs,
        "Vite build",
    );

    const server = spawnOwned(
        node,
        [
            viteCli,
            "preview",
            "--host",
            previewHost,
            "--port",
            String(previewPort),
            "--strictPort",
        ],
        { cwd: windowsApp },
    );
    await waitForPreview(server);
    exitCode = await runPlaywright();
} catch (error) {
    console.error(error instanceof Error ? error.message : error);
    exitCode = 1;
} finally {
    try {
        await stopAllChildren();
    } catch (error) {
        console.error(error instanceof Error ? error.message : error);
        exitCode = 1;
    }
}

process.exit(exitCode);
