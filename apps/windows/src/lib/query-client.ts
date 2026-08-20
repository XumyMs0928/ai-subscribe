import { QueryClient } from "@tanstack/react-query";
import type { DesktopApi } from "./desktop-api/desktop-api";
import type { QueryIntelFeedInputV1 } from "./desktop-api/desktop-api";

const desktopApiKeys = new WeakMap<DesktopApi, number>();
let nextDesktopApiKey = 1;

export function desktopApiQueryKey(api: DesktopApi): number {
    const existing = desktopApiKeys.get(api);
    if (existing !== undefined) return existing;
    const key = nextDesktopApiKey;
    nextDesktopApiKey += 1;
    desktopApiKeys.set(api, key);
    return key;
}

export const demoIntelligenceKeys = {
    health: (apiKey: number) => ["health", apiKey] as const,
    bootstrap: (apiKey: number) => ["demo-bootstrap", apiKey] as const,
    catalog: (apiKey: number, query: string, track: string | null) =>
        ["demo-catalog", apiKey, query, track] as const,
    detail: (apiKey: number, id: string | null) =>
        ["demo-detail", apiKey, id] as const,
};

export const setupKeys = {
    progress: (apiKey: number) => ["setup-progress", apiKey] as const,
};

export const configurationKeys = {
    current: (apiKey: number) => ["configuration", apiKey, "current"] as const,
};

export const sourceKeys = {
    root: (apiKey: number) => ["sources", apiKey] as const,
    page: (apiKey: number, cursor: string | null, limit: number) =>
        ["sources", apiKey, "page", cursor, limit] as const,
};

export const syncKeys = {
    root: (apiKey: number) => ["sync", apiKey] as const,
    health: (apiKey: number) => ["sync", apiKey, "health"] as const,
    task: (apiKey: number, taskId: string) =>
        ["sync", apiKey, "task", taskId] as const,
    result: (
        apiKey: number,
        syncRunId: string,
        cursor: string | null,
        limit: number,
    ) => ["sync", apiKey, "result", syncRunId, cursor, limit] as const,
};

export const intelFeedKeys = {
    root: (apiKey: number) => ["intel-feed", apiKey] as const,
    pages: (apiKey: number, input: Omit<QueryIntelFeedInputV1, "cursor">) =>
        ["intel-feed", apiKey, "pages", input] as const,
};

export function createAppQueryClient() {
    return new QueryClient({
        defaultOptions: {
            queries: { retry: false, staleTime: Number.POSITIVE_INFINITY },
        },
    });
}
