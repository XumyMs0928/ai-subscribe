import { expect, test } from "../support/fixtures/demo-app.fixture";
import { createDemoItem } from "../support/factories/demo-dto.factory";

test.describe("Story 1.7 - 可访问的演示情报与证据详情", () => {
    test("[P0] 键盘选择、进入证据详情并返回原列表项", async ({
        page,
        demoApp,
    }) => {
        const detail = createDemoItem();
        await page.goto("/demo");

        const selected = page.getByRole("button", {
            name: /Agents SDK 发布新的会话追踪能力/,
        });
        await selected.focus();
        await expect(selected).toHaveAttribute("aria-current", "true");
        await selected.press("Enter");

        await expect(
            page.getByRole("heading", { name: detail.title, level: 2 }),
        ).toBeFocused();
        for (const heading of [
            "发生了什么",
            "为什么重要",
            "可能影响",
            "原始事实",
            "规则判断",
            "演示 AI 生成",
            "来源溯源",
        ]) {
            await expect(
                page.getByRole("heading", { name: heading }),
            ).toBeVisible();
        }

        await page.keyboard.press("Escape");
        await expect(selected).toBeFocused();
        expect(await demoApp.externalCalls()).toEqual([]);
    });

    test("[P1] 响应式重排保留选择、筛选与详情", async ({ page }) => {
        await page.setViewportSize({ width: 1280, height: 800 });
        await page.goto("/demo");
        const rust = page.getByRole("button", { name: /Rust 1\.97/ });
        await rust.click();
        await expect(rust).toHaveAttribute("aria-current", "true");

        await page.setViewportSize({ width: 900, height: 800 });
        await expect(
            page.getByRole("region", { name: "演示情报详情" }),
        ).toBeVisible();
        await page.getByRole("button", { name: "返回列表" }).click();
        await expect(rust).toHaveAttribute("aria-current", "true");
        await expect(rust).toBeFocused();
        await expect(page.getByRole("textbox", { name: "搜索" })).toHaveValue(
            "",
        );
    });

    test("[P1] 浅深主题下以等效视口覆盖 100%-200% 布局压缩且无阻断溢出", async ({
        page,
    }) => {
        await page.setViewportSize({ width: 1280, height: 800 });
        for (const colorScheme of ["light", "dark"] as const) {
            await page.emulateMedia({ colorScheme });
            await page.goto("/demo");
            for (const scale of [1, 1.25, 1.5, 1.75, 2]) {
                await page.setViewportSize({
                    width: Math.floor(1280 / scale),
                    height: Math.floor(800 / scale),
                });
                await expect(
                    page.getByRole("button", {
                        name: /Agents SDK 发布新的会话追踪能力/,
                    }),
                ).toBeVisible();
                expect(
                    await page.evaluate(
                        () =>
                            document.documentElement.scrollWidth <=
                            document.documentElement.clientWidth,
                    ),
                ).toBe(true);
            }
        }

        await page.emulateMedia({
            colorScheme: "dark",
            forcedColors: "active",
            reducedMotion: "reduce",
        });
        await page.goto("/demo");
        await expect(
            page.getByRole("button", {
                name: /Agents SDK 发布新的会话追踪能力/,
            }),
        ).toHaveAttribute("aria-current", "true");
    });
});
