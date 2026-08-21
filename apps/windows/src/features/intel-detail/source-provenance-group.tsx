import { useRef, useState } from "react";

import { Button } from "../../components/ui/button";
import type {
    AssociationEvidenceV1,
    IntelProvenanceV1,
} from "../../lib/desktop-api/desktop-api";
import { DesktopTimeoutError } from "../../lib/desktop-api/desktop-api";

export function SourceProvenanceGroup({
    provenance,
    association,
    onOpenOriginal,
}: {
    readonly provenance: readonly IntelProvenanceV1[];
    readonly association: AssociationEvidenceV1;
    readonly onOpenOriginal: (source: IntelProvenanceV1) => Promise<void>;
}) {
    const [expanded, setExpanded] = useState(false);
    const [openingId, setOpeningId] = useState<string | null>(null);
    const [openErrorId, setOpenErrorId] = useState<string | null>(null);
    const [openedId, setOpenedId] = useState<string | null>(null);
    const [indeterminateIds, setIndeterminateIds] = useState<
        ReadonlySet<string>
    >(() => new Set());
    const openingGuard = useRef(false);
    const primary = provenance[0];
    const associated = provenance.slice(1);

    async function open(source: IntelProvenanceV1) {
        if (openingGuard.current || indeterminateIds.has(source.provenance_id))
            return;
        openingGuard.current = true;
        setOpeningId(source.provenance_id);
        setOpenErrorId(null);
        setOpenedId(null);
        try {
            await onOpenOriginal(source);
            setOpenedId(source.provenance_id);
        } catch (error) {
            if (error instanceof DesktopTimeoutError) {
                setIndeterminateIds(
                    (current) => new Set([...current, source.provenance_id]),
                );
            } else {
                setOpenErrorId(source.provenance_id);
            }
        } finally {
            openingGuard.current = false;
            setOpeningId(null);
        }
    }

    return (
        <section aria-labelledby="intel-provenance-heading">
            <h3 id="intel-provenance-heading">来源溯源</h3>
            <ProvenanceRecord
                source={primary}
                opening={openingId === primary.provenance_id}
                anotherOpening={openingId !== null}
                openError={openErrorId === primary.provenance_id}
                opened={openedId === primary.provenance_id}
                indeterminate={indeterminateIds.has(primary.provenance_id)}
                onOpen={() => void open(primary)}
            />
            {associated.length > 0 && (
                <div className="associated-sources">
                    <Button
                        variant="secondary"
                        aria-expanded={expanded}
                        aria-controls="associated-source-records"
                        onClick={() => setExpanded((value) => !value)}
                    >
                        {expanded ? "收起" : "展开"}关联来源（
                        {associated.length}）
                    </Button>
                    {expanded && (
                        <div id="associated-source-records">
                            {associated.map((source) => (
                                <ProvenanceRecord
                                    key={source.provenance_id}
                                    source={source}
                                    opening={openingId === source.provenance_id}
                                    anotherOpening={openingId !== null}
                                    openError={
                                        openErrorId === source.provenance_id
                                    }
                                    opened={openedId === source.provenance_id}
                                    indeterminate={indeterminateIds.has(
                                        source.provenance_id,
                                    )}
                                    onOpen={() => void open(source)}
                                />
                            ))}
                        </div>
                    )}
                </div>
            )}
            {association.relation_type && (
                <p className="association-basis">
                    关联依据：规范化原文地址一致 · {association.relation_type} ·
                    v{association.basis_version}
                </p>
            )}
            {association.status === "incomplete" && (
                <p role="status" className="demo-inline-error">
                    部分关联来源暂时无法可靠读取；主来源与本地事实仍保持可用。
                </p>
            )}
        </section>
    );
}

function ProvenanceRecord({
    source,
    opening,
    anotherOpening,
    openError,
    opened,
    indeterminate,
    onOpen,
}: {
    readonly source: IntelProvenanceV1;
    readonly opening: boolean;
    readonly anotherOpening: boolean;
    readonly openError: boolean;
    readonly opened: boolean;
    readonly indeterminate: boolean;
    readonly onOpen: () => void;
}) {
    const sourceLabel = `${source.publisher}《${source.original_title}》`;
    return (
        <article
            className="provenance-record"
            aria-label={`${sourceLabel}溯源`}
        >
            <dl className="evidence-fields">
                <dt>来源类型</dt>
                <dd>RSS/Atom</dd>
                <dt>证据记录 ID</dt>
                <dd>{source.provenance_id}</dd>
                <dt>发布方</dt>
                <dd>{source.publisher}</dd>
                <dt>作者</dt>
                <dd>{source.author ?? "未提供"}</dd>
                <dt>原始标题</dt>
                <dd>{source.original_title}</dd>
                <dt>原文地址</dt>
                <dd className="provenance-url">{source.display_url}</dd>
                <dt>发布时间</dt>
                <dd>{source.published_at ?? "未提供"}</dd>
                <dt>采集时间</dt>
                <dd>{source.collected_at}</dd>
                <dt>首次发现</dt>
                <dd>{source.first_discovered_at}</dd>
                <dt>最后更新</dt>
                <dd>{source.last_updated_at}</dd>
                <dt>最后已知原文状态</dt>
                <dd>
                    {source.availability_status === "available"
                        ? "采集时可用"
                        : "当前记录不可用"}
                </dd>
            </dl>
            <Button
                disabled={
                    !source.can_open_original || anotherOpening || indeterminate
                }
                onClick={onOpen}
                aria-label={`打开 ${sourceLabel} 的原文（系统浏览器）`}
            >
                {opening ? "正在请求系统浏览器…" : "打开原文"}
            </Button>
            {!source.can_open_original && (
                <p role="status">
                    原文当前不可打开；已保存的元数据和本地摘要仍可核验。
                </p>
            )}
            {openError && (
                <p role="alert" className="demo-inline-error">
                    系统浏览器未能打开该来源。数据未被修改，可稍后重试或返回详情。
                </p>
            )}
            {indeterminate && (
                <p role="alert" className="demo-inline-error">
                    系统浏览器打开状态未知。为避免重复打开，本次详情会话已禁用该来源；可返回列表后重新进入详情。
                </p>
            )}
            {opened && (
                <p role="status">
                    已请求系统浏览器打开；这不代表站点当前可达。
                </p>
            )}
        </article>
    );
}
