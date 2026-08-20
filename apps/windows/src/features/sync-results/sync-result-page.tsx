import { useInfiniteQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router";

import { useDesktopApi } from "../../app/providers/use-desktop-api";
import { Button } from "../../components/ui/button";
import type {
    SyncResultCountsV1,
    SyncResultItemV1,
    SyncResultPageV1,
    SyncRunOutcomeV1,
    SyncSourceResultV1,
} from "../../lib/desktop-api/desktop-api";
import { desktopApiQueryKey, syncKeys } from "../../lib/query-client";

const PAGE_SIZE = 25;

function outcomeLabel(outcome: SyncRunOutcomeV1) {
    return {
        succeeded_with_results: "同步成功，已获得新结果",
        succeeded_zero_results: "同步完成，本轮没有新候选",
        partially_succeeded: "部分来源成功，已保留确认结果",
        failed: "本轮同步失败",
    }[outcome];
}

function Counts({ counts }: { readonly counts: SyncResultCountsV1 }) {
    return (
        <dl className="sync-result-counts" aria-label="本轮计数">
            <div>
                <dt>新增</dt>
                <dd>{counts.inserted}</dd>
            </div>
            <div>
                <dt>更新</dt>
                <dd>{counts.updated}</dd>
            </div>
            <div>
                <dt>跳过</dt>
                <dd>{counts.skipped}</dd>
            </div>
            <div>
                <dt>失败</dt>
                <dd>{counts.failed}</dd>
            </div>
        </dl>
    );
}

function sameSummary(
    left: SyncResultPageV1["summary"],
    right: SyncResultPageV1["summary"],
) {
    return JSON.stringify(left) === JSON.stringify(right);
}

function collectConsistentPages(pages: readonly SyncResultPageV1[]) {
    const trusted: SyncResultPageV1[] = [];
    const cursors = new Set<string>();
    const itemIds = new Set<string>();
    const first = pages[0];
    let rejected = false;
    for (const page of pages) {
        if (first && !sameSummary(first.summary, page.summary)) {
            rejected = true;
            break;
        }
        if (
            page.items.some((item) => itemIds.has(item.result_item_id)) ||
            (page.next_cursor !== null && cursors.has(page.next_cursor))
        ) {
            rejected = true;
            break;
        }
        for (const item of page.items) itemIds.add(item.result_item_id);
        if (page.next_cursor !== null) cursors.add(page.next_cursor);
        trusted.push(page);
    }
    return { trusted, rejected };
}

function emptySourceCopy(
    outcome: SyncRunOutcomeV1,
    source: SyncSourceResultV1,
) {
    if (["failed", "cancelled", "retry_wait"].includes(source.status)) {
        return "该来源同步失败，没有可显示的成功结果。";
    }
    if (outcome === "partially_succeeded") {
        return "该来源已完成，但本轮没有成功转换的新结果。";
    }
    if (outcome === "failed") {
        return "本轮全部来源失败，没有可显示的成功结果。";
    }
    return "同步已完成，本轮没有可显示的新候选。";
}

function ResultItem({ item }: { readonly item: SyncResultItemV1 }) {
    return (
        <li>
            <strong>{item.original_title}</strong>
            <span>{item.publisher} · RSS/Atom</span>
            <span>{item.disposition === "inserted" ? "新增" : "更新"}</span>
            <span>
                发布时间：{item.published_at ?? "未提供"}；采集时间：
                {item.collected_at}
            </span>
            <span className="provenance-url">{item.original_url}</span>
        </li>
    );
}

export function SyncResultPage() {
    const api = useDesktopApi();
    const apiKey = desktopApiQueryKey(api);
    const { syncRunId = "" } = useParams();
    const validRunId = /^run:[0-9a-f]{24}$/.test(syncRunId);
    const result = useInfiniteQuery({
        queryKey: syncKeys.result(apiKey, syncRunId, null, PAGE_SIZE),
        queryFn: ({ pageParam }) =>
            api.getSyncResult({
                contract_version: 1,
                sync_run_id: syncRunId,
                cursor: pageParam,
                limit: PAGE_SIZE,
            }),
        initialPageParam: null as string | null,
        getNextPageParam: (page, _pages, _lastPageParam, pageParams) =>
            page.next_cursor !== null && !pageParams.includes(page.next_cursor)
                ? page.next_cursor
                : undefined,
        enabled: validRunId,
    });
    const consistency = collectConsistentPages(result.data?.pages ?? []);
    const firstPage = consistency.trusted[0];
    const items = consistency.trusted.flatMap((page) => page.items);

    if (!validRunId) {
        return (
            <main className="sync-result-page">
                <h1>本轮同步结果</h1>
                <div role="alert">本轮结果标识无效，未发起任何读取。</div>
                <Link to="/sources">返回来源</Link>
            </main>
        );
    }

    return (
        <main className="sync-result-page">
            <header>
                <p className="eyebrow">AI SUBSCRIBE · WINDOWS</p>
                <h1>本轮同步结果</h1>
                <p>当前仅 RSS/Atom；结果保存在此 Windows 设备并可离线读取。</p>
                <Link to="/sources">返回来源</Link>
            </header>
            {result.isPending && <p role="status">正在读取本轮结果…</p>}
            {result.isError && !firstPage && (
                <div role="alert">
                    本轮结果暂时不可用，已保留当前页面上下文。
                    <Button type="button" onClick={() => void result.refetch()}>
                        重试
                    </Button>
                </div>
            )}
            {(result.isFetchNextPageError || result.isRefetchError) &&
                firstPage && (
                    <div role="alert">
                        后续结果读取失败；已显示的结果仍保留。
                        <Button
                            type="button"
                            onClick={() =>
                                void (result.isFetchNextPageError
                                    ? result.fetchNextPage()
                                    : result.refetch())
                            }
                        >
                            重试读取
                        </Button>
                    </div>
                )}
            {consistency.rejected && firstPage && (
                <div role="alert">
                    后续结果与本轮身份不一致，已停止拼接并保留已确认内容。
                    <Button type="button" onClick={() => void result.refetch()}>
                        重新读取
                    </Button>
                </div>
            )}
            {firstPage && (
                <>
                    <section aria-labelledby="sync-result-summary-title">
                        <h2 id="sync-result-summary-title">
                            {outcomeLabel(firstPage.summary.outcome)}
                        </h2>
                        <p>完成时间：{firstPage.summary.finished_at}</p>
                        <Counts counts={firstPage.summary.counts} />
                    </section>
                    <section aria-labelledby="sync-source-results-title">
                        <h2 id="sync-source-results-title">来源结果</h2>
                        <ul className="sync-source-results">
                            {firstPage.summary.sources.map((source) => {
                                const sourceItems = items.filter(
                                    (item) =>
                                        item.source_id === source.source_id,
                                );
                                return (
                                    <li
                                        key={source.source_id}
                                        data-source-id={source.source_id}
                                        data-testid={`source-result-${source.source_id}`}
                                    >
                                        <strong>{source.publisher}</strong>
                                        <span>RSS/Atom · {source.status}</span>
                                        <Counts counts={source.counts} />
                                        {source.error_code && (
                                            <p role="alert">
                                                失败范围：{source.error_code}
                                            </p>
                                        )}
                                        <h3>本来源成功转换结果</h3>
                                        {sourceItems.length === 0 ? (
                                            <p>
                                                {emptySourceCopy(
                                                    firstPage.summary.outcome,
                                                    source,
                                                )}
                                            </p>
                                        ) : (
                                            <ul className="sync-result-items">
                                                {sourceItems.map((item) => (
                                                    <ResultItem
                                                        key={
                                                            item.result_item_id
                                                        }
                                                        item={item}
                                                    />
                                                ))}
                                            </ul>
                                        )}
                                    </li>
                                );
                            })}
                        </ul>
                        {result.hasNextPage && (
                            <Button
                                type="button"
                                onClick={() => void result.fetchNextPage()}
                                disabled={result.isFetchingNextPage}
                            >
                                {result.isFetchingNextPage
                                    ? "正在加载…"
                                    : "加载更多"}
                            </Button>
                        )}
                    </section>
                </>
            )}
        </main>
    );
}
