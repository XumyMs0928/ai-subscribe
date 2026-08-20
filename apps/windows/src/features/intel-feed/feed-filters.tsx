import type { FormEvent } from "react";

import { Button } from "../../components/ui/button";
import type { IntelFeedTimeWindowV1 } from "../../lib/desktop-api/desktop-api";

export function FeedFilters({
    track,
    source,
    timeWindow,
    importance,
    onTrackChange,
    onSourceChange,
    onTimeWindowChange,
    onImportanceChange,
    onApply,
    onReset,
    validationMessage,
}: {
    readonly track: string;
    readonly source: string;
    readonly timeWindow: IntelFeedTimeWindowV1;
    readonly importance: readonly ("low" | "medium" | "high")[];
    readonly onTrackChange: (value: string) => void;
    readonly onSourceChange: (value: string) => void;
    readonly onTimeWindowChange: (value: IntelFeedTimeWindowV1) => void;
    readonly onImportanceChange: (
        value: readonly ("low" | "medium" | "high")[],
    ) => void;
    readonly onApply: () => void;
    readonly onReset: () => void;
    readonly validationMessage: string | null;
}) {
    function submit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        onApply();
    }
    return (
        <form className="feed-filters" aria-label="情报筛选" onSubmit={submit}>
            <label>
                赛道 ID
                <input
                    value={track}
                    aria-invalid={validationMessage !== null}
                    onChange={(event) => onTrackChange(event.target.value)}
                />
            </label>
            <label>
                来源 ID
                <input
                    value={source}
                    aria-invalid={validationMessage !== null}
                    onChange={(event) => onSourceChange(event.target.value)}
                />
            </label>
            <label>
                时间范围
                <select
                    value={timeWindow}
                    onChange={(event) =>
                        onTimeWindowChange(
                            event.target.value as IntelFeedTimeWindowV1,
                        )
                    }
                >
                    <option value="all_time">全部时间</option>
                    <option value="last_24h">最近 24 小时</option>
                    <option value="last_7d">最近 7 天</option>
                    <option value="last_30d">最近 30 天</option>
                </select>
            </label>
            <fieldset>
                <legend>重要度（可多选）</legend>
                {(
                    [
                        ["high", "高"],
                        ["medium", "中"],
                        ["low", "低"],
                    ] as const
                ).map(([value, label]) => (
                    <label key={value}>
                        <input
                            type="checkbox"
                            checked={importance.includes(value)}
                            onChange={(event) =>
                                onImportanceChange(
                                    event.target.checked
                                        ? [...importance, value].sort()
                                        : importance.filter(
                                              (entry) => entry !== value,
                                          ),
                                )
                            }
                        />
                        {label}
                    </label>
                ))}
            </fieldset>
            {validationMessage && (
                <p role="alert" className="demo-inline-error">
                    {validationMessage}
                </p>
            )}
            <Button type="submit">应用筛选</Button>
            <Button type="button" variant="secondary" onClick={onReset}>
                恢复默认
            </Button>
        </form>
    );
}
