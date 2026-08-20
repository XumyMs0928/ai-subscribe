import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import {
    type KeyboardEvent,
    useEffect,
    useMemo,
    useRef,
    useState,
} from "react";
import { flushSync } from "react-dom";

import { useDesktopApi } from "../../app/providers/use-desktop-api";
import { Button } from "../../components/ui/button";
import type { DemoItemV1 } from "../../lib/desktop-api/desktop-api";
import {
    demoIntelligenceKeys,
    desktopApiQueryKey,
} from "../../lib/query-client";
import { EvidenceDetailPanel } from "./evidence-detail-panel";
import { IntelligenceFeedItem } from "./intelligence-feed-item";

const IPC_TIMEOUT_MS = 10_000;
const PAGE_SIZE = 20;

async function withTimeout<T>(promise: Promise<T>): Promise<T> {
    let timeout: ReturnType<typeof setTimeout> | undefined;
    try {
        return await Promise.race([
            promise,
            new Promise<never>((_, reject) => {
                timeout = setTimeout(
                    () => reject(new Error("desktop IPC timeout")),
                    IPC_TIMEOUT_MS,
                );
            }),
        ]);
    } finally {
        if (timeout !== undefined) clearTimeout(timeout);
    }
}

function stableErrorCode(error: unknown): string {
    return error instanceof Error &&
        "code" in error &&
        typeof error.code === "string"
        ? error.code
        : "internal.desktop_contract_mismatch";
}

function uniqueItems(items: readonly DemoItemV1[]): readonly DemoItemV1[] {
    const seen = new Set<string>();
    return items.filter((item) => {
        if (seen.has(item.id)) return false;
        seen.add(item.id);
        return true;
    });
}

export function DemoIntelligence() {
    const desktopApi = useDesktopApi();
    const apiKey = desktopApiQueryKey(desktopApi);
    const [draftQuery, setDraftQuery] = useState("");
    const [query, setQuery] = useState("");
    const [track, setTrack] = useState<string | null>(null);
    const [selectedId, setSelectedId] = useState<string | null>(null);
    const [compactDetailOpen, setCompactDetailOpen] = useState(false);
    const [selectionNotice, setSelectionNotice] = useState<string | null>(null);
    const searchRef = useRef<HTMLInputElement>(null);
    const detailHeadingRef = useRef<HTMLHeadingElement>(null);
    const detailContainerRef = useRef<HTMLElement>(null);
    const itemRefs = useRef(new Map<string, HTMLButtonElement>());
    const pendingDetailFocusRef = useRef(false);

    const health = useQuery({
        queryKey: demoIntelligenceKeys.health(apiKey),
        queryFn: () => withTimeout(desktopApi.health()),
    });
    const bootstrap = useQuery({
        queryKey: demoIntelligenceKeys.bootstrap(apiKey),
        queryFn: () => withTimeout(desktopApi.demoBootstrap()),
    });
    const catalog = useInfiniteQuery({
        queryKey: demoIntelligenceKeys.catalog(apiKey, query, track),
        initialPageParam: null as string | null,
        enabled: bootstrap.isSuccess,
        queryFn: async ({ pageParam }) => {
            if (query) {
                const result = await withTimeout(
                    desktopApi.demoSearch(query, track),
                );
                return { ...result, next_cursor: null };
            }
            return withTimeout(
                track
                    ? desktopApi.demoFilter(track, pageParam, PAGE_SIZE)
                    : desktopApi.demoList(pageParam, PAGE_SIZE),
            );
        },
        getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
    });
    const items = useMemo(
        () =>
            uniqueItems(
                catalog.data?.pages.flatMap((page) => page.items) ?? [],
            ),
        [catalog.data],
    );
    const effectiveSelectedId = items.some((item) => item.id === selectedId)
        ? selectedId
        : (items[0]?.id ?? null);
    const detail = useQuery({
        queryKey: demoIntelligenceKeys.detail(apiKey, effectiveSelectedId),
        queryFn: () =>
            withTimeout(desktopApi.demoDetail(effectiveSelectedId ?? "")),
        enabled: effectiveSelectedId !== null,
    });
    const visibleDetail =
        detail.data?.id === effectiveSelectedId ? detail.data : null;
    const tracks = useMemo(
        () =>
            [
                ...new Set(
                    bootstrap.data?.items.map((item) => item.track) ?? [],
                ),
            ].sort(),
        [bootstrap.data],
    );

    useEffect(() => {
        if (selectedId !== null && selectedId !== effectiveSelectedId) {
            queueMicrotask(() => {
                setSelectionNotice(
                    "刷新后的结果中已无原选中项，已选择第一条可用情报。",
                );
                setSelectedId(effectiveSelectedId);
            });
        }
    }, [effectiveSelectedId, selectedId]);

    useEffect(() => {
        if (detailContainerRef.current)
            detailContainerRef.current.scrollTop = 0;
    }, [effectiveSelectedId]);

    useEffect(() => {
        if (visibleDetail && pendingDetailFocusRef.current) {
            pendingDetailFocusRef.current = false;
            detailHeadingRef.current?.focus();
        }
    }, [compactDetailOpen, visibleDetail]);

    function selectAndFocus(index: number) {
        if (items.length === 0) return;
        const normalized = (index + items.length) % items.length;
        const id = items[normalized].id;
        setSelectionNotice(null);
        setSelectedId(id);
        queueMicrotask(() => itemRefs.current.get(id)?.focus());
    }

    function handleApplicationShortcut(event: KeyboardEvent<HTMLElement>) {
        const target = event.target;
        if (
            !(target instanceof HTMLElement) ||
            target.matches("input, textarea, select, [contenteditable='true']")
        ) {
            return;
        }
        if (event.key === "/") {
            event.preventDefault();
            searchRef.current?.focus();
        }
    }

    function returnToList() {
        pendingDetailFocusRef.current = false;
        flushSync(() => setCompactDetailOpen(false));
        if (effectiveSelectedId) {
            itemRefs.current.get(effectiveSelectedId)?.focus();
        }
    }

    function openDetail() {
        pendingDetailFocusRef.current = true;
        flushSync(() => setCompactDetailOpen(true));
        if (visibleDetail) {
            pendingDetailFocusRef.current = false;
            detailHeadingRef.current?.focus();
        }
    }

    const blockingError =
        bootstrap.isError || (catalog.isError && !catalog.data);
    const blockingErrorValue = bootstrap.error ?? catalog.error;
    const initialLoading =
        bootstrap.isPending || (catalog.isPending && !catalog.data);

    return (
        <main className="app-shell" onKeyDown={handleApplicationShortcut}>
            <header className="shell-header">
                <div>
                    <p className="eyebrow">AI SUBSCRIBE · WINDOWS</p>
                    <h1>演示情报</h1>
                </div>
                <div
                    id="core-health-status"
                    className="core-status"
                    aria-live="polite"
                >
                    {health.data && "共享核心 healthy · contract_version: 1"}
                    {health.isPending && "正在连接共享核心"}
                    {health.isError && (
                        <>
                            共享核心连接失败
                            <Button
                                variant="secondary"
                                onClick={() => void health.refetch()}
                            >
                                重新连接
                            </Button>
                        </>
                    )}
                </div>
            </header>

            <form
                className="demo-toolbar"
                aria-label="演示情报筛选"
                onSubmit={(event) => {
                    event.preventDefault();
                    setQuery(draftQuery.trim());
                }}
            >
                <label>
                    搜索
                    <input
                        ref={searchRef}
                        value={draftQuery}
                        maxLength={128}
                        onChange={(event) => setDraftQuery(event.target.value)}
                        placeholder="标题、发布方或赛道"
                    />
                </label>
                <Button type="submit">搜索</Button>
                <label>
                    赛道
                    <select
                        value={track ?? ""}
                        onChange={(event) =>
                            setTrack(event.target.value || null)
                        }
                    >
                        <option value="">全部赛道</option>
                        {tracks.map((value) => (
                            <option key={value}>{value}</option>
                        ))}
                    </select>
                </label>
            </form>

            {initialLoading && (
                <div role="status" aria-busy="true" className="demo-message">
                    正在加载演示数据…
                </div>
            )}
            {blockingError && (
                <div role="alert" className="demo-message demo-error">
                    <strong>演示数据暂时无法加载</strong>
                    <p>本地读取失败；没有发送网络请求，也没有修改真实数据。</p>
                    <code>{stableErrorCode(blockingErrorValue)}</code>
                    <Button
                        onClick={() =>
                            void (bootstrap.isError
                                ? bootstrap.refetch()
                                : catalog.refetch())
                        }
                    >
                        重试
                    </Button>
                </div>
            )}
            {catalog.data && items.length === 0 && (
                <div className="demo-message">
                    <p>当前搜索或筛选没有演示结果。</p>
                    <p>数据新鲜度：固定离线演示集 · 请清除条件后重试。</p>
                    <Button
                        variant="secondary"
                        onClick={() => {
                            setDraftQuery("");
                            setQuery("");
                            setTrack(null);
                        }}
                    >
                        清除条件
                    </Button>
                </div>
            )}
            {catalog.data && items.length > 0 && (
                <div
                    className={
                        compactDetailOpen
                            ? "demo-workspace compact-detail-open"
                            : "demo-workspace"
                    }
                >
                    <section
                        id="demo-intelligence-list"
                        aria-label="演示情报列表"
                        className="demo-list"
                        aria-busy={catalog.isFetching}
                    >
                        <p className="demo-cache-status">
                            固定离线演示缓存 · 无外部请求
                        </p>
                        {(catalog.isFetching || selectionNotice) && (
                            <p
                                className="demo-inline-status"
                                aria-live="polite"
                            >
                                {selectionNotice ??
                                    "正在刷新演示情报，当前内容保持可用。"}
                            </p>
                        )}
                        {catalog.isError && catalog.data && (
                            <div role="alert" className="demo-inline-error">
                                刷新失败，继续显示上次可用内容。
                                <code>{stableErrorCode(catalog.error)}</code>
                                <Button
                                    variant="secondary"
                                    onClick={() => void catalog.refetch()}
                                >
                                    重试刷新
                                </Button>
                            </div>
                        )}
                        <ul>
                            {items.map((item, index) => (
                                <li key={item.id}>
                                    <IntelligenceFeedItem
                                        ref={(node) => {
                                            if (node)
                                                itemRefs.current.set(
                                                    item.id,
                                                    node,
                                                );
                                            else
                                                itemRefs.current.delete(
                                                    item.id,
                                                );
                                        }}
                                        item={item}
                                        selected={
                                            item.id === effectiveSelectedId
                                        }
                                        tabIndex={
                                            item.id === effectiveSelectedId
                                                ? 0
                                                : -1
                                        }
                                        onSelect={() => {
                                            setSelectionNotice(null);
                                            setSelectedId(item.id);
                                        }}
                                        onNavigate={(direction) =>
                                            selectAndFocus(index + direction)
                                        }
                                        onOpenDetail={openDetail}
                                    />
                                </li>
                            ))}
                        </ul>
                        {catalog.hasNextPage && (
                            <Button
                                variant="secondary"
                                disabled={catalog.isFetchingNextPage}
                                onClick={() => void catalog.fetchNextPage()}
                            >
                                {catalog.isFetchingNextPage
                                    ? "正在加载…"
                                    : "加载更多"}
                            </Button>
                        )}
                    </section>
                    <section
                        ref={detailContainerRef}
                        id="demo-intelligence-detail"
                        aria-label="演示情报详情"
                        className="demo-detail"
                        aria-busy={detail.isFetching}
                    >
                        {visibleDetail && (
                            <EvidenceDetailPanel
                                ref={detailHeadingRef}
                                detail={visibleDetail}
                                onReturnToList={returnToList}
                            />
                        )}
                        {detail.isPending && !visibleDetail && (
                            <div role="status">正在加载所选情报详情…</div>
                        )}
                        {detail.isError && (
                            <div
                                role="alert"
                                className="demo-message demo-error"
                            >
                                <strong>演示详情暂时无法加载</strong>
                                <p>
                                    列表仍可用；未修改数据，也未执行外部调用。
                                </p>
                                <code>{stableErrorCode(detail.error)}</code>
                                <Button onClick={() => void detail.refetch()}>
                                    重试详情
                                </Button>
                            </div>
                        )}
                    </section>
                </div>
            )}
        </main>
    );
}
