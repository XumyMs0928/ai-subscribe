import { Link } from "react-router";

import type { SetupStepStatusV1 } from "../../lib/desktop-api/desktop-api";

type SetupGuideEntryStatus = SetupStepStatusV1 | "loading" | "unavailable";

const STATUS_LABELS: Record<SetupGuideEntryStatus, string> = {
    not_started: "未开始",
    in_progress: "进行中",
    skipped: "已跳过",
    partially_completed: "部分完成",
    completed: "已完成",
    loading: "正在读取",
    unavailable: "状态暂时不可用",
};

export function SetupGuideEntry({ status }: { status: SetupGuideEntryStatus }) {
    const label = STATUS_LABELS[status];
    return (
        <Link
            id="setup-guide-entry"
            className="setup-entry"
            aria-label={`配置引导，${label}`}
            to="/settings/setup"
        >
            <span>配置引导</span>
            <span>{label}</span>
        </Link>
    );
}
