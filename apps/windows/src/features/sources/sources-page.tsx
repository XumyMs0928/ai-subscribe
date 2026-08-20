import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useEffect, useMemo, useRef, useState } from "react";

import { useDesktopApi } from "../../app/providers/use-desktop-api";
import { Button } from "../../components/ui/button";
import type {
    SyncTargetV1,
    TaskSnapshotV1,
    TaskStateV1,
} from "../../lib/desktop-api/desktop-api";
import {
    configurationKeys,
    desktopApiQueryKey,
    sourceKeys,
    syncKeys,
} from "../../lib/query-client";
import {
    healthPollInterval,
    isActiveTaskState,
    isRetryDeadlinePending,
    sourceHasActiveTask,
    taskPollInterval,
} from "./sync-queries";

const TASK_POLL_INTERVAL_MS = 1_500;
const HEALTH_POLL_INTERVAL_MS = 3_000;
const FOREGROUND_SYNC_BUDGET_MS = 30_000;

async function sourceIdempotencyKey(revision: number, url: string) {
    const input = new TextEncoder().encode(`${revision}\u0000${url}`);
    const digest = await crypto.subtle.digest("SHA-256", input);
    const hash = Array.from(new Uint8Array(digest), (byte) =>
        byte.toString(16).padStart(2, "0"),
    ).join("");
    return `source-save-${revision}-${hash}`;
}

function errorCode(error: unknown) {
    return error instanceof Error &&
        "code" in error &&
        typeof error.code === "string"
        ? error.code
        : "internal.desktop_contract_mismatch";
}

function taskStateLabel(state: TaskStateV1) {
    const labels: Record<TaskStateV1, string> = {
        queued: "排队中",
        running: "同步中",
        retry_wait: "等待重试",
        succeeded: "同步成功",
        partially_succeeded: "部分来源同步成功",
        failed: "同步失败",
        cancelled: "已取消",
    };
    return labels[state];
}

function readinessLabel(
    status: "not_configured" | "ready" | "syncing" | "blocked",
    latestTaskState: TaskStateV1 | undefined,
) {
    if (latestTaskState === "partially_succeeded")
        return "部分来源同步成功，其他 RSS/Atom 来源仍需处理。";
    if (status === "ready") return "Windows RSS 最小闭环就绪";
    if (status === "syncing") return "RSS/Atom 正在同步。";
    if (status === "blocked")
        return "RSS/Atom 当前未就绪；请查看来源级状态后重试。";
    return "尚未配置已启用的 RSS/Atom 来源。";
}

function sourceReadinessLabel(status: string | undefined) {
    switch (status) {
        case "available":
            return "可用";
        case "syncing":
            return "同步中";
        case "rate_limited":
            return "已限流";
        case "retry_wait":
            return "等待重试";
        case "failed":
            return "失败";
        case "disabled":
            return "已停用";
        case "not_configured":
            return "未配置";
        default:
            return "状态待确认";
    }
}

function targetKey(target: SyncTargetV1) {
    return target.kind === "all_enabled_rss_atom"
        ? "all-enabled-rss-atom"
        : `source-${target.source_id}`;
}

function newestTaskProjection(
    observed: TaskSnapshotV1 | undefined,
    health: TaskSnapshotV1 | null | undefined,
) {
    if (!observed) return health ?? null;
    if (!health) return observed;
    if (observed.task_id === health.task_id)
        return observed.revision >= health.revision ? observed : health;
    const observedActive = isActiveTaskState(observed.state);
    const healthActive = isActiveTaskState(health.state);
    if (observedActive !== healthActive)
        return observedActive ? observed : health;
    if (observed.updated_at !== health.updated_at)
        return observed.updated_at > health.updated_at ? observed : health;
    return observed.task_id > health.task_id ? observed : health;
}

export function SourcesPage() {
    const api = useDesktopApi();
    const apiKey = desktopApiQueryKey(api);
    const queryClient = useQueryClient();
    const inputRef = useRef<HTMLInputElement>(null);
    const syncIntentKeys = useRef(new Map<string, string>());
    const handledTaskRevision = useRef<string | null>(null);
    const [url, setUrl] = useState("");
    const [savedMessage, setSavedMessage] = useState<string | null>(null);
    const [observedTaskId, setObservedTaskId] = useState<string | null>(null);
    const configuration = useQuery({
        queryKey: configurationKeys.current(apiKey),
        queryFn: () => api.configuration(),
    });
    const sources = useQuery({
        queryKey: sourceKeys.page(apiKey, null, 100),
        queryFn: () => api.querySources(null, 100),
    });
    const health = useQuery({
        queryKey: syncKeys.health(apiKey),
        queryFn: () => api.syncHealth(),
        refetchInterval: (query) =>
            healthPollInterval(query.state.data, HEALTH_POLL_INTERVAL_MS),
    });
    const healthActiveTaskId = isActiveTaskState(
        health.data?.latest_task?.state,
    )
        ? (health.data?.latest_task?.task_id ?? null)
        : null;
    const taskId = observedTaskId ?? healthActiveTaskId;
    const task = useQuery({
        queryKey: syncKeys.task(apiKey, taskId ?? "inactive"),
        queryFn: () => api.task(taskId as string),
        enabled: taskId !== null,
        refetchInterval: (query) =>
            taskPollInterval(query.state.data, TASK_POLL_INTERVAL_MS),
    });
    const sourceFailureCode = sources.isError ? errorCode(sources.error) : null;
    const isReadOnlyMigrationFailure = sourceFailureCode === "migration.source";
    const isSourceWriteBlocked =
        isReadOnlyMigrationFailure || sourceFailureCode === "storage.source";
    const save = useMutation({
        mutationFn: async (value: string) => {
            const revision = configuration.data?.revision;
            if (revision === undefined)
                throw new Error("configuration unavailable");
            return api.saveSource({
                contract_version: 1,
                source_kind: "rss_atom",
                url: value,
                expected_configuration_revision: revision,
                idempotency_key: await sourceIdempotencyKey(revision, value),
            });
        },
        onSuccess: async (source, submittedUrl) => {
            setSavedMessage(`已保存 ${source.display_url}`);
            setUrl((current) =>
                current.trim() === submittedUrl ? "" : current,
            );
            await Promise.all([
                queryClient.invalidateQueries({
                    queryKey: sourceKeys.root(apiKey),
                }),
                queryClient.invalidateQueries({
                    queryKey: configurationKeys.current(apiKey),
                }),
                queryClient.invalidateQueries({
                    queryKey: syncKeys.health(apiKey),
                }),
            ]);
        },
        onError: (failure) => {
            const code = errorCode(failure);
            if (
                code === "conflict.configuration_revision" ||
                code === "timeout.desktop_command"
            ) {
                void queryClient.invalidateQueries({
                    queryKey: configurationKeys.current(apiKey),
                });
                void queryClient.invalidateQueries({
                    queryKey: sourceKeys.root(apiKey),
                });
            }
            inputRef.current?.focus();
        },
    });
    const startSync = useMutation({
        mutationFn: async (target: SyncTargetV1) => {
            const key = targetKey(target);
            let intentKey = syncIntentKeys.current.get(key);
            if (!intentKey) {
                intentKey = `rss-sync-${crypto.randomUUID()}`;
                syncIntentKeys.current.set(key, intentKey);
            }
            try {
                return await api.startSync({
                    contract_version: 1,
                    target,
                    idempotency_key: intentKey,
                    foreground_budget_ms: FOREGROUND_SYNC_BUDGET_MS,
                });
            } catch (failure) {
                if (errorCode(failure) !== "timeout.desktop_command")
                    syncIntentKeys.current.delete(key);
                throw failure;
            }
        },
        onSuccess: async (taskRef, target) => {
            syncIntentKeys.current.delete(targetKey(target));
            setObservedTaskId(taskRef.task_id);
            await queryClient.invalidateQueries({
                queryKey: syncKeys.root(apiKey),
            });
        },
    });

    useEffect(() => {
        if (!task.data) return;
        const marker = `${task.data.task_id}:${task.data.revision}`;
        if (handledTaskRevision.current === marker) return;
        handledTaskRevision.current = marker;
        void queryClient.invalidateQueries({
            queryKey: syncKeys.health(apiKey),
        });
        void queryClient.invalidateQueries({
            queryKey: sourceKeys.root(apiKey),
        });
    }, [apiKey, queryClient, task.data]);

    const latestTask = newestTaskProjection(
        task.data,
        health.data?.latest_task,
    );
    const sourceResults = useMemo(
        () => [
            ...(latestTask?.sources ?? []),
            ...(health.data?.source_results ?? []),
        ],
        [health.data?.source_results, latestTask],
    );
    const taskBlocksNewSync = sourceResults.some(
        (source) =>
            source.state === "queued" ||
            source.state === "running" ||
            (source.state === "retry_wait" &&
                isRetryDeadlinePending(source.next_allowed_at)),
    );
    const anyActiveTask = startSync.isPending || taskBlocksNewSync;
    const enabledSourceCount =
        sources.data?.items.filter((source) => source.enabled).length ?? 0;
    const syncUnavailable = health.isPending || health.isError;

    const submit = (event: FormEvent) => {
        event.preventDefault();
        setSavedMessage(null);
        if (save.isPending || !url.trim()) return;
        save.mutate(url.trim());
    };

    return (
        <main className="sources-page">
            <header>
                <p className="eyebrow">AI SUBSCRIBE · WINDOWS</p>
                <h1>RSS / Atom 来源</h1>
                <p>当前仅 RSS/Atom；仅此 Windows 设备。</p>
                <p>
                    真实网络请求仅访问已验证的公开 HTTPS RSS/Atom 地址；
                    同步期间仍可添加来源、浏览和滚动。
                </p>
            </header>

            <section aria-labelledby="sync-health-title" aria-live="polite">
                <div className="source-list-heading">
                    <h2 id="sync-health-title">RSS/Atom 同步状态</h2>
                    <Button
                        id="sync-all-rss-button"
                        type="button"
                        onClick={() =>
                            startSync.mutate({ kind: "all_enabled_rss_atom" })
                        }
                        disabled={
                            enabledSourceCount === 0 ||
                            anyActiveTask ||
                            syncUnavailable
                        }
                        aria-describedby="sync-all-explanation"
                    >
                        {startSync.isPending
                            ? "正在创建任务…"
                            : "同步全部 RSS/Atom"}
                    </Button>
                </div>
                {health.isPending && <p role="status">正在读取同步健康状态…</p>}
                {health.isError && (
                    <p role="alert">
                        同步健康状态暂时不可用（{errorCode(health.error)}），
                        为避免重复任务，已暂时禁用同步。
                    </p>
                )}
                {health.data && (
                    <>
                        <p className="sync-readiness" role="status">
                            {readinessLabel(
                                health.data.readiness.status,
                                latestTask?.state,
                            )}
                        </p>
                        <p>
                            最后成功：
                            {health.data.last_success_at ?? "尚无成功同步"}
                            ；新鲜度：{health.data.freshness ?? "待首次同步"}
                        </p>
                        <p>待处理任务：{health.data.pending_task_count}</p>
                    </>
                )}
                {latestTask && (
                    <div data-testid="latest-sync-task">
                        <p>
                            当前任务：{taskStateLabel(latestTask.state)}（修订版
                            {latestTask.revision}）
                        </p>
                        {latestTask.result_ref !== null &&
                            [
                                "succeeded",
                                "partially_succeeded",
                                "failed",
                                "cancelled",
                            ].includes(latestTask.state) && (
                                <a href={`/sync/${latestTask.result_ref}`}>
                                    查看本轮结果
                                </a>
                            )}
                    </div>
                )}
                {startSync.isError && (
                    <p role="alert">
                        未能创建同步任务（{errorCode(startSync.error)}）。
                        {errorCode(startSync.error) ===
                        "timeout.desktop_command"
                            ? "再次尝试会复用同一同步意图，不会自动创建重复任务。"
                            : "请查看来源状态后重试。"}
                    </p>
                )}
                <p id="sync-all-explanation">
                    {enabledSourceCount === 0
                        ? "添加并启用至少一个 RSS/Atom 来源后才能同步全部。"
                        : anyActiveTask
                          ? "已有同步任务正在处理；终态后按钮会重新启用。"
                          : "只同步当前设备已启用的 RSS/Atom 来源；各来源独立提交。"}
                </p>
            </section>

            {configuration.isError && !configuration.data && (
                <div role="alert" id="source-configuration-error">
                    当前设备配置暂时不可用（{errorCode(configuration.error)}），
                    因此不能安全添加来源。
                    <Button
                        type="button"
                        onClick={() => void configuration.refetch()}
                    >
                        重试配置
                    </Button>
                </div>
            )}
            <form onSubmit={submit} aria-busy={save.isPending}>
                <label htmlFor="source-url">公开 HTTPS Feed 地址</label>
                <div className="source-form-row">
                    <input
                        ref={inputRef}
                        id="source-url"
                        type="url"
                        inputMode="url"
                        required
                        placeholder="请输入公开 HTTPS Feed 地址"
                        value={url}
                        onChange={(event) => setUrl(event.target.value)}
                        aria-describedby={
                            save.isError
                                ? "source-save-error"
                                : "source-url-help"
                        }
                    />
                    <Button
                        id="source-save-button"
                        type="submit"
                        disabled={
                            save.isPending ||
                            !configuration.data ||
                            isSourceWriteBlocked
                        }
                    >
                        {save.isPending ? "正在安全验证…" : "添加来源"}
                    </Button>
                </div>
                <p id="source-url-help">
                    保存前由共享核心检查 DNS、TLS、重定向、响应预算和 Feed
                    格式。
                </p>
                {save.isError && (
                    <p id="source-save-error" role="alert">
                        来源未保存（{errorCode(save.error)}
                        ）。请修正地址或稍后重试；输入已保留。
                    </p>
                )}
                {savedMessage && <p role="status">{savedMessage}</p>}
            </form>

            <section
                aria-labelledby="source-list-title"
                aria-busy={sources.isFetching}
            >
                <div className="source-list-heading">
                    <h2 id="source-list-title">当前来源</h2>
                    <Button
                        id="source-refresh-button"
                        type="button"
                        onClick={() => void sources.refetch()}
                        disabled={sources.isFetching}
                    >
                        {sources.isFetching && sources.data
                            ? "刷新中…"
                            : "刷新"}
                    </Button>
                </div>
                {sources.isPending && <p role="status">正在读取本机来源…</p>}
                {sources.isError && !sources.data && (
                    <div role="alert">
                        {isReadOnlyMigrationFailure
                            ? "来源数据库升级失败，当前页面为只读；现有配置不会被改写。"
                            : sourceFailureCode === "storage.source"
                              ? "来源存储读取失败，已阻断写入以避免部分保存。"
                              : "来源读取暂时失败，可安全重试。"}
                        （{sourceFailureCode}）
                        <Button
                            type="button"
                            onClick={() => void sources.refetch()}
                        >
                            重试
                        </Button>
                    </div>
                )}
                {sources.isError && sources.data && (
                    <p role="alert">刷新失败，保留上次来源列表。</p>
                )}
                {sources.data?.items.length === 0 && (
                    <p id="source-empty-state" role="status">
                        尚未添加 RSS / Atom 来源。
                    </p>
                )}
                {sources.data && sources.data.items.length > 0 && (
                    <ul className="source-list">
                        {sources.data.items.map((source) => {
                            const result = sourceResults.find(
                                (candidate) =>
                                    candidate.source_id === source.source_id,
                            );
                            const readiness =
                                health.data?.readiness.sources.find(
                                    (candidate) =>
                                        candidate.source_id ===
                                        source.source_id,
                                );
                            const nextAllowedAt =
                                result?.next_allowed_at ??
                                readiness?.next_allowed_at ??
                                source.next_allowed_at;
                            const retryPending =
                                isRetryDeadlinePending(nextAllowedAt);
                            const active = sourceHasActiveTask(
                                source.source_id,
                                sourceResults,
                            );
                            const disabledReason = !source.enabled
                                ? "来源已停用，不能同步。"
                                : retryPending
                                  ? `服务端要求等待至 ${nextAllowedAt}。`
                                  : active
                                    ? "该来源已有同步任务。"
                                    : syncUnavailable
                                      ? "同步健康状态不可用，已避免重复任务。"
                                      : null;
                            return (
                                <li key={source.source_id}>
                                    <strong>{source.display_url}</strong>
                                    <span>
                                        {source.enabled ? "已启用" : "已停用"}
                                    </span>
                                    <span>
                                        来源状态：
                                        {sourceReadinessLabel(
                                            readiness?.status,
                                        )}
                                    </span>
                                    <span>
                                        最后成功：
                                        {result?.last_success_at ??
                                            source.last_success_at ??
                                            "尚未同步"}
                                    </span>
                                    {result?.error_code && (
                                        <span role="alert">
                                            来源错误：{result.error_code}
                                        </span>
                                    )}
                                    <Button
                                        type="button"
                                        onClick={() =>
                                            startSync.mutate({
                                                kind: "source_id",
                                                source_id: source.source_id,
                                            })
                                        }
                                        disabled={
                                            startSync.isPending ||
                                            disabledReason !== null
                                        }
                                        aria-describedby={`source-sync-reason-${source.source_id}`}
                                    >
                                        立即同步
                                    </Button>
                                    <span
                                        id={`source-sync-reason-${source.source_id}`}
                                    >
                                        {disabledReason ??
                                            "只同步此 RSS/Atom 来源。"}
                                    </span>
                                </li>
                            );
                        })}
                    </ul>
                )}
            </section>
        </main>
    );
}
