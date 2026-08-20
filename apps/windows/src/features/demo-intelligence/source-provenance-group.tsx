import type { DemoProvenanceV1 } from "../../lib/desktop-api/desktop-api";

export function SourceProvenanceGroup({
    provenance,
}: {
    readonly provenance: DemoProvenanceV1;
}) {
    return (
        <section aria-labelledby="demo-provenance-heading">
            <h3 id="demo-provenance-heading">来源溯源</h3>
            <dl className="evidence-fields">
                <dt>来源类型</dt>
                <dd>{provenance.source_kind}</dd>
                <dt>发布方</dt>
                <dd>{provenance.publisher}</dd>
                <dt>作者</dt>
                <dd>{provenance.author ?? "未提供"}</dd>
                <dt>原始标题</dt>
                <dd>{provenance.original_title}</dd>
                <dt>原文地址</dt>
                <dd className="provenance-url">{provenance.original_url}</dd>
                <dt>发布时间</dt>
                <dd>{provenance.published_at ?? "未提供"}</dd>
                <dt>采集时间</dt>
                <dd>{provenance.collected_at}</dd>
                <dt>首次发现</dt>
                <dd>{provenance.first_discovered_at}</dd>
                <dt>最后更新</dt>
                <dd>{provenance.last_updated_at}</dd>
                <dt>原文状态</dt>
                <dd>
                    {provenance.availability_status === "available"
                        ? "可用"
                        : "当前不可用"}
                </dd>
                <dt>确定性关联依据</dt>
                <dd>
                    {provenance.deterministic_association_basis ?? "未提供"}
                </dd>
            </dl>
        </section>
    );
}
