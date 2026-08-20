import type { KeyboardEvent } from "react";

import type { IntelFeedItemV1 } from "../../lib/desktop-api/desktop-api";

export function IntelligenceFeedItem({
    item,
    selected,
    tabIndex,
    onSelect,
    onNavigate,
    onActivate,
    onEscape,
    itemRef,
}: {
    readonly item: IntelFeedItemV1;
    readonly selected: boolean;
    readonly tabIndex: number;
    readonly onSelect: () => void;
    readonly onNavigate: (offset: number) => void;
    readonly onActivate: (intelItemId: string) => void;
    readonly onEscape: () => void;
    readonly itemRef: (node: HTMLButtonElement | null) => void;
}) {
    const effectiveTime = item.published_at ?? item.collected_at;
    const dispositionLabel =
        item.stream_disposition === "high_value" ? "高价值" : "普通候选";
    function onKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
        if (["ArrowDown", "j", "J"].includes(event.key)) {
            event.preventDefault();
            onNavigate(1);
        } else if (["ArrowUp", "k", "K"].includes(event.key)) {
            event.preventDefault();
            onNavigate(-1);
        } else if (event.key === "Enter") {
            event.preventDefault();
            onSelect();
            onActivate(item.intel_item_id);
        } else if (event.key === "Escape") {
            event.preventDefault();
            onEscape();
        }
    }

    return (
        <button
            ref={itemRef}
            type="button"
            className="feed-item"
            aria-pressed={selected}
            tabIndex={tabIndex}
            onClick={onSelect}
            onKeyDown={onKeyDown}
        >
            <span className="feed-status">{dispositionLabel}</span>
            <strong>{item.title}</strong>
            <span>
                {item.publisher} · {effectiveTime}
            </span>
            {item.source_excerpt && <span>{item.source_excerpt}</span>}
            <span>
                重要度：{item.importance} · 得分：{item.score}
            </span>
            <span>赛道：{item.matched_track_ids.join("、") || "未匹配"}</span>
            <span>AI：未启用</span>
        </button>
    );
}
