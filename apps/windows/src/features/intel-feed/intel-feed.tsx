import {
    type InfiniteData,
    useInfiniteQuery,
    useQuery,
    useQueryClient,
} from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { Link, useLocation, useSearchParams } from "react-router";

import { useDesktopApi } from "../../app/providers/use-desktop-api";
import { Button } from "../../components/ui/button";
import type {
    IntelFeedFiltersV1,
    IntelFeedPageV1,
    IntelFeedStreamV1,
} from "../../lib/desktop-api/desktop-api";
import { DesktopContractError } from "../../lib/desktop-api/desktop-api";
import {
    desktopApiQueryKey,
    intelFeedKeys,
    syncKeys,
} from "../../lib/query-client";
import { FeedFilters } from "./feed-filters";
import { IntelligenceFeedItem } from "./intelligence-feed-item";

const PAGE_SIZE = 30;
const DEFAULT_FILTERS: IntelFeedFiltersV1 = {
    track_ids: [],
    source_ids: [],
    time_window: "all_time",
    importance: [],
};

function stableErrorCode(error: unknown) {
    return error instanceof Error &&
        "code" in error &&
        typeof error.code === "string"
        ? error.code
        : "internal.desktop_contract_mismatch";
}

function canonicalValues(value: string): readonly string[] {
    return [
        ...new Set(
            value
                .split(",")
                .map((entry) => entry.trim())
                .filter(Boolean),
        ),
    ].sort();
}

function sameProjection(left: IntelFeedPageV1, right: IntelFeedPageV1) {
    return (
        left.stream === right.stream &&
        left.sort === right.sort &&
        left.rule_version === right.rule_version &&
        left.configuration_revision === right.configuration_revision &&
        left.configuration_hash === right.configuration_hash &&
        left.as_of_ms === right.as_of_ms &&
        left.filters.time_window === right.filters.time_window &&
        sameValues(left.filters.track_ids, right.filters.track_ids) &&
        sameValues(left.filters.source_ids, right.filters.source_ids) &&
        sameValues(left.filters.importance, right.filters.importance)
    );
}

function sameValues(left: readonly string[], right: readonly string[]) {
    return (
        left.length === right.length &&
        left.every((value, index) => value === right[index])
    );
}

function validateFilterIds(
    tracks: readonly string[],
    sources: readonly string[],
): string | null {
    if (tracks.length > 32) return "赛道 ID 最多可填写 32 个。";
    if (sources.length > 64) return "来源 ID 最多可填写 64 个。";
    if (tracks.some((value) => !/^[A-Za-z0-9_.:-]{1,128}$/.test(value)))
        return "赛道 ID 只能包含字母、数字、下划线、连字符、点和冒号，且最长 128 字节。";
    if (sources.some((value) => !/^source:[0-9a-f]{24}$/.test(value)))
        return "来源 ID 必须使用 source: 加 24 位小写十六进制字符。";
    return null;
}

export function IntelFeed() {
    const desktopApi = useDesktopApi();
    const queryClient = useQueryClient();
    const location = useLocation();
    const apiKey = desktopApiQueryKey(desktopApi);
    const [searchParams, setSearchParams] = useSearchParams();
    const stream: IntelFeedStreamV1 =
        searchParams.get("stream") === "ordinary_candidate"
            ? "ordinary_candidate"
            : "high_value";
    const filters = useMemo<IntelFeedFiltersV1>(() => {
        const time = searchParams.get("time");
        const timeWindow = ["last_24h", "last_7d", "last_30d"].includes(
            String(time),
        )
            ? (time as IntelFeedFiltersV1["time_window"])
            : "all_time";
        const importance = canonicalValues(
            searchParams.get("importance") ?? "",
        ).filter(
            (value): value is "low" | "medium" | "high" =>
                value === "low" || value === "medium" || value === "high",
        );
        return {
            track_ids: canonicalValues(searchParams.get("tracks") ?? ""),
            source_ids: canonicalValues(searchParams.get("sources") ?? ""),
            time_window: timeWindow,
            importance,
        };
    }, [searchParams]);
    const [trackDraft, setTrackDraft] = useState(() =>
        filters.track_ids.join(","),
    );
    const [sourceDraft, setSourceDraft] = useState(() =>
        filters.source_ids.join(","),
    );
    const [timeDraft, setTimeDraft] = useState<
        IntelFeedFiltersV1["time_window"]
    >(filters.time_window);
    const [importanceDraft, setImportanceDraft] = useState<
        readonly ("low" | "medium" | "high")[]
    >(() => filters.importance);
    const [validationMessage, setValidationMessage] = useState<string | null>(
        null,
    );
    const [selectedId, setSelectedId] = useState<string | null>(null);
    const [selectionNotice, setSelectionNotice] = useState<string | null>(null);
    const [activationNotice, setActivationNotice] = useState<string | null>(
        null,
    );
    const itemRefs = useRef(new Map<string, HTMLButtonElement>());
    const previousItems = useRef<readonly string[]>([]);
    const visibleAnchor = useRef<{ id: string; top: number } | null>(null);
    const feedScrollOffset = useRef(0);
    const previousFeedRoute = useRef(
        location.pathname === "/" || location.pathname === "/intel",
    );

    useEffect(() => {
        // URL state is authoritative when navigation/reload restores the feed.
        queueMicrotask(() => {
            setTrackDraft(filters.track_ids.join(","));
            setSourceDraft(filters.source_ids.join(","));
            setTimeDraft(filters.time_window);
            setImportanceDraft(filters.importance);
            setValidationMessage(null);
        });
    }, [filters]);

    const isFeedRoute =
        location.pathname === "/" || location.pathname === "/intel";
    useEffect(() => {
        if (previousFeedRoute.current && !isFeedRoute) {
            feedScrollOffset.current = window.scrollY;
        } else if (!previousFeedRoute.current && isFeedRoute) {
            queueMicrotask(() => window.scrollTo(0, feedScrollOffset.current));
        }
        previousFeedRoute.current = isFeedRoute;
    }, [isFeedRoute]);

    const baseInput = useMemo(
        () => ({
            contract_version: 1 as const,
            stream,
            filters,
            sort: "score_desc" as const,
            limit: PAGE_SIZE,
        }),
        [filters, stream],
    );
    const feedQueryKey = useMemo(
        () => intelFeedKeys.pages(apiKey, baseInput),
        [apiKey, baseInput],
    );
    const feed = useInfiniteQuery({
        queryKey: feedQueryKey,
        initialPageParam: null as string | null,
        queryFn: async ({ pageParam }) => {
            const page = await desktopApi.queryIntelFeed({
                ...baseInput,
                cursor: pageParam,
            });
            if (pageParam !== null) {
                const cached =
                    queryClient.getQueryData<
                        InfiniteData<IntelFeedPageV1, string | null>
                    >(feedQueryKey);
                const first = cached?.pages[0];
                const priorIds = new Set(
                    cached?.pages.flatMap((entry) =>
                        entry.items.map((item) => item.intel_item_id),
                    ) ?? [],
                );
                if (
                    (first && !sameProjection(first, page)) ||
                    page.items.some((item) => priorIds.has(item.intel_item_id))
                ) {
                    throw new DesktopContractError();
                }
            }
            return page;
        },
        getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
    });
    const health = useQuery({
        queryKey: syncKeys.health(apiKey),
        queryFn: () => desktopApi.syncHealth(),
    });
    const items = useMemo(
        () => feed.data?.pages.flatMap((page) => page.items) ?? [],
        [feed.data],
    );
    const effectiveSelectedId = items.some(
        (item) => item.intel_item_id === selectedId,
    )
        ? selectedId
        : (items[0]?.intel_item_id ?? null);

    useEffect(() => {
        if (selectedId !== null && selectedId !== effectiveSelectedId) {
            const previousIndex = previousItems.current.indexOf(selectedId);
            const replacement =
                items[Math.min(Math.max(previousIndex, 0), items.length - 1)]
                    ?.intel_item_id ?? null;
            queueMicrotask(() => {
                setSelectionNotice(
                    "原选中项已不在当前结果中，已选择相邻可用情报。",
                );
                setSelectedId(replacement);
                if (replacement) itemRefs.current.get(replacement)?.focus();
            });
        }
        previousItems.current = items.map((item) => item.intel_item_id);
    }, [effectiveSelectedId, items, selectedId]);

    useEffect(() => {
        const rememberAnchor = () => {
            if (!effectiveSelectedId) return;
            const top = itemRefs.current
                .get(effectiveSelectedId)
                ?.getBoundingClientRect().top;
            if (top !== undefined)
                visibleAnchor.current = { id: effectiveSelectedId, top };
        };
        const restoreAfterResize = () => {
            const anchor = visibleAnchor.current;
            if (!anchor) return;
            queueMicrotask(() => {
                const current = itemRefs.current
                    .get(anchor.id)
                    ?.getBoundingClientRect().top;
                if (current !== undefined)
                    window.scrollBy(0, current - anchor.top);
            });
        };
        rememberAnchor();
        window.addEventListener("scroll", rememberAnchor, { passive: true });
        window.addEventListener("resize", restoreAfterResize);
        return () => {
            window.removeEventListener("scroll", rememberAnchor);
            window.removeEventListener("resize", restoreAfterResize);
        };
    }, [effectiveSelectedId]);

    function selectAndFocus(index: number) {
        if (!items.length) return;
        const normalized = (index + items.length) % items.length;
        const id = items[normalized].intel_item_id;
        setSelectionNotice(null);
        setSelectedId(id);
        queueMicrotask(() => itemRefs.current.get(id)?.focus());
    }

    function applyFilters() {
        const trackIds = canonicalValues(trackDraft);
        const sourceIds = canonicalValues(sourceDraft);
        const validation = validateFilterIds(trackIds, sourceIds);
        setValidationMessage(validation);
        if (validation) return;
        updateQuery(stream, {
            track_ids: trackIds,
            source_ids: sourceIds,
            time_window: timeDraft,
            importance: [...importanceDraft].sort(),
        });
    }

    function resetFilters() {
        setTrackDraft("");
        setSourceDraft("");
        setTimeDraft("all_time");
        setImportanceDraft([]);
        setValidationMessage(null);
        updateQuery(stream, DEFAULT_FILTERS);
    }

    function updateQuery(
        nextStream: IntelFeedStreamV1,
        nextFilters: IntelFeedFiltersV1,
    ) {
        const next = new URLSearchParams();
        if (nextStream === "ordinary_candidate") next.set("stream", nextStream);
        if (nextFilters.track_ids.length)
            next.set("tracks", nextFilters.track_ids.join(","));
        if (nextFilters.source_ids.length)
            next.set("sources", nextFilters.source_ids.join(","));
        if (nextFilters.time_window !== "all_time")
            next.set("time", nextFilters.time_window);
        if (nextFilters.importance.length)
            next.set("importance", nextFilters.importance.join(","));
        setSearchParams(next, { replace: true });
    }

    function switchStream(nextStream: IntelFeedStreamV1) {
        setSelectedId(null);
        setSelectionNotice(null);
        updateQuery(nextStream, filters);
    }

    function captureSelectedOffset() {
        const id = effectiveSelectedId;
        const top = id
            ? itemRefs.current.get(id)?.getBoundingClientRect().top
            : null;
        return id && top !== null && top !== undefined ? { id, top } : null;
    }

    function restoreSelectedOffset(anchor: { id: string; top: number } | null) {
        if (!anchor) return;
        queueMicrotask(() => {
            const current = itemRefs.current
                .get(anchor.id)
                ?.getBoundingClientRect().top;
            if (current !== undefined) window.scrollBy(0, current - anchor.top);
        });
    }

    async function refreshFromFirstPage() {
        const anchor = captureSelectedOffset();
        await queryClient.resetQueries({ queryKey: feedQueryKey, exact: true });
        restoreSelectedOffset(anchor);
    }

    const blockingError = feed.isError && !feed.data;
    return (
        <main className="app-shell intel-feed-shell">
            <header className="shell-header">
                <div>
                    <p className="eyebrow">AI SUBSCRIBE · WINDOWS</p>
                    <h1>情报</h1>
                    <p>真实 RSS/Atom · 当前设备本地读取</p>
                </div>
                <div className="feed-header-actions">
                    <Button
                        variant="secondary"
                        disabled={feed.isRefetching}
                        onClick={() => void refreshFromFirstPage()}
                    >
                        {feed.isRefetching ? "正在刷新…" : "刷新情报"}
                    </Button>
                    <Link to="/demo">查看明确标记的演示数据</Link>
                </div>
            </header>

            <div className="feed-stream-tabs" aria-label="情报流类型">
                <Button
                    variant={stream === "high_value" ? "primary" : "secondary"}
                    aria-pressed={stream === "high_value"}
                    onClick={() => switchStream("high_value")}
                >
                    高价值
                </Button>
                <Button
                    variant={
                        stream === "ordinary_candidate"
                            ? "primary"
                            : "secondary"
                    }
                    aria-pressed={stream === "ordinary_candidate"}
                    onClick={() => switchStream("ordinary_candidate")}
                >
                    普通候选
                </Button>
            </div>

            <FeedFilters
                track={trackDraft}
                source={sourceDraft}
                timeWindow={timeDraft}
                importance={importanceDraft}
                onTrackChange={setTrackDraft}
                onSourceChange={setSourceDraft}
                onTimeWindowChange={setTimeDraft}
                onImportanceChange={setImportanceDraft}
                onApply={applyFilters}
                onReset={resetFilters}
                validationMessage={validationMessage}
            />

            {Object.values(filters).some((value) =>
                Array.isArray(value) ? value.length : value !== "all_time",
            ) && (
                <p className="feed-filter-summary" aria-live="polite">
                    已应用筛选 · 可使用“恢复默认”清除
                </p>
            )}
            <div className="feed-filter-chips" aria-label="已启用筛选">
                {filters.track_ids.map((value) => (
                    <Button
                        key={`track-${value}`}
                        variant="secondary"
                        onClick={() => {
                            const next = {
                                ...filters,
                                track_ids: filters.track_ids.filter(
                                    (entry) => entry !== value,
                                ),
                            };
                            setTrackDraft(next.track_ids.join(","));
                            updateQuery(stream, next);
                        }}
                    >
                        赛道：{value} ×
                    </Button>
                ))}
                {filters.source_ids.map((value) => (
                    <Button
                        key={`source-${value}`}
                        variant="secondary"
                        onClick={() => {
                            const next = {
                                ...filters,
                                source_ids: filters.source_ids.filter(
                                    (entry) => entry !== value,
                                ),
                            };
                            setSourceDraft(next.source_ids.join(","));
                            updateQuery(stream, next);
                        }}
                    >
                        来源：{value} ×
                    </Button>
                ))}
                {filters.time_window !== "all_time" && (
                    <Button
                        variant="secondary"
                        onClick={() => {
                            const next = {
                                ...filters,
                                time_window: "all_time" as const,
                            };
                            setTimeDraft("all_time");
                            updateQuery(stream, next);
                        }}
                    >
                        时间：{filters.time_window} ×
                    </Button>
                )}
                {filters.importance.map((value) => (
                    <Button
                        key={`importance-${value}`}
                        variant="secondary"
                        onClick={() => {
                            const next = {
                                ...filters,
                                importance: filters.importance.filter(
                                    (entry) => entry !== value,
                                ),
                            };
                            setImportanceDraft(next.importance);
                            updateQuery(stream, next);
                        }}
                    >
                        重要度：{value} ×
                    </Button>
                ))}
            </div>
            {health.isError && (
                <p role="status" className="demo-inline-status">
                    同步状态暂时无法读取；继续显示已保存的本地情报。
                </p>
            )}
            {health.data?.readiness.status === "not_configured" && (
                <p role="status" className="demo-inline-status">
                    尚未配置可用 RSS 来源；本地已有情报仍可读取。
                </p>
            )}
            {health.data?.readiness.status === "blocked" && (
                <p role="status" className="demo-inline-status">
                    {health.data.readiness.sources.some(
                        (source) => source.status === "available",
                    )
                        ? "部分来源同步失败；继续显示已保存的本地情报。"
                        : "所有已配置来源当前均无法同步；继续显示已保存的本地情报。"}
                </p>
            )}
            {feed.isPending && (
                <div
                    role="status"
                    aria-busy="true"
                    aria-label="正在加载情报"
                    className="feed-skeleton"
                >
                    {[0, 1, 2].map((row) => (
                        <div className="feed-skeleton-row" key={row}>
                            <span />
                            <strong />
                            <span />
                            <span />
                        </div>
                    ))}
                </div>
            )}
            {blockingError && (
                <div role="alert" className="demo-message demo-error">
                    <strong>本地情报暂时无法读取</strong>
                    <p>现有数据未被修改，也没有执行外部请求。</p>
                    <code>{stableErrorCode(feed.error)}</code>
                    <Button onClick={() => void refreshFromFirstPage()}>
                        从首屏重试
                    </Button>
                </div>
            )}
            {feed.data && items.length === 0 && (
                <div className="demo-message">
                    <p>
                        {stream === "high_value"
                            ? "当前条件下没有高价值情报。"
                            : "当前条件下没有普通候选。"}
                    </p>
                    <p>
                        数据来自当前设备；新鲜度：
                        {health.data?.freshness ?? "尚无可用记录"}；最近同步：
                        {health.data?.last_success_at ?? "尚未成功同步"}。
                    </p>
                    <Button variant="secondary" onClick={resetFilters}>
                        恢复默认
                    </Button>
                    {stream === "high_value" && (
                        <Button
                            variant="secondary"
                            onClick={() => switchStream("ordinary_candidate")}
                        >
                            查看普通候选
                        </Button>
                    )}
                    <Link to="/sources">前往来源同步</Link>
                </div>
            )}
            {feed.data && items.length > 0 && (
                <section
                    className="feed-list"
                    aria-label="真实 RSS 情报列表"
                    aria-busy={feed.isFetching}
                >
                    {feed.isRefetching && !feed.isFetchingNextPage && (
                        <p className="demo-inline-status" aria-live="polite">
                            正在刷新，当前内容保持可用。
                        </p>
                    )}
                    {selectionNotice && (
                        <p className="demo-inline-status" aria-live="polite">
                            {selectionNotice}
                        </p>
                    )}
                    {activationNotice && (
                        <p className="demo-inline-status" aria-live="polite">
                            {activationNotice}
                        </p>
                    )}
                    {feed.isError && !feed.isFetchNextPageError && (
                        <div role="alert" className="demo-inline-error">
                            刷新失败，继续显示上次可用内容。
                            <Button
                                variant="secondary"
                                onClick={() => void refreshFromFirstPage()}
                            >
                                重试刷新
                            </Button>
                        </div>
                    )}
                    <ul>
                        {items.map((item, index) => (
                            <li key={item.intel_item_id}>
                                <IntelligenceFeedItem
                                    item={item}
                                    selected={
                                        item.intel_item_id ===
                                        effectiveSelectedId
                                    }
                                    tabIndex={
                                        item.intel_item_id ===
                                        effectiveSelectedId
                                            ? 0
                                            : -1
                                    }
                                    onSelect={() => {
                                        setSelectionNotice(null);
                                        setSelectedId(item.intel_item_id);
                                    }}
                                    onNavigate={(offset) =>
                                        selectAndFocus(index + offset)
                                    }
                                    onActivate={(intelItemId) =>
                                        setActivationNotice(
                                            `已选择 ${intelItemId}；证据详情将在下一阶段开放。`,
                                        )
                                    }
                                    onEscape={() => {
                                        setActivationNotice(null);
                                        itemRefs.current
                                            .get(item.intel_item_id)
                                            ?.focus();
                                    }}
                                    itemRef={(node) => {
                                        if (node)
                                            itemRefs.current.set(
                                                item.intel_item_id,
                                                node,
                                            );
                                        else
                                            itemRefs.current.delete(
                                                item.intel_item_id,
                                            );
                                    }}
                                />
                            </li>
                        ))}
                    </ul>
                    {feed.hasNextPage && (
                        <Button
                            variant="secondary"
                            disabled={feed.isFetchingNextPage}
                            onClick={() => void feed.fetchNextPage()}
                        >
                            {feed.isFetchingNextPage ? "正在加载…" : "加载更多"}
                        </Button>
                    )}
                    {feed.isFetchNextPageError && (
                        <div role="alert">
                            下一页加载失败，已显示内容保持可用。
                            {stableErrorCode(feed.error) ===
                            "validation.source" ? (
                                <Button
                                    onClick={() => void refreshFromFirstPage()}
                                >
                                    游标已失效，重新加载首屏
                                </Button>
                            ) : (
                                <Button
                                    onClick={() => void feed.fetchNextPage()}
                                >
                                    重试下一页
                                </Button>
                            )}
                        </div>
                    )}
                </section>
            )}
        </main>
    );
}
