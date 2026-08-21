import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import {
    DesktopTimeoutError,
    type IntelEvidenceDetailV1,
} from "../../lib/desktop-api/desktop-api";
import { EvidenceDetailPanel } from "./evidence-detail-panel";

function fixture(): IntelEvidenceDetailV1 {
    return JSON.parse(
        readFileSync(
            resolve(
                process.cwd(),
                "../../contracts/fixtures/intel-detail/phase1-v1.json",
            ),
            "utf8",
        ),
    ) as IntelEvidenceDetailV1;
}

function renderDetail(
    detail = fixture(),
    onOpenOriginal = vi.fn().mockResolvedValue(undefined),
) {
    render(
        <EvidenceDetailPanel
            detail={detail}
            refreshing={false}
            refreshError={false}
            onRefresh={vi.fn()}
            onReturnToList={vi.fn()}
            onOpenOriginal={onOpenOriginal}
        />,
    );
    return { onOpenOriginal };
}

describe("EvidenceDetailPanel", () => {
    test("以判断、事实、AI 与独立溯源展示规范详情", async () => {
        const detail = fixture();
        renderDetail(detail);

        expect(
            screen.getByRole("heading", { name: "发生了什么" }),
        ).toBeVisible();
        expect(screen.getByText(/关注赛道：\s*25 分/)).toBeVisible();
        expect(screen.getByText("命中关注赛道")).toBeVisible();
        expect(screen.getByText("ai_agents")).toBeVisible();
        expect(
            screen.getByText(detail.provenance[0].provenance_id),
        ).toBeVisible();
        expect(screen.getByText(/本阶段未启用/)).toBeVisible();
        const associated = screen.getByRole("button", {
            name: "展开关联来源（1）",
        });
        expect(associated).toHaveAttribute("aria-expanded", "false");

        await userEvent.click(associated);

        expect(associated).toHaveAttribute("aria-expanded", "true");
        expect(screen.getByText("Example Security")).toBeVisible();
        expect(screen.getAllByText("未提供").length).toBeGreaterThan(0);
        expect(screen.getByText(/关联依据：规范化原文地址一致/)).toBeVisible();
    });

    test("只把稳定 provenance 记录交给打开原文意图", async () => {
        const detail = fixture();
        const { onOpenOriginal } = renderDetail(detail);

        await userEvent.click(
            screen.getByRole("button", {
                name: /打开 Example Engineering.*的原文/,
            }),
        );

        expect(onOpenOriginal).toHaveBeenCalledWith(detail.provenance[0]);
        expect(screen.getByText(/已请求系统浏览器打开/)).toBeVisible();
    });

    test("规则证据过期时保留事实和溯源且不展示旧判断", () => {
        const detail = fixture();
        renderDetail({
            ...detail,
            rule_status: "stale",
            rule_issue_code: "rule_evidence.stale",
            rule: null,
        });

        expect(screen.getByText(/规则依据已过期或无法验证/)).toBeVisible();
        expect(
            screen.getAllByText(detail.facts.publisher).length,
        ).toBeGreaterThan(0);
        expect(screen.getByRole("heading", { name: "来源溯源" })).toBeVisible();
        expect(screen.queryByText("命中关注赛道")).not.toBeInTheDocument();
    });

    test("规则不可用时保留事实且明确不展示系统判断", () => {
        const detail = fixture();
        renderDetail({
            ...detail,
            rule_status: "unavailable",
            rule_issue_code: "rule_evidence.unavailable",
            rule: null,
        });

        expect(screen.getByText(/当前规则依据不可用/)).toBeVisible();
        expect(
            screen.getByRole("heading", { name: detail.facts.title }),
        ).toBeVisible();
        expect(screen.queryByText("命中关注赛道")).not.toBeInTheDocument();
    });

    test("原文不可用时禁用对应动作并保留本地证据", async () => {
        const detail = fixture();
        renderDetail(detail);
        await userEvent.click(
            screen.getByRole("button", { name: "展开关联来源（1）" }),
        );
        const unavailable = screen.getByRole("button", {
            name: /打开 Example Security.*的原文/,
        });

        expect(unavailable).toBeDisabled();
        expect(screen.getByText(/原文当前不可打开/)).toBeVisible();
        expect(
            screen.getByRole("heading", { name: detail.facts.title }),
        ).toBeVisible();
    });

    test("长摘要可展开，原文 adapter 失败只显示来源局部错误", async () => {
        const detail = fixture();
        const summary = "证据".repeat(300);
        renderDetail(
            { ...detail, facts: { ...detail.facts, source_summary: summary } },
            vi.fn().mockRejectedValue(new Error("redacted adapter failure")),
        );
        const expansion = screen.getByRole("button", { name: "展开全文摘录" });
        expect(expansion).toHaveAttribute("aria-expanded", "false");
        expect(screen.queryByText(summary)).not.toBeInTheDocument();

        await userEvent.click(expansion);
        expect(screen.getByText(summary)).toBeVisible();
        await userEvent.click(
            screen.getByRole("button", {
                name: /打开 Example Engineering.*的原文/,
            }),
        );

        expect(screen.getByRole("alert")).toHaveTextContent(
            "数据未被修改，可稍后重试",
        );
        expect(
            screen.getByRole("heading", { name: detail.facts.title }),
        ).toBeVisible();
        expect(
            screen.queryByText(/redacted adapter failure/),
        ).not.toBeInTheDocument();
    });

    test("打开超时后保持不确定状态并在当前详情会话禁用重试", async () => {
        renderDetail(
            fixture(),
            vi.fn().mockRejectedValue(new DesktopTimeoutError()),
        );
        const open = screen.getByRole("button", {
            name: /打开 Example Engineering.*的原文/,
        });

        await userEvent.click(open);

        expect(screen.getByRole("alert")).toHaveTextContent(
            "系统浏览器打开状态未知",
        );
        expect(open).toBeDisabled();
    });

    test("一个来源正在打开时同步禁用其他来源，避免并发副作用", async () => {
        const pending = new Promise<void>(() => undefined);
        renderDetail(fixture(), vi.fn().mockReturnValue(pending));
        await userEvent.click(
            screen.getByRole("button", { name: "展开关联来源（1）" }),
        );
        const primary = screen.getByRole("button", {
            name: /打开 Example Engineering.*的原文/,
        });
        const associated = screen.getByRole("button", {
            name: /打开 Example Security.*的原文/,
        });

        await userEvent.click(primary);

        expect(primary).toBeDisabled();
        expect(associated).toBeDisabled();
    });
});
