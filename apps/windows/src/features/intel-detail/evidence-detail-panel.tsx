import { forwardRef, type KeyboardEvent, useState } from "react";

import { Button } from "../../components/ui/button";
import type {
    IntelEvidenceDetailV1,
    IntelProvenanceV1,
} from "../../lib/desktop-api/desktop-api";
import { SourceProvenanceGroup } from "./source-provenance-group";

const REASON_LABELS: Readonly<Record<string, string>> = {
    "track.matched": "命中关注赛道",
    "track.no_match": "未命中关注赛道",
    score_below_threshold: "评分低于当前提醒阈值",
    source_not_configured: "来源不在当前关注配置中",
    source_disabled: "来源当前已停用",
    track_not_matched: "未匹配当前赛道",
    "source_trust.configured": "来源信任度已纳入评分",
    "source_trust.not_configured": "来源未配置，信任度按零分处理",
    "freshness.collected_at_fallback": "发布时间缺失，按采集时间判断新鲜度",
    "freshness.future_timestamp": "来源时间晚于采集时点",
    "freshness.older_than_30d": "内容发布时间超过 30 天",
    "freshness.within_24h": "内容发布于 24 小时内",
    "freshness.within_7d": "内容发布于 7 天内",
    "freshness.within_30d": "内容发布于 30 天内",
    "technical_impact.no_match": "未识别到明确技术影响",
    "technical_impact.model_capability": "涉及模型能力变化",
    "technical_impact.development_framework": "涉及开发框架或 API 变化",
    "technical_impact.deployment": "涉及部署或运行时变化",
    "technical_impact.cost": "涉及价格或成本变化",
    "technical_impact.security": "涉及安全、漏洞或补丁",
    "technical_impact.technical_selection": "涉及技术选型、发布或迁移",
    "user_rule.include_satisfied": "满足用户包含规则",
    "user_rule.include_not_matched": "未满足用户包含规则",
    trust_outside_range: "来源信任度不在配置范围内",
    include_expression_not_matched: "未满足包含表达式",
    exclude_expression_matched: "命中排除表达式",
    outside_active_window: "当前不在规则生效时间窗内",
    publisher_missing: "发布方信息缺失",
    original_url_invalid: "原文地址无效",
};

const FACTOR_LABELS: Readonly<Record<string, string>> = {
    track: "关注赛道",
    source_trust: "来源信任度",
    freshness: "内容新鲜度",
    technical_impact: "技术影响",
    user_rule: "用户规则",
};

function reasonLabel(code: string) {
    return REASON_LABELS[code] ?? "未识别的规则依据";
}

export const EvidenceDetailPanel = forwardRef<
    HTMLHeadingElement,
    {
        readonly detail: IntelEvidenceDetailV1;
        readonly refreshing: boolean;
        readonly refreshError: boolean;
        readonly onRefresh: () => void;
        readonly onReturnToList: () => void;
        readonly onOpenOriginal: (source: IntelProvenanceV1) => Promise<void>;
    }
>(function EvidenceDetailPanel(
    {
        detail,
        refreshing,
        refreshError,
        onRefresh,
        onReturnToList,
        onOpenOriginal,
    },
    headingRef,
) {
    const [summaryExpanded, setSummaryExpanded] = useState(false);
    const summary = detail.facts.source_summary;
    const summaryNeedsExpansion = summary !== null && [...summary].length > 480;
    const visibleSummary =
        summaryNeedsExpansion && !summaryExpanded
            ? `${[...summary].slice(0, 480).join("")}…`
            : summary;

    function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
        if (event.key === "Escape") {
            event.preventDefault();
            onReturnToList();
        }
    }

    return (
        <article
            className="intel-evidence-detail"
            onKeyDown={handleKeyDown}
            aria-labelledby="intel-detail-title"
            aria-busy={refreshing}
        >
            <p className="detail-source">
                {detail.facts.publisher} · 真实 RSS/Atom
            </p>
            <h2 ref={headingRef} id="intel-detail-title" tabIndex={-1}>
                {detail.facts.title}
            </h2>
            {refreshing && <p role="status">正在刷新；当前详情保持可用。</p>}
            {refreshError && (
                <p role="alert" className="demo-inline-error">
                    刷新失败，继续显示上次可用详情。
                </p>
            )}
            <section aria-labelledby="intel-summary-heading">
                <h3 id="intel-summary-heading">发生了什么</h3>
                <p>{visibleSummary ?? "来源未提供摘要"}</p>
                {summaryNeedsExpansion && (
                    <Button
                        variant="secondary"
                        aria-expanded={summaryExpanded}
                        onClick={() => setSummaryExpanded((value) => !value)}
                    >
                        {summaryExpanded ? "收起" : "展开全文摘录"}
                    </Button>
                )}
            </section>
            <section aria-labelledby="intel-rule-heading">
                <h3 id="intel-rule-heading">为什么重要 / 规则依据</h3>
                {detail.rule_status === "current" && detail.rule ? (
                    <>
                        <dl className="evidence-summary">
                            <dt>重要程度</dt>
                            <dd>{detail.rule.importance}</dd>
                            <dt>评分</dt>
                            <dd>{detail.rule.score}</dd>
                            <dt>流向</dt>
                            <dd>
                                {detail.rule.disposition === "high_value"
                                    ? "高价值"
                                    : "普通候选"}
                            </dd>
                            <dt>命中赛道</dt>
                            <dd>
                                {detail.rule.matched_track_ids.length > 0
                                    ? detail.rule.matched_track_ids.join("、")
                                    : "未命中"}
                            </dd>
                        </dl>
                        <ul>
                            {detail.rule.factors.map((factor) => (
                                <li key={factor.factor}>
                                    {FACTOR_LABELS[factor.factor] ??
                                        "其他评分因子"}
                                    ：{factor.points} 分
                                    {factor.reason_codes.length > 0 && (
                                        <ul>
                                            {factor.reason_codes.map((code) => (
                                                <li key={code}>
                                                    {reasonLabel(code)}
                                                </li>
                                            ))}
                                        </ul>
                                    )}
                                </li>
                            ))}
                            {detail.rule.filter_reasons.map((reason) => (
                                <li key={`filter-${reason.code}`}>
                                    {reasonLabel(reason.code)}
                                    {reason.actual !== null &&
                                        `：当前 ${reason.actual}${
                                            reason.threshold !== null
                                                ? `，阈值 ${reason.threshold}`
                                                : ""
                                        }`}
                                </li>
                            ))}
                        </ul>
                    </>
                ) : (
                    <p role="status">
                        {detail.rule_status === "stale"
                            ? "规则依据已过期或无法验证；未将旧结果冒充当前判断。"
                            : "当前规则依据不可用。"}
                        事实与溯源仍可独立核验。
                    </p>
                )}
            </section>
            <section
                aria-labelledby="intel-ai-heading"
                data-ai-status="unavailable"
            >
                <h3 id="intel-ai-heading">AI 状态</h3>
                <p>本阶段未启用；未向任何外部 AI provider 发送数据。</p>
            </section>
            <section aria-labelledby="intel-facts-heading">
                <h3 id="intel-facts-heading">原始事实</h3>
                <dl className="evidence-fields">
                    <dt>发布方</dt>
                    <dd>{detail.facts.publisher}</dd>
                    <dt>标题</dt>
                    <dd>{detail.facts.title}</dd>
                    <dt>发布时间</dt>
                    <dd>{detail.facts.published_at ?? "未提供"}</dd>
                    <dt>采集时间</dt>
                    <dd>{detail.facts.collected_at}</dd>
                    <dt>内容状态</dt>
                    <dd>仅本地元数据与来源摘要</dd>
                </dl>
            </section>
            <SourceProvenanceGroup
                key={detail.facts.intel_item_id}
                provenance={detail.provenance}
                association={detail.association}
                onOpenOriginal={onOpenOriginal}
            />
            <div className="detail-actions" aria-label="详情操作">
                <Button variant="secondary" onClick={onReturnToList}>
                    返回列表
                </Button>
                <Button
                    variant="secondary"
                    disabled={refreshing}
                    onClick={onRefresh}
                >
                    {refreshing ? "正在刷新…" : "刷新详情"}
                </Button>
            </div>
        </article>
    );
});
