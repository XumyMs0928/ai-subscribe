import { test, expect, response } from "../support/fixtures";
import { createSetupProgress } from "../support/factories/demo-dto.factory";

test.describe("Story 1.8 progressive setup", () => {
    test("[P0] fresh users see the useful feed before any optional guide", async ({
        page,
        demoApp,
    }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "情报" })).toBeVisible();
        await expect(page.getByText("AI agent security release")).toBeVisible();
        await expect(
            page.getByRole("link", { name: /演示数据/ }),
        ).toHaveAttribute("href", "/demo");
        await expect(
            page.getByRole("heading", { name: "配置引导" }),
        ).toHaveCount(0);
        await expect.poll(() => demoApp.externalCalls()).toEqual([]);
    });

    test("[P0] settings exposes the fixed setup entry within two navigations", async ({
        page,
    }) => {
        await page.goto("/");
        await page.getByRole("link", { name: "设置" }).click();
        const entry = page.getByRole("link", { name: "配置引导，未开始" });
        await expect(entry).toBeVisible();
        await entry.click();
        await expect(page).toHaveURL(/\/settings\/setup$/);
        await expect(
            page.getByRole("heading", { name: "配置引导" }),
        ).toBeVisible();
        await expect(page.getByText("仅影响此 Windows 设备")).toBeVisible();
    });

    test("[P0] demo defaults are explicit and saving sends one narrow intent", async ({
        page,
        demoApp,
    }) => {
        const afterSave = createSetupProgress({
            revision: 1,
            overall_status: "partially_completed",
            steps: createSetupProgress().steps.map((step) =>
                step.step_id === "tracks"
                    ? { ...step, status: "completed", saved_fields_version: 1 }
                    : step,
            ),
            next_step_id: "source_examples",
            saved_config: {
                ...createSetupProgress().saved_config,
                track_ids: ["ai_agents"],
            },
        });
        await page.goto("/settings/setup");
        await demoApp.setResponse("save_setup_step_v1", afterSave);
        await expect(page.getByText("AI 智能体 · 示例/演示")).toBeVisible();
        await expect(
            page.getByRole("checkbox", { name: /AI 智能体/ }),
        ).toBeChecked();
        await page.getByRole("checkbox", { name: /基础模型/ }).uncheck();
        await page.getByRole("checkbox", { name: /本地模型/ }).uncheck();
        await page.getByRole("button", { name: "保存并继续" }).click();
        await expect(
            page.getByRole("group", { name: "查看来源示例" }),
        ).toBeVisible();
        await expect(
            page.getByText("GitHub Release 示例 · 示例/演示"),
        ).toBeVisible();
        const saveCall = (await demoApp.invokeCalls()).find(
            (call) => call.command === "save_setup_step_v1",
        );
        expect(saveCall?.args?.input).toMatchObject({
            contract_version: 1,
            step_id: "tracks",
            action: "save",
            selected_values: ["ai_agents"],
            expected_revision: 0,
            expected_configuration_revision: 1,
        });
        await expect.poll(() => demoApp.externalCalls()).toEqual([]);
    });

    test("[P1] skip and later return to the preserved feed", async ({
        page,
        demoApp,
    }) => {
        const skipped = createSetupProgress({
            revision: 1,
            overall_status: "skipped",
            steps: createSetupProgress().steps.map((step) =>
                step.step_id === "tracks"
                    ? { ...step, status: "skipped" }
                    : step,
            ),
            next_step_id: "tracks",
        });
        await page.goto("/settings/setup");
        await demoApp.setResponse("save_setup_step_v1", skipped);
        await page.getByRole("button", { name: "跳过此步" }).click();
        await expect(page).toHaveURL(/\/$/);
        await expect(page.getByRole("heading", { name: "情报" })).toBeVisible();

        const later = createSetupProgress({
            revision: 1,
            overall_status: "in_progress",
            steps: createSetupProgress().steps.map((step) =>
                step.step_id === "tracks"
                    ? { ...step, status: "in_progress" }
                    : step,
            ),
            next_step_id: "tracks",
        });
        await page.goto("/settings/setup");
        await demoApp.setResponse("save_setup_step_v1", later);
        await page.getByRole("button", { name: "稍后继续" }).click();
        await expect(page).toHaveURL(/\/$/);
        await expect(page.getByRole("heading", { name: "情报" })).toBeVisible();
        await expect.poll(() => demoApp.externalCalls()).toEqual([]);
    });

    test.describe("persisted partial progress", () => {
        const partial = createSetupProgress({
            revision: 1,
            overall_status: "partially_completed",
            steps: createSetupProgress().steps.map((step) =>
                step.step_id === "tracks"
                    ? { ...step, status: "completed", saved_fields_version: 1 }
                    : step,
            ),
            next_step_id: "source_examples",
            saved_config: {
                ...createSetupProgress().saved_config,
                track_ids: ["ai_agents"],
            },
        });
        test.use({
            tauriCommandOverrides: { setup_progress_v1: response(partial) },
        });

        test("[P1] reload resumes at the first incomplete step", async ({
            page,
        }) => {
            await page.goto("/settings/setup");
            await expect(
                page.getByRole("group", { name: "查看来源示例" }),
            ).toBeVisible();
            await page.reload();
            await expect(
                page.getByRole("group", { name: "查看来源示例" }),
            ).toBeVisible();
            await expect(
                page.getByText("GitHub Release 示例 · 示例/演示"),
            ).toBeVisible();
        });
    });

    test.describe("completed progress", () => {
        const completed = createSetupProgress({
            revision: 4,
            overall_status: "completed",
            steps: createSetupProgress().steps.map((step) => ({
                ...step,
                status: "completed",
                saved_fields_version: 1,
            })),
            next_step_id: null,
        });
        test.use({
            tauriCommandOverrides: {
                setup_progress_v1: response(completed),
            },
        });

        test("[P1] completed steps are not replayed after reload", async ({
            page,
        }) => {
            await page.goto("/settings/setup");
            await expect(
                page.getByRole("heading", { name: "配置引导已完成" }),
            ).toBeVisible();
            await page.reload();
            await expect(
                page.getByRole("heading", { name: "配置引导已完成" }),
            ).toBeVisible();
            await expect(
                page.getByRole("button", { name: "保存并继续" }),
            ).toHaveCount(0);
        });
    });

    test("[P1] setup remains operable across four Windows layout classes", async ({
        page,
    }) => {
        for (const viewport of [
            { width: 1440, height: 900 },
            { width: 1100, height: 760 },
            { width: 800, height: 720 },
            { width: 560, height: 760 },
        ]) {
            await page.setViewportSize(viewport);
            await page.goto("/settings/setup");
            await expect(
                page.getByRole("heading", { name: "配置引导" }),
            ).toBeVisible();
            await expect(
                page.getByRole("button", { name: "稍后继续" }),
            ).toBeVisible();
            expect(
                await page.evaluate(
                    () =>
                        document.documentElement.scrollWidth <=
                        document.documentElement.clientWidth,
                ),
            ).toBe(true);
        }
    });
});
