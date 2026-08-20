import { expect, test } from "../support/fixtures/demo-app.fixture";
import { createDesktopApiError } from "../support/factories/demo-dto.factory";

test("[P0] Windows 来源页通过 DesktopApi 保存 HTTPS RSS/Atom 且 WebView 零外联", async ({
    page,
    demoApp,
}) => {
    await page.goto("/sources");
    await expect(
        page.getByRole("heading", { name: "RSS / Atom 来源" }),
    ).toBeVisible();
    await expect(page.getByText(/仅此 Windows 设备/)).toBeVisible();
    await expect(page.getByText("尚未添加 RSS / Atom 来源。")).toBeVisible();

    await page
        .getByLabel("公开 HTTPS Feed 地址")
        .fill("https://example.com/feed.xml");
    await page.getByRole("button", { name: "添加来源" }).click();
    await expect(
        page
            .getByRole("status")
            .filter({ hasText: "已保存 https://example.com/feed.xml" }),
    ).toBeVisible();
    await expect(
        page.getByText("https://example.com/feed.xml", { exact: true }),
    ).toBeVisible();
    const calls = await demoApp.invokeCalls();
    const save = calls.find((call) => call.command === "save_source_v1");
    await page.reload();
    await expect(
        page.getByText("https://example.com/feed.xml", { exact: true }),
    ).toBeVisible();

    expect(calls.some((call) => call.command === "query_sources_v1")).toBe(
        true,
    );
    expect(save?.args?.input).toMatchObject({
        contract_version: 1,
        source_kind: "rss_atom",
        url: "https://example.com/feed.xml",
        expected_configuration_revision: 1,
    });
    expect(await demoApp.externalCalls()).toEqual([]);
    await expect(page.getByText(/GitHub|arXiv|AI 来源/)).toHaveCount(0);
});

test("[P1] 阻断来源保存保留输入且不新增列表记录", async ({ page, demoApp }) => {
    await page.goto("/sources");
    await demoApp.setError(
        "save_source_v1",
        createDesktopApiError({
            code: "validation.source",
            category: "validation",
            message_key: "error.validation",
            retryability: "never",
            details_allowlisted: "",
        }),
    );
    const input = page.getByLabel("公开 HTTPS Feed 地址");
    await input.fill("http://127.0.0.1/feed.xml");
    await page.getByRole("button", { name: "添加来源" }).click();

    await expect(page.getByRole("alert")).toContainText("validation.source");
    await expect(input).toHaveValue("http://127.0.0.1/feed.xml");
    await expect(input).toBeFocused();
    await expect(page.getByText("尚未添加 RSS / Atom 来源。")).toBeVisible();
    expect(await demoApp.externalCalls()).toEqual([]);
});
