import { test as base, expect, type Page } from "@playwright/test";

import {
    createDemoCatalog,
    createDemoPage,
    createHealthStatus,
} from "../factories/demo-dto.factory";
import {
    commandError,
    mergeCommandBehaviors,
    response,
    type ExternalCall,
    type TauriCommand,
    type TauriCommandBehavior,
    type TauriCommandOverrides,
    type TauriInvokeCall,
} from "../helpers/tauri-command-mock";

interface DemoAppFixture {
    invokeCalls(): Promise<TauriInvokeCall[]>;
    externalCalls(): Promise<ExternalCall[]>;
    setResponse(command: TauriCommand, value: unknown): Promise<void>;
    setError(command: TauriCommand, error: unknown): Promise<void>;
}

interface DemoAppFixtures {
    demoApp: DemoAppFixture;
    tauriCommandOverrides: TauriCommandOverrides;
}

const defaultCatalog = createDemoCatalog();

const defaultBehaviors: Record<TauriCommand, TauriCommandBehavior> = {
    health_v1: response(createHealthStatus()),
    demo_bootstrap_v1: response(defaultCatalog),
    demo_search_v1: response(defaultCatalog),
    demo_list_v1: response(createDemoPage()),
    demo_filter_v1: response(createDemoPage()),
    demo_detail_v1: response(defaultCatalog.items[0]),
};

async function readInvokeCalls(page: Page): Promise<TauriInvokeCall[]> {
    return page.evaluate(() =>
        structuredClone(
            (window as Window & { __TEST_TAURI_CALLS__?: TauriInvokeCall[] })
                .__TEST_TAURI_CALLS__ ?? [],
        ),
    );
}

async function readExternalCalls(page: Page): Promise<ExternalCall[]> {
    return page.evaluate(() =>
        structuredClone(
            (window as Window & { __TEST_EXTERNAL_CALLS__?: ExternalCall[] })
                .__TEST_EXTERNAL_CALLS__ ?? [],
        ),
    );
}

export const test = base.extend<DemoAppFixtures>({
    tauriCommandOverrides: [{}, { option: true }],

    demoApp: async ({ page, tauriCommandOverrides }, use, testInfo) => {
        const behaviors = mergeCommandBehaviors(
            defaultBehaviors,
            tauriCommandOverrides,
        );

        await page.addInitScript((initialBehaviors) => {
            type Behavior =
                | { kind: "response"; value: unknown }
                | { kind: "error"; error: unknown };
            type InvokeCall = {
                command: string;
                args: Record<string, unknown> | null;
            };
            type OutboundCall = {
                kind:
                    | "fetch"
                    | "xhr"
                    | "websocket"
                    | "sendBeacon"
                    | "notification";
                target: string;
                method: string | null;
            };
            type TestWindow = Window &
                typeof globalThis & {
                    __TAURI_INTERNALS__?: Record<string, unknown>;
                    __TEST_TAURI_BEHAVIORS__: Record<string, Behavior>;
                    __TEST_TAURI_CALLS__: InvokeCall[];
                    __TEST_EXTERNAL_CALLS__: OutboundCall[];
                };

            const testWindow = window as TestWindow;
            testWindow.__TEST_TAURI_BEHAVIORS__ = initialBehaviors;
            testWindow.__TEST_TAURI_CALLS__ = [];
            testWindow.__TEST_EXTERNAL_CALLS__ = [];

            const recordExternal = (
                kind: OutboundCall["kind"],
                target: string,
                method: string | null,
            ) => {
                testWindow.__TEST_EXTERNAL_CALLS__.push({
                    kind,
                    target,
                    method,
                });
            };

            testWindow.__TAURI_INTERNALS__ = {
                ...(testWindow.__TAURI_INTERNALS__ ?? {}),
                invoke: async (
                    command: string,
                    args?: Record<string, unknown>,
                ) => {
                    testWindow.__TEST_TAURI_CALLS__.push({
                        command,
                        args: args ?? null,
                    });
                    const behavior =
                        testWindow.__TEST_TAURI_BEHAVIORS__[command];
                    if (!behavior) {
                        throw new Error(
                            `Unexpected Tauri command in browser test: ${command}`,
                        );
                    }
                    if (behavior.kind === "error") throw behavior.error;
                    return structuredClone(behavior.value);
                },
            };

            testWindow.fetch = ((
                input: RequestInfo | URL,
                init?: RequestInit,
            ) => {
                const target =
                    input instanceof Request ? input.url : String(input);
                const method =
                    init?.method ??
                    (input instanceof Request ? input.method : "GET");
                recordExternal("fetch", target, method);
                return Promise.reject(
                    new TypeError(
                        "External network is disabled in demo tests.",
                    ),
                );
            }) as typeof fetch;

            const xhrMetadata = new WeakMap<
                XMLHttpRequest,
                { method: string; target: string }
            >();
            XMLHttpRequest.prototype.open = function (
                method: string,
                url: string | URL,
            ) {
                xhrMetadata.set(this, { method, target: String(url) });
            };
            XMLHttpRequest.prototype.send = function () {
                const metadata = xhrMetadata.get(this) ?? {
                    method: "GET",
                    target: "unknown",
                };
                recordExternal("xhr", metadata.target, metadata.method);
                throw new TypeError(
                    "External network is disabled in demo tests.",
                );
            };

            const NativeWebSocket = testWindow.WebSocket;
            const BlockedWebSocket = function (
                this: WebSocket,
                url: string | URL,
            ) {
                recordExternal("websocket", String(url), null);
                throw new TypeError(
                    "External network is disabled in demo tests.",
                );
            } as unknown as typeof WebSocket;
            Object.setPrototypeOf(
                BlockedWebSocket.prototype,
                NativeWebSocket.prototype,
            );
            testWindow.WebSocket = BlockedWebSocket;

            if (testWindow.navigator.sendBeacon) {
                Object.defineProperty(testWindow.navigator, "sendBeacon", {
                    configurable: true,
                    value: (url: string | URL): boolean => {
                        recordExternal("sendBeacon", String(url), "POST");
                        return false;
                    },
                });
            }

            if ("Notification" in testWindow) {
                const NativeNotification = testWindow.Notification;
                const BlockedNotification = function (
                    this: Notification,
                    title: string,
                ) {
                    recordExternal("notification", title, "construct");
                    throw new TypeError(
                        "Notification is disabled in demo tests.",
                    );
                } as unknown as typeof Notification;
                Object.setPrototypeOf(
                    BlockedNotification.prototype,
                    NativeNotification.prototype,
                );
                Object.defineProperty(BlockedNotification, "permission", {
                    configurable: true,
                    get: () => NativeNotification.permission,
                });
                Object.defineProperty(
                    BlockedNotification,
                    "requestPermission",
                    {
                        configurable: true,
                        value: async (): Promise<NotificationPermission> => {
                            recordExternal(
                                "notification",
                                "requestPermission",
                                "requestPermission",
                            );
                            return "denied";
                        },
                    },
                );
                testWindow.Notification = BlockedNotification;
            }
        }, behaviors);

        const demoApp: DemoAppFixture = {
            invokeCalls: () => readInvokeCalls(page),
            externalCalls: () => readExternalCalls(page),
            setResponse: (command, value) =>
                page.evaluate(
                    ({ command, value }) => {
                        const target = window as Window & {
                            __TEST_TAURI_BEHAVIORS__: Record<
                                string,
                                TauriCommandBehavior
                            >;
                        };
                        target.__TEST_TAURI_BEHAVIORS__[command] = {
                            kind: "response",
                            value,
                        };
                    },
                    { command, value },
                ),
            setError: (command, error) =>
                page.evaluate(
                    ({ command, error }) => {
                        const target = window as Window & {
                            __TEST_TAURI_BEHAVIORS__: Record<
                                string,
                                TauriCommandBehavior
                            >;
                        };
                        target.__TEST_TAURI_BEHAVIORS__[command] = {
                            kind: "error",
                            error,
                        };
                    },
                    { command, error },
                ),
        };

        await use(demoApp);

        if (!page.isClosed()) {
            const [invokeCalls, externalCalls] = await Promise.all([
                readInvokeCalls(page),
                readExternalCalls(page),
            ]);
            await testInfo.attach("tauri-invoke-calls", {
                body: JSON.stringify(invokeCalls, null, 2),
                contentType: "application/json",
            });
            await testInfo.attach("external-calls", {
                body: JSON.stringify(externalCalls, null, 2),
                contentType: "application/json",
            });
        }
    },
});

export { commandError, expect, response };
export type {
    DemoAppFixture,
    ExternalCall,
    TauriCommand,
    TauriCommandOverrides,
    TauriInvokeCall,
};
