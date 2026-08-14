import type {
    DemoCatalogV1,
    DemoItemV1,
    DemoPageV1,
    DesktopApiError,
    HealthStatusV1,
} from "../../../apps/windows/src/lib/desktop-api/desktop-api";

const DEFAULT_ITEMS: readonly DemoItemV1[] = [
    {
        id: "demo:openai-agents-sdk-001",
        data_origin: "demo",
        publisher: "OpenAI",
        title: "Agents SDK 发布新的会话追踪能力",
        track: "AI Agent",
        summary: "固定演示样本：展示如何从来源事实形成可核验的情报摘要。",
        original_url: "https://openai.com/",
        published_at: "2026-07-01T08:00:00Z",
        collected_at: "2026-07-01T09:00:00Z",
    },
    {
        id: "demo:rust-197-001",
        data_origin: "demo",
        publisher: "Rust Project",
        title: "Rust 1.97 提升工具链诊断体验",
        track: "开发工具",
        summary: "固定演示样本：说明版本变化、影响范围与原始来源。",
        original_url: "https://www.rust-lang.org/",
        published_at: "2026-06-20T10:00:00Z",
        collected_at: "2026-06-20T10:30:00Z",
    },
    {
        id: "demo:local-model-001",
        data_origin: "demo",
        publisher: "AI Subscribe Demo",
        title: "本地模型推理的内存占用进一步下降",
        track: "本地模型",
        summary: "固定演示样本：帮助用户理解离线情报浏览和基础筛选。",
        original_url: "https://example.com/ai-subscribe-demo/local-model",
        published_at: "2026-06-10T02:00:00Z",
        collected_at: "2026-06-10T03:00:00Z",
    },
];

export function createDemoItem(
    overrides: Partial<DemoItemV1> = {},
): DemoItemV1 {
    return { ...DEFAULT_ITEMS[0], ...overrides };
}

export function createDemoCatalog(
    overrides: Partial<DemoCatalogV1> = {},
): DemoCatalogV1 {
    return {
        contract_version: 1,
        dataset_id: "demo-v1",
        items: DEFAULT_ITEMS.map((item) => ({ ...item })),
        ...overrides,
    };
}

export function createDemoPage(
    overrides: Partial<DemoPageV1> = {},
): DemoPageV1 {
    return {
        ...createDemoCatalog(),
        next_cursor: null,
        ...overrides,
    };
}

export function createHealthStatus(
    overrides: Partial<HealthStatusV1> = {},
): HealthStatusV1 {
    return {
        contract_version: 1,
        status: "ok",
        checked_at: "2026-06-22T09:30:00Z",
        ...overrides,
    };
}

export function createDesktopApiError(
    overrides: Partial<DesktopApiError> = {},
): DesktopApiError {
    return {
        contract_version: 1,
        code: "internal.demo_fixture_error",
        category: "internal",
        message_key: "error.demo_fixture",
        retryability: "retryable",
        source_id: null,
        task_id: null,
        details_allowlisted: "deterministic Playwright fixture error",
        correlation_id: "pw-demo-error-001",
        ...overrides,
    };
}
