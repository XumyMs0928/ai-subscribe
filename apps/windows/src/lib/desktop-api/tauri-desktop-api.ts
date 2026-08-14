import { invoke } from "@tauri-apps/api/core";

import {
    DesktopContractError,
    DesktopCommandError,
    type DesktopApi,
    isDemoCatalogV1,
    isDemoBootstrapCatalogV1,
    isDemoItemV1,
    isDemoPageV1,
    isDesktopApiError,
    isHealthStatusV1,
} from "./desktop-api";

export function createTauriDesktopApi(): DesktopApi {
    async function invokeChecked<T>(
        command: string,
        args: Record<string, unknown> | undefined,
        guard: (value: unknown) => value is T,
    ): Promise<T> {
        try {
            const response: unknown =
                args === undefined
                    ? await invoke(command)
                    : await invoke(command, args);
            if (!guard(response)) throw new DesktopContractError();
            return response;
        } catch (error) {
            if (error instanceof DesktopContractError) throw error;
            if (isDesktopApiError(error)) throw new DesktopCommandError(error);
            throw new DesktopContractError();
        }
    }
    return {
        async health() {
            return invokeChecked("health_v1", undefined, isHealthStatusV1);
        },
        async demoBootstrap() {
            return invokeChecked(
                "demo_bootstrap_v1",
                undefined,
                isDemoBootstrapCatalogV1,
            );
        },
        async demoSearch(query, track) {
            return invokeChecked(
                "demo_search_v1",
                { query, track },
                isDemoCatalogV1,
            );
        },
        async demoList(cursor, limit) {
            return invokeChecked(
                "demo_list_v1",
                { cursor, limit },
                isDemoPageV1,
            );
        },
        async demoFilter(track, cursor, limit) {
            return invokeChecked(
                "demo_filter_v1",
                { track, cursor, limit },
                isDemoPageV1,
            );
        },
        async demoDetail(id) {
            return invokeChecked("demo_detail_v1", { id }, isDemoItemV1);
        },
    };
}
