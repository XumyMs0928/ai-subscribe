import { expect, test } from "../support/fixtures/demo-app.fixture";
import {
    createDemoCatalog,
    createDemoItem,
} from "../support/factories/demo-dto.factory";

const rustItem = createDemoItem({
    id: "demo:rust-197-001",
    publisher: "Rust Project",
    title: "Rust 1.97 提升工具链诊断体验",
    track: "开发工具",
    summary: "固定演示样本：说明版本变化、影响范围与原始来源。",
    original_url: "https://www.rust-lang.org/",
    published_at: "2026-06-20T10:00:00Z",
    collected_at: "2026-06-20T10:30:00Z",
});

const openAiItem = createDemoItem();

test.describe("Story 1.6 - 安全隔离的演示情报", () => {
    test("无需注册、Key 或通知授权即可看到三条固定演示数据", async ({
        page,
    }) => {
        await test.step("Given 启动使用项目内固定数据的 Windows 演示页", async () => {
            await page.goto("/");
        });

        await test.step("Then 页面直接展示三条情报，不出现访问门槛", async () => {
            const list = page.locator("#demo-intelligence-list");
            await expect(list).toBeVisible();
            await expect(list.getByRole("button")).toHaveCount(3);
            await expect(
                page.getByText(/注册|API Key|通知授权|开启通知/),
            ).toHaveCount(0);
        });
    });

    test("列表每一项及详情均以文字标明演示数据", async ({ page }) => {
        await test.step("Given 演示情报已经加载", async () => {
            await page.goto("/");
            await expect(
                page.locator("#demo-intelligence-list").getByRole("button"),
            ).toHaveCount(3);
        });

        await test.step("Then 每个列表项都有非颜色的演示标签", async () => {
            const list = page.locator("#demo-intelligence-list");
            await expect(list.getByRole("button")).toHaveCount(3);
            await expect(
                list.getByText("演示数据", { exact: true }),
            ).toHaveCount(3);
        });

        await test.step("And 当前详情也有相同的文字标签", async () => {
            const detail = page.getByRole("region", { name: "演示情报详情" });
            await expect(
                detail.getByText("演示数据", { exact: true }),
            ).toBeVisible();
        });
    });

    test("点击 Rust 条目打开对应详情", async ({ page, demoApp }) => {
        await test.step("Given 演示列表包含 Rust 1.97 条目", async () => {
            await page.goto("/");
            await expect(
                page.getByRole("button", { name: /Rust 1\.97/ }),
            ).toBeVisible();
            await demoApp.setResponse("demo_detail_v1", rustItem);
        });

        await test.step("When 用户点击 Rust 条目", async () => {
            await page.getByRole("button", { name: /Rust 1\.97/ }).click();
        });

        await test.step("Then 详情显示 Rust 条目的标题和发布方", async () => {
            const detail = page.getByRole("region", { name: "演示情报详情" });
            await expect(
                detail.getByRole("heading", { name: /Rust 1\.97/ }),
            ).toBeVisible();
            await expect(
                detail.getByText("Rust Project", { exact: true }),
            ).toBeVisible();
            await expect
                .poll(() => demoApp.invokeCalls())
                .toContainEqual({
                    command: "demo_detail_v1",
                    args: { id: "demo:rust-197-001" },
                });
        });
    });

    test("提交搜索并选择赛道时调用桌面 mock 并展示对应结果", async ({
        page,
        demoApp,
    }) => {
        await test.step("Given 演示目录已经加载", async () => {
            await page.goto("/");
            await expect(
                page.locator("#demo-intelligence-list").getByRole("button"),
            ).toHaveCount(3);
        });

        await test.step("When 用户提交 Rust 搜索", async () => {
            await demoApp.setResponse(
                "demo_search_v1",
                createDemoCatalog({ items: [rustItem] }),
            );
            await page.getByRole("textbox", { name: "搜索" }).fill("Rust");
            await page
                .getByRole("button", { name: "搜索", exact: true })
                .click();
        });

        await test.step("Then mock 收到搜索参数且只显示 Rust 结果", async () => {
            await expect
                .poll(() => demoApp.invokeCalls())
                .toContainEqual({
                    command: "demo_search_v1",
                    args: { query: "Rust", track: null },
                });
            const list = page.locator("#demo-intelligence-list");
            await expect(list.getByRole("button")).toHaveCount(1);
            await expect(
                list.getByRole("button", { name: /Rust 1\.97/ }),
            ).toBeVisible();
        });

        await test.step("When 用户清空搜索并选择 AI Agent 赛道", async () => {
            await page.getByRole("textbox", { name: "搜索" }).fill("");
            await page
                .getByRole("button", { name: "搜索", exact: true })
                .click();
            await expect(
                page.getByRole("option", { name: "AI Agent" }),
            ).toBeAttached();
            await demoApp.setResponse(
                "demo_search_v1",
                createDemoCatalog({ items: [openAiItem] }),
            );
            await page
                .getByRole("combobox", { name: "赛道" })
                .selectOption("AI Agent");
        });

        await test.step("Then mock 收到赛道参数且只显示该赛道结果", async () => {
            await expect
                .poll(() => demoApp.invokeCalls())
                .toContainEqual({
                    command: "demo_search_v1",
                    args: { query: "", track: "AI Agent" },
                });
            const list = page.locator("#demo-intelligence-list");
            await expect(list.getByRole("button")).toHaveCount(1);
            await expect(
                list.getByRole("button", { name: /OpenAI/ }),
            ).toBeVisible();
        });
    });

    test("无匹配结果时显示空态并可一键清除条件", async ({ page, demoApp }) => {
        await test.step("Given 演示目录已经加载", async () => {
            await page.goto("/");
            await expect(
                page.locator("#demo-intelligence-list").getByRole("button"),
            ).toHaveCount(3);
        });

        await test.step("When 用户提交无匹配的搜索条件", async () => {
            await demoApp.setResponse(
                "demo_search_v1",
                createDemoCatalog({ items: [] }),
            );
            await page
                .getByRole("textbox", { name: "搜索" })
                .fill("不存在的演示条目");
            await page
                .getByRole("button", { name: "搜索", exact: true })
                .click();
        });

        await test.step("Then 页面显示空态且 mock 收到搜索参数", async () => {
            await expect(
                page.getByText("当前搜索或筛选没有演示结果。", {
                    exact: true,
                }),
            ).toBeVisible();
            await expect
                .poll(() => demoApp.invokeCalls())
                .toContainEqual({
                    command: "demo_search_v1",
                    args: { query: "不存在的演示条目", track: null },
                });
        });

        await test.step("When 用户清除条件", async () => {
            await page.getByRole("button", { name: "清除条件" }).click();
        });

        await test.step("Then 固定三条演示数据恢复", async () => {
            await expect(
                page.getByRole("textbox", { name: "搜索" }),
            ).toHaveValue("");
            await expect(
                page.locator("#demo-intelligence-list").getByRole("button"),
            ).toHaveCount(3);
        });
    });

    test("浏览演示情报不会发起网络或通知外部调用", async ({
        page,
        demoApp,
    }) => {
        await test.step("Given 用户打开演示页并查看 Rust 详情", async () => {
            await page.goto("/");
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
        });

        await test.step("Then 网络与通知外部调用记录均为空", async () => {
            await expect.poll(() => demoApp.externalCalls()).toEqual([]);
        });
    });
});
