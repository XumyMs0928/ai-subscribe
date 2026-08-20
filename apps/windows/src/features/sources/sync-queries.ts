import type {
    SourceSyncStatusV1,
    SyncHealthSummaryV1,
    TaskSnapshotV1,
    TaskStateV1,
} from "../../lib/desktop-api/desktop-api";

const MAX_RETRY_WAIT_POLL_MS = 30_000;

export function isActiveTaskState(state: TaskStateV1 | undefined) {
    return state === "queued" || state === "running" || state === "retry_wait";
}

export function sourceHasActiveTask(
    sourceId: string,
    sourceResults: readonly SourceSyncStatusV1[],
) {
    return sourceResults.some(
        (result) =>
            result.source_id === sourceId &&
            (result.state === "queued" ||
                result.state === "running" ||
                (result.state === "retry_wait" &&
                    isRetryDeadlinePending(result.next_allowed_at))),
    );
}

export function isRetryDeadlinePending(nextAllowedAt: string | null) {
    if (nextAllowedAt === null) return false;
    const deadline = Date.parse(nextAllowedAt);
    return Number.isFinite(deadline) && deadline > Date.now();
}

export function taskPollInterval(
    task: TaskSnapshotV1 | null | undefined,
    activeIntervalMs: number,
    nowMs = Date.now(),
    retrySources?: readonly SourceSyncStatusV1[],
): number | false {
    if (!task) return false;
    if (task.state === "queued" || task.state === "running")
        return activeIntervalMs;
    if (task.state !== "retry_wait") return false;

    const futureDeadlines = (retrySources ?? task.sources)
        .map((source) =>
            source.next_allowed_at === null
                ? Number.NaN
                : Date.parse(source.next_allowed_at),
        )
        .filter((deadline) => Number.isFinite(deadline) && deadline > nowMs);
    if (futureDeadlines.length === 0) return false;
    const remainingMs = Math.min(...futureDeadlines) - nowMs;
    return Math.max(1, Math.min(remainingMs, MAX_RETRY_WAIT_POLL_MS));
}

export function healthPollInterval(
    health: SyncHealthSummaryV1 | undefined,
    activeIntervalMs: number,
    nowMs = Date.now(),
) {
    const interval = taskPollInterval(
        health?.latest_task,
        activeIntervalMs,
        nowMs,
        health?.source_results,
    );
    if (interval !== false) return interval;
    if (
        health !== undefined &&
        health.pending_task_count > 0 &&
        health.source_results.some((source) =>
            ["queued", "running"].includes(source.state),
        )
    )
        return activeIntervalMs;
    return false;
}
