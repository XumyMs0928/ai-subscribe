import {
    commandError,
    expect,
    test,
} from "../support/fixtures/demo-app.fixture";
import {
    createDemoCatalog,
    createDemoItem,
    createDesktopApiError,
} from "../support/factories/demo-dto.factory";
import type { Page } from "@playwright/test";

const rustItem = createDemoItem({
    id: "demo:rust-197-001",
    publisher: "Rust Project",
    title: "Rust 1.97 提升工具链诊断体验",
    track: "开发工具",
    summary: "固定演示样本：说明版本变化、影响范围与原始来源。",
    original_url: "https://www.rust-lang.org/",
    provenance: {
        ...createDemoItem().provenance,
        publisher: "Rust Project",
        original_title: "Rust 1.97 diagnostics demo",
        original_url: "https://www.rust-lang.org/",
        published_at: "2026-06-20T10:00:00Z",
        collected_at: "2026-06-20T10:30:00Z",
        first_discovered_at: "2026-06-20T10:30:00Z",
        last_updated_at: "2026-06-20T10:30:00Z",
    },
    published_at: "2026-06-20T10:00:00Z",
    collected_at: "2026-06-20T10:30:00Z",
});

const catalogFixture = createDemoCatalog();
const rustSummary = catalogFixture.items.find(
    (item) => item.id === "demo:rust-197-001",
)!;
const openAiSummary = catalogFixture.items.find(
    (item) => item.id === "demo:openai-agents-sdk-001",
)!;
const privateCanary = "PRIVATE-CANARY-NEVER-RENDER";

async function openDemoCatalog(page: Page) {
    await page.goto("/demo");
    const list = page.getByRole("region", { name: "演示情报列表" });
    await expect(list).toBeVisible();
    return list;
}

test.describe("Story 1.6 - 安全隔离的演示情报", () => {
    test("[P0] 无需注册、Key 或通知授权即可看到三条固定演示数据", async ({
        page,
    }) => {
        await test.step("Given 启动使用项目内固定数据的 Windows 演示页", async () => {
            await page.goto("/demo");
        });

        await test.step("Then 页面直接展示三条情报，不出现访问门槛", async () => {
            const list = page.getByRole("region", { name: "演示情报列表" });
            await expect(list).toBeVisible();
            await expect(list.getByRole("button")).toHaveCount(3);
            await expect(
                list.getByText("演示数据", { exact: true }),
            ).toHaveCount(3);
            await expect(
                page.getByText(/注册|API Key|通知授权|开启通知/),
            ).toHaveCount(0);
        });
    });

    test("[P1] 列表每一项及详情均以文字标明演示数据", async ({ page }) => {
        const list = await openDemoCatalog(page);
        await expect(list.getByRole("button")).toHaveCount(3);
        await expect(list.getByText("演示数据", { exact: true })).toHaveCount(
            3,
        );
        const detail = page.getByRole("region", { name: "演示情报详情" });
        await expect(
            detail.getByText("演示数据", { exact: true }),
        ).toBeVisible();
    });

    test("[P1] 点击 Rust 条目打开对应详情", async ({ page, demoApp }) => {
        await openDemoCatalog(page);
        await expect(
            page.getByRole("button", { name: /Rust 1\.97/ }),
        ).toBeVisible();
        await demoApp.setResponse("demo_detail_v1", rustItem);
        await page.getByRole("button", { name: /Rust 1\.97/ }).click();

        const detail = page.getByRole("region", { name: "演示情报详情" });
        await expect(
            detail.getByRole("heading", { name: /Rust 1\.97/ }),
        ).toBeVisible();
        await expect(
            detail.getByRole("paragraph").filter({ hasText: /^Rust Project$/ }),
        ).toBeVisible();
        await expect
            .poll(() => demoApp.invokeCalls())
            .toContainEqual({
                command: "demo_detail_v1",
                args: { id: "demo:rust-197-001" },
            });
    });

    test("[P1] 提交搜索并选择赛道时调用桌面 mock 并展示对应结果", async ({
        page,
        demoApp,
    }) => {
        const initialList = await openDemoCatalog(page);
        await expect(initialList.getByRole("button")).toHaveCount(3);

        await demoApp.setResponse(
            "demo_search_v1",
            createDemoCatalog({ items: [rustSummary] }),
        );
        await page.getByRole("textbox", { name: "搜索" }).fill("Rust");
        await page.getByRole("button", { name: "搜索", exact: true }).click();
        await expect
            .poll(() => demoApp.invokeCalls())
            .toContainEqual({
                command: "demo_search_v1",
                args: { query: "Rust", track: null },
            });
        let list = page.getByRole("region", { name: "演示情报列表" });
        await expect(
            list.getByRole("button", { name: /Rust 1\.97/ }),
        ).toBeVisible();
        await expect(list.getByText("演示数据", { exact: true })).toHaveCount(
            1,
        );

        await demoApp.setResponse("demo_filter_v1", {
            ...createDemoCatalog({ items: [openAiSummary] }),
            next_cursor: null,
        });
        await page.getByRole("textbox", { name: "搜索" }).fill("");
        await page.getByRole("button", { name: "搜索", exact: true }).click();
        await page
            .getByRole("combobox", { name: "赛道" })
            .selectOption("AI Agent");
        await expect
            .poll(() => demoApp.invokeCalls())
            .toContainEqual({
                command: "demo_filter_v1",
                args: { track: "AI Agent", cursor: null, limit: 20 },
            });
        list = page.getByRole("region", { name: "演示情报列表" });
        await expect(
            list.getByRole("button", { name: /OpenAI/ }),
        ).toBeVisible();
        await expect(list.getByText("演示数据", { exact: true })).toHaveCount(
            1,
        );
    });

    test("[P1] 无匹配结果时显示空态并可一键清除条件", async ({
        page,
        demoApp,
    }) => {
        const initialList = await openDemoCatalog(page);
        await expect(initialList.getByRole("button")).toHaveCount(3);

        await demoApp.setResponse(
            "demo_search_v1",
            createDemoCatalog({ items: [] }),
        );
        await page
            .getByRole("textbox", { name: "搜索" })
            .fill("不存在的演示条目");
        await page.getByRole("button", { name: "搜索", exact: true }).click();
        await expect(
            page.getByText("当前搜索或筛选没有演示结果。", { exact: true }),
        ).toBeVisible();
        await expect
            .poll(() => demoApp.invokeCalls())
            .toContainEqual({
                command: "demo_search_v1",
                args: { query: "不存在的演示条目", track: null },
            });

        await page.getByRole("button", { name: "清除条件" }).click();
        await expect(page.getByRole("textbox", { name: "搜索" })).toHaveValue(
            "",
        );
        await expect(
            page
                .getByRole("region", { name: "演示情报列表" })
                .getByRole("button"),
        ).toHaveCount(3);
    });

    test("[P0] 浏览演示情报不会发起网络或通知外部调用", async ({
        page,
        demoApp,
    }) => {
        await openDemoCatalog(page);
        await expect(
            page.getByRole("button", { name: /Rust 1\.97/ }),
        ).toBeVisible();
        await demoApp.setResponse("demo_detail_v1", rustItem);
        await page.getByRole("button", { name: /Rust 1\.97/ }).click();
        await expect(
            page
                .getByRole("region", { name: "演示情报详情" })
                .getByRole("heading", { name: /Rust 1\.97/ }),
        ).toBeVisible();
        await expect.poll(() => demoApp.externalCalls()).toEqual([]);
    });

    test.describe("目录命令错误恢复", () => {
        test.use({
            tauriCommandOverrides: {
                demo_bootstrap_v1: commandError(
                    createDesktopApiError({
                        code: "internal.unexpected",
                        message_key: "error.internal",
                        retryability: "manual",
                        details_allowlisted: privateCanary,
                        correlation_id: "pw-private-canary-001",
                    }),
                ),
            },
        });

        test("[P0] 命令错误不泄漏私有详情并可重试恢复", async ({
            page,
            demoApp,
        }) => {
            await page.goto("/demo");
            const alert = page.getByRole("alert");
            await expect(alert).toContainText("演示数据暂时无法加载");
            await expect(
                alert.getByText("internal.unexpected", { exact: true }),
            ).toBeVisible();
            await expect(alert).not.toContainText(privateCanary);
            await expect(
                page.getByText(privateCanary, { exact: true }),
            ).toHaveCount(0);

            await demoApp.setResponse("demo_bootstrap_v1", createDemoCatalog());
            await page
                .getByRole("button", { name: "重试", exact: true })
                .click();
            const list = page.getByRole("region", { name: "演示情报列表" });
            await expect(list.getByRole("button")).toHaveCount(3);
            await expect(
                list.getByText("演示数据", { exact: true }),
            ).toHaveCount(3);
            await expect(alert).toHaveCount(0);
        });
    });
});
