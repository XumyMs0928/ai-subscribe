import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { expect, test } from "../support/fixtures";
import {
    createIntelFeedItem,
    createIntelFeedPage,
} from "../support/factories/demo-dto.factory";

const itemId = `intel:${"1".repeat(64)}`;
const primaryProvenanceId = "prov:primary:aaaaaaaaaaaaaaaaaaaaaaaa";
const detailFixture = JSON.parse(
    readFileSync(
        resolve(
            process.cwd(),
            "contracts/fixtures/intel-detail/phase1-v1.json",
        ),
        "utf8",
    ),
) as Record<string, unknown>;

test.describe("Story 4.6 evidence detail and original verification", () => {
    test("[P0] opens an in-place evidence detail and sends only stable IDs to original intent", async ({
        page,
        demoApp,
    }) => {
        await page.goto("/");
        const row = page.getByRole("button", {
            name: /AI agent security release/,
        });
        await row.click();

        await expect(
            page.getByRole("heading", {
                name: "Agent runtime security release",
            }),
        ).toBeVisible();
        await expect(page.getByText(/本阶段未启用/)).toBeVisible();
        const associated = page.locator(
            '[aria-controls="associated-source-records"]',
        );
        await expect(associated).toHaveAccessibleName("展开关联来源（1）");
        await associated.click();
        await expect(associated).toHaveAttribute("aria-expanded", "true");
        await expect(page.getByText("Example Security")).toBeVisible();

        const beforeOpen = page.url();
        await page
            .getByRole("button", { name: /打开 Example Engineering.*的原文/ })
            .click();
        await expect(page).toHaveURL(beforeOpen);
        await expect(page.getByText(/已请求系统浏览器打开/)).toBeVisible();

        const open = (await demoApp.invokeCalls()).findLast(
            (call) => call.command === "open_intel_original_v1",
        );
        expect(open?.args).toEqual({
            input: {
                contract_version: 1,
                intel_item_id: itemId,
                provenance_id: primaryProvenanceId,
            },
        });
        expect(JSON.stringify(open?.args)).not.toContain("https://");
        await expect
            .poll(() => demoApp.externalCalls())
            .toEqual([
                {
                    kind: "system_browser",
                    target: `${itemId}|${primaryProvenanceId}`,
                    method: null,
                },
            ]);
    });

    test("[P0] Escape returns to the preserved feed selection and query context", async ({
        page,
    }) => {
        await page.goto("/?stream=high_value&tracks=ai_agents");
        const row = page.getByRole("button", {
            name: /AI agent security release/,
        });
        await row.focus();
        await page.keyboard.press("Enter");
        await expect(page).toHaveURL(
            /\/intel\/intel:[0-9a-f]{64}\?stream=high_value&tracks=ai_agents/,
        );
        await page.keyboard.press("Escape");
        await expect(page).toHaveURL(
            /\/intel\?stream=high_value&tracks=ai_agents/,
        );
        await expect(row).toBeFocused();
    });

    test("[P0] a cold detail deep link does not query the feed or navigate the WebView", async ({
        page,
        demoApp,
    }) => {
        await demoApp.setResponse("query_intel_feed_v1", {
            __intelFeedState: "read",
            initialPage: createIntelFeedPage({
                items: [createIntelFeedItem()],
            }),
        });
        await page.goto(`/intel/${itemId}`);

        await expect(
            page.getByRole("heading", {
                name: "Agent runtime security release",
            }),
        ).toBeVisible();
        const commands = (await demoApp.invokeCalls()).map(
            (call) => call.command,
        );
        expect(commands).toContain("query_intel_evidence_detail_v1");
        expect(commands).not.toContain("query_intel_feed_v1");
        expect(commands).not.toContain("sync_health_v1");
        await expect(page).toHaveURL(`/intel/${itemId}`);
        await expect.poll(() => demoApp.externalCalls()).toEqual([]);
    });

    test("[P1] ordinary candidates expose their real rule disposition and reason", async ({
        page,
    }) => {
        await page.goto("/?stream=ordinary_candidate");
        await page
            .getByRole("button", { name: /Quarterly community note/ })
            .click();

        await expect(
            page.getByRole("heading", { name: "Quarterly community note" }),
        ).toBeVisible();
        await expect(
            page.getByLabel("证据详情").getByText("普通候选", { exact: true }),
        ).toBeVisible();
        await expect(page.getByText(/评分低于当前提醒阈值/)).toBeVisible();
    });

    test("[P0] unknown and malformed detail IDs recover without a blank surface", async ({
        page,
    }) => {
        await page.goto(`/intel/intel:${"f".repeat(64)}`);
        await expect(
            page.getByRole("heading", { name: "该情报已不存在" }),
        ).toBeVisible();
        await expect(page.getByText("not_found.intel_detail")).toBeVisible();

        await page.goto("/intel/not-a-stable-id");
        await expect(page).toHaveURL(/\/intel$/);
        await expect(page.getByRole("heading", { name: "情报" })).toBeVisible();
    });

    test("[P1] narrow Windows layout returns focus only after the list is visible", async ({
        page,
    }) => {
        await page.setViewportSize({ width: 800, height: 720 });
        await page.goto("/");
        const row = page.getByRole("button", {
            name: /AI agent security release/,
        });
        await row.click();
        await expect(
            page.getByRole("heading", {
                name: "Agent runtime security release",
            }),
        ).toBeFocused();
        await page.getByRole("button", { name: "返回列表" }).click();
        await expect(page).toHaveURL(/\/intel$/);
        await expect(row).toBeFocused();
    });

    test("[P1] late detail responses cannot replace the later selected item", async ({
        page,
        demoApp,
    }) => {
        await demoApp.setResponse("query_intel_feed_v1", {
            __intelFeedState: "read",
            initialPage: createIntelFeedPage({
                items: [
                    createIntelFeedItem(),
                    createIntelFeedItem({
                        intel_item_id: `intel:${"2".repeat(64)}`,
                        title: "Quarterly community note",
                        score: 90,
                    }),
                ],
            }),
        });
        await demoApp.setResponse("query_intel_evidence_detail_v1", {
            __intelDetailState: "read",
            initialDetail: detailFixture,
            detailDelayFramesById: { [itemId]: 12 },
        });
        await page.goto("/");
        await page
            .getByRole("button", { name: /AI agent security release/ })
            .click();
        await page
            .getByRole("button", { name: /Quarterly community note/ })
            .click();

        await expect(
            page.getByRole("heading", { name: "Quarterly community note" }),
        ).toBeVisible();
        await expect.poll(() => demoApp.settledDetailIds()).toContain(itemId);
        await expect(
            page.getByRole("heading", { name: "Quarterly community note" }),
        ).toBeVisible();
        await expect(
            page.getByRole("heading", {
                name: "Agent runtime security release",
            }),
        ).toHaveCount(0);
    });
});
