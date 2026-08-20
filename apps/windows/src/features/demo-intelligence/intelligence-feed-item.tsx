import { forwardRef, type KeyboardEvent } from "react";

import type {
    AiStatusV1,
    DemoItemV1,
    ImportanceV1,
} from "../../lib/desktop-api/desktop-api";

interface IntelligenceFeedItemProps {
    readonly item: DemoItemV1;
    readonly selected: boolean;
    readonly tabIndex: number;
    readonly onSelect: () => void;
    readonly onNavigate: (direction: -1 | 1) => void;
    readonly onOpenDetail: () => void;
}

export const IntelligenceFeedItem = forwardRef<
    HTMLButtonElement,
    IntelligenceFeedItemProps
>(function IntelligenceFeedItem(
    { item, selected, tabIndex, onSelect, onNavigate, onOpenDetail },
    ref,
) {
    function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
        if (event.key === "ArrowDown" || event.key.toLowerCase() === "j") {
            event.preventDefault();
            onNavigate(1);
        } else if (event.key === "ArrowUp" || event.key.toLowerCase() === "k") {
            event.preventDefault();
            onNavigate(-1);
        } else if (event.key === "Enter") {
            event.preventDefault();
            onOpenDetail();
        }
    }

    return (
        <button
            ref={ref}
            id={"demo-item-" + item.id}
            type="button"
            className="demo-list-item"
            aria-current={selected ? "true" : undefined}
            tabIndex={tabIndex}
            onClick={() => {
                onSelect();
                onOpenDetail();
            }}
            onFocus={onSelect}
            onKeyDown={handleKeyDown}
        >
            <span className="demo-badge">演示数据</span>
            <strong>{item.title}</strong>
            <span>{item.publisher}</span>
            <span>{item.summary}</span>
            <span className="demo-item-meta">
                {item.track} · 发布 {item.published_at} · 采集{" "}
                {item.collected_at}
            </span>
            <span className="demo-item-meta">
                重要度 {importanceLabel(item.importance ?? "medium")} · AI{" "}
                {aiStatusLabel(item.ai_status ?? "unavailable")}
            </span>
        </button>
    );
});

function importanceLabel(value: ImportanceV1): string {
    return { low: "低", medium: "中", high: "高" }[value];
}

function aiStatusLabel(value: AiStatusV1): string {
    return {
        generated: "已生成",
        waiting: "等待中",
        failed: "失败",
        unavailable: "不可用",
    }[value];
}
