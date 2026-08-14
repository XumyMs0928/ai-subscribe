export const TAURI_COMMANDS = [
    "health_v1",
    "demo_bootstrap_v1",
    "demo_search_v1",
    "demo_list_v1",
    "demo_filter_v1",
    "demo_detail_v1",
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
        "fetch" | "xhr" | "websocket" | "sendBeacon" | "notification";
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
