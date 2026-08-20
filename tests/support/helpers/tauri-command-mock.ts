export const TAURI_COMMANDS = [
    "health_v1",
    "demo_bootstrap_v1",
    "demo_search_v1",
    "demo_list_v1",
    "demo_filter_v1",
    "demo_detail_v1",
    "setup_progress_v1",
    "save_setup_step_v1",
    "configuration_v1",
    "validate_configuration_v1",
    "save_configuration_v1",
    "save_source_v1",
    "query_sources_v1",
    "start_sync_v1",
    "task_v1",
    "sync_health_v1",
    "get_sync_result_v1",
    "query_intel_feed_v1",
] as const;

export type TauriCommand = (typeof TAURI_COMMANDS)[number];

export type TauriCommandBehavior =
    | { readonly kind: "response"; readonly value: unknown }
    | { readonly kind: "error"; readonly error: unknown };

export type TauriCommandOverrides = Partial<
    Record<TauriCommand, TauriCommandBehavior>
>;

export interface TauriInvokeCall {
    readonly command: string;
    readonly args: Record<string, unknown> | null;
}

export interface ExternalCall {
    readonly kind:
        | "fetch"
        | "xhr"
        | "websocket"
        | "sendBeacon"
        | "notification"
        | "browser_resource";
    readonly target: string;
    readonly method: string | null;
}

export function response(value: unknown): TauriCommandBehavior {
    return { kind: "response", value };
}

export function commandError(error: unknown): TauriCommandBehavior {
    return { kind: "error", error };
}

export function mergeCommandBehaviors(
    defaults: Record<TauriCommand, TauriCommandBehavior>,
    overrides: TauriCommandOverrides = {},
): Record<TauriCommand, TauriCommandBehavior> {
    return { ...defaults, ...overrides };
}
