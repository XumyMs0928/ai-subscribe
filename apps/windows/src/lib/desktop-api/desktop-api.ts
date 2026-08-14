export interface HealthStatusV1 {
    readonly contract_version: 1;
    readonly status: "ok";
    readonly checked_at: string | null;
}

export interface DesktopApiError {
    readonly contract_version: 1;
    readonly code: string;
    readonly category: string;
    readonly message_key: string;
    readonly retryability: string;
    readonly source_id: string | null;
    readonly task_id: string | null;
    readonly details_allowlisted: string;
    readonly correlation_id: string;
}

export interface DesktopApi {
    health(): Promise<HealthStatusV1>;
    demoBootstrap(): Promise<DemoCatalogV1>;
    demoSearch(query: string, track: string | null): Promise<DemoCatalogV1>;
    demoList(cursor: string | null, limit: number): Promise<DemoPageV1>;
    demoFilter(
        track: string,
        cursor: string | null,
        limit: number,
    ): Promise<DemoPageV1>;
    demoDetail(id: string): Promise<DemoItemV1>;
}

export interface DemoItemV1 {
    readonly id: string;
    readonly data_origin: "demo";
    readonly publisher: string;
    readonly title: string;
    readonly track: string;
    readonly summary: string;
    readonly original_url: string;
    readonly published_at: string;
    readonly collected_at: string;
}

export interface DemoCatalogV1 {
    readonly contract_version: 1;
    readonly dataset_id: "demo-v1";
    readonly items: readonly DemoItemV1[];
}

export class DesktopContractError extends Error {
    readonly code = "internal.desktop_contract_mismatch";

    constructor() {
        super("The desktop bridge returned an invalid v1 health contract.");
        this.name = "DesktopContractError";
    }
}

export class DesktopCommandError extends Error implements DesktopApiError {
    readonly contract_version: 1;
    readonly code: string;
    readonly category: string;
    readonly message_key: string;
    readonly retryability: string;
    readonly source_id: string | null;
    readonly task_id: string | null;
    readonly details_allowlisted: string;
    readonly correlation_id: string;

    constructor(error: DesktopApiError) {
        super(error.message_key);
        this.name = "DesktopCommandError";
        this.contract_version = error.contract_version;
        this.code = error.code;
        this.category = error.category;
        this.message_key = error.message_key;
        this.retryability = error.retryability;
        this.source_id = error.source_id;
        this.task_id = error.task_id;
        this.details_allowlisted = error.details_allowlisted;
        this.correlation_id = error.correlation_id;
    }
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return !!value && typeof value === "object";
}

function isNullableString(value: unknown): value is string | null {
    return value === null || typeof value === "string";
}

function isRfc3339Utc(value: string): boolean {
    const match =
        /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?Z$/.exec(
            value,
        );
    if (!match) return false;
    const [, yearText, monthText, dayText, hourText, minuteText, secondText] =
        match;
    const year = Number(yearText);
    const month = Number(monthText);
    const day = Number(dayText);
    const hour = Number(hourText);
    const minute = Number(minuteText);
    const second = Number(secondText);
    const leapYear = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
    const days = [
        31,
        leapYear ? 29 : 28,
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    return (
        month >= 1 &&
        month <= 12 &&
        day >= 1 &&
        day <= days[month - 1] &&
        hour <= 23 &&
        minute <= 59 &&
        second <= 60
    );
}

export interface DemoPageV1 extends DemoCatalogV1 {
    readonly next_cursor: string | null;
}

function isNonEmptyTrimmed(value: unknown): value is string {
    return (
        typeof value === "string" && value.length > 0 && value.trim() === value
    );
}

function isHttpsUrl(value: unknown): value is string {
    if (typeof value !== "string") return false;
    try {
        const parsed = new URL(value);
        return parsed.protocol === "https:" && parsed.hostname.length > 0;
    } catch {
        return false;
    }
}

export function isHealthStatusV1(value: unknown): value is HealthStatusV1 {
    if (!isRecord(value)) return false;
    const record = value;
    return (
        record.contract_version === 1 &&
        record.status === "ok" &&
        Object.hasOwn(record, "checked_at") &&
        (record.checked_at === null ||
            (typeof record.checked_at === "string" &&
                isRfc3339Utc(record.checked_at)))
    );
}

export function isDesktopApiError(value: unknown): value is DesktopApiError {
    if (!isRecord(value)) return false;
    return (
        value.contract_version === 1 &&
        typeof value.code === "string" &&
        typeof value.category === "string" &&
        typeof value.message_key === "string" &&
        typeof value.retryability === "string" &&
        isNullableString(value.source_id) &&
        isNullableString(value.task_id) &&
        typeof value.details_allowlisted === "string" &&
        typeof value.correlation_id === "string" &&
        value.correlation_id.length > 0
    );
}

export function isDemoItemV1(value: unknown): value is DemoItemV1 {
    if (!isRecord(value)) return false;
    return (
        typeof value.id === "string" &&
        /^demo:[A-Za-z0-9._:-]+$/.test(value.id) &&
        value.id.length > 5 &&
        value.id.length <= 128 &&
        value.data_origin === "demo" &&
        isNonEmptyTrimmed(value.publisher) &&
        isNonEmptyTrimmed(value.title) &&
        isNonEmptyTrimmed(value.track) &&
        isNonEmptyTrimmed(value.summary) &&
        isHttpsUrl(value.original_url) &&
        typeof value.published_at === "string" &&
        isRfc3339Utc(value.published_at) &&
        typeof value.collected_at === "string" &&
        isRfc3339Utc(value.collected_at)
    );
}

export function isDemoCatalogV1(value: unknown): value is DemoCatalogV1 {
    return (
        isRecord(value) &&
        value.contract_version === 1 &&
        value.dataset_id === "demo-v1" &&
        Array.isArray(value.items) &&
        value.items.every(isDemoItemV1) &&
        new Set(value.items.map((item) => item.id)).size === value.items.length
    );
}

export function isDemoBootstrapCatalogV1(
    value: unknown,
): value is DemoCatalogV1 {
    return isDemoCatalogV1(value) && value.items.length > 0;
}

export function isDemoPageV1(value: unknown): value is DemoPageV1 {
    return (
        isDemoCatalogV1(value) &&
        isRecord(value) &&
        Object.hasOwn(value, "next_cursor") &&
        (value.next_cursor === null ||
            (typeof value.next_cursor === "string" &&
                /^offset:\d+$/.test(value.next_cursor)))
    );
}
