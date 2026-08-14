import { QueryClient } from "@tanstack/react-query";
import type { DesktopApi } from "./desktop-api/desktop-api";

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

export function createAppQueryClient() {
    return new QueryClient({
        defaultOptions: {
            queries: { retry: false, staleTime: Number.POSITIVE_INFINITY },
        },
    });
}
