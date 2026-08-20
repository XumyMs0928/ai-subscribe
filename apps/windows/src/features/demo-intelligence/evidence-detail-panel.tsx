import { forwardRef, type KeyboardEvent } from "react";

import type { DemoEvidenceDetailV1 } from "../../lib/desktop-api/desktop-api";
import { SourceProvenanceGroup } from "./source-provenance-group";

export const EvidenceDetailPanel = forwardRef<
    HTMLHeadingElement,
    {
        readonly detail: DemoEvidenceDetailV1;
        readonly onReturnToList: () => void;
    }
>(function EvidenceDetailPanel({ detail, onReturnToList }, headingRef) {
    function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
        if (event.key === "Escape") {
            event.preventDefault();
            onReturnToList();
        }
    }

    return (
        <article onKeyDown={handleKeyDown} aria-labelledby="demo-detail-title">
            <span className="demo-badge">演示数据</span>
            <p className="detail-source">{detail.publisher}</p>
            <h2 ref={headingRef} id="demo-detail-title" tabIndex={-1}>
                {detail.title}
            </h2>
            <section aria-labelledby="what-happened-heading">
                <h3 id="what-happened-heading">发生了什么</h3>
                <p>{detail.what_happened}</p>
            </section>
            <section aria-labelledby="why-it-matters-heading">
                <h3 id="why-it-matters-heading">为什么重要</h3>
                <p>{detail.why_it_matters}</p>
            </section>
            <section aria-labelledby="possible-impact-heading">
                <h3 id="possible-impact-heading">可能影响</h3>
                <p>{detail.possible_impact}</p>
            </section>
            <dl className="evidence-summary">
                <dt>重要程度</dt>
                <dd>{detail.importance ?? "medium"}</dd>
                <dt>AI 置信度</dt>
                <dd>{detail.ai_confidence_percent}%</dd>
            </dl>
            <section aria-labelledby="facts-heading">
                <h3 id="facts-heading">原始事实</h3>
                <ul>
                    {detail.facts.map((fact) => (
                        <li key={fact}>{fact}</li>
                    ))}
                </ul>
            </section>
            <section aria-labelledby="rules-heading">
                <h3 id="rules-heading">规则判断</h3>
                <ul>
                    {detail.rule_reasons.map((reason) => (
                        <li key={reason}>{reason}</li>
                    ))}
                </ul>
            </section>
            <AiEvidenceSection detail={detail} />
            <SourceProvenanceGroup provenance={detail.provenance} />
            <div className="detail-actions" aria-label="主要操作">
                <button type="button" onClick={onReturnToList}>
                    返回列表
                </button>
            </div>
        </article>
    );
});

function AiEvidenceSection({
    detail,
}: {
    readonly detail: DemoEvidenceDetailV1;
}) {
    const status = detail.ai_status ?? "unavailable";
    const content = {
        generated: detail.ai_content,
        waiting: "演示 AI 分析正在等待中；当前内容不作为已生成结论展示。",
        failed: "演示 AI 分析失败；原始事实与规则判断仍可独立核验。",
        unavailable: "演示 AI 分析当前不可用；未向任何外部提供商发送数据。",
    }[status];
    const heading = {
        generated: "演示 AI 生成",
        waiting: "演示 AI 等待中",
        failed: "演示 AI 失败",
        unavailable: "演示 AI 不可用",
    }[status];
    return (
        <section aria-labelledby="ai-heading" data-ai-status={status}>
            <h3 id="ai-heading">{heading}</h3>
            <p>{content}</p>
        </section>
    );
}
