import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";

import { Button } from "../../components/ui/button";
import { desktopApiQueryKey } from "../../lib/query-client";
import { useDesktopApi } from "../../app/providers/use-desktop-api";

const IPC_TIMEOUT_MS = 10_000;

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

export function DemoIntelligence() {
    const desktopApi = useDesktopApi();
    const apiKey = desktopApiQueryKey(desktopApi);
    const [draftQuery, setDraftQuery] = useState("");
    const [query, setQuery] = useState("");
    const [track, setTrack] = useState<string | null>(null);
    const [selectedId, setSelectedId] = useState<string | null>(null);

    const health = useQuery({
        queryKey: ["health", apiKey],
        queryFn: () => withTimeout(desktopApi.health()),
    });
    const catalog = useQuery({
        queryKey: ["demo-catalog", apiKey, query, track],
        queryFn: () =>
            withTimeout(
                query || track
                    ? desktopApi.demoSearch(query, track)
                    : desktopApi.demoBootstrap(),
            ),
    });
    const effectiveSelectedId = catalog.data?.items.some(
        (item) => item.id === selectedId,
    )
        ? selectedId
        : (catalog.data?.items[0]?.id ?? null);
    const detail = useQuery({
        queryKey: ["demo-detail", apiKey, effectiveSelectedId],
        queryFn: () =>
            withTimeout(desktopApi.demoDetail(effectiveSelectedId ?? "")),
        enabled: effectiveSelectedId !== null,
    });
    const tracks = useMemo(
        () =>
            [
                ...new Set(catalog.data?.items.map((item) => item.track) ?? []),
            ].sort(),
        [catalog.data],
    );

    return (
        <main className="app-shell">
            <header className="shell-header">
                <div>
                    <p className="eyebrow">AI SUBSCRIBE · WINDOWS</p>
                    <h1>演示情报</h1>
                </div>
                <div className="core-status" aria-live="polite">
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

            {catalog.isPending && (
                <div role="status" aria-busy="true" className="demo-message">
                    正在加载演示数据…
                </div>
            )}
            {catalog.isError && (
                <div role="alert" className="demo-message demo-error">
                    <strong>演示数据暂时无法加载</strong>
                    <p>没有发送网络请求，也没有修改真实数据。</p>
                    <code>
                        {catalog.error instanceof Error &&
                        "code" in catalog.error &&
                        typeof catalog.error.code === "string"
                            ? catalog.error.code
                            : "internal.desktop_contract_mismatch"}
                    </code>
                    <Button onClick={() => void catalog.refetch()}>重试</Button>
                </div>
            )}
            {catalog.data && catalog.data.items.length === 0 && (
                <div className="demo-message">
                    <p>当前搜索或筛选没有演示结果。</p>
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
            {catalog.data && catalog.data.items.length > 0 && (
                <div className="demo-workspace">
                    <section
                        id="demo-intelligence-list"
                        aria-label="演示情报列表"
                        className="demo-list"
                    >
                        {catalog.data.items.map((item) => (
                            <button
                                key={item.id}
                                id={`demo-item-${item.id}`}
                                type="button"
                                className="demo-list-item"
                                aria-pressed={item.id === effectiveSelectedId}
                                onClick={() => setSelectedId(item.id)}
                            >
                                <span className="demo-badge">演示数据</span>
                                <strong>{item.title}</strong>
                                <span>
                                    {item.publisher} · {item.track}
                                </span>
                            </button>
                        ))}
                    </section>
                    <section
                        id="demo-intelligence-detail"
                        aria-label="演示情报详情"
                        className="demo-detail"
                        aria-busy={detail.isPending}
                    >
                        {detail.data && (
                            <>
                                <span className="demo-badge">演示数据</span>
                                <h2 id="demo-detail-title">
                                    {detail.data.title}
                                </h2>
                                <p>{detail.data.summary}</p>
                                <dl>
                                    <dt>发布方</dt>
                                    <dd>{detail.data.publisher}</dd>
                                    <dt>赛道</dt>
                                    <dd>{detail.data.track}</dd>
                                    <dt>发布时间</dt>
                                    <dd>{detail.data.published_at}</dd>
                                </dl>
                            </>
                        )}
                        {detail.isError && (
                            <div
                                role="alert"
                                className="demo-message demo-error"
                            >
                                <strong>演示详情暂时无法加载</strong>
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
