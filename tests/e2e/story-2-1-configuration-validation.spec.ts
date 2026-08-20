import { createConfigurationValidationResult } from "../support/factories/demo-dto.factory";
import { expect, response, test } from "../support/fixtures/demo-app.fixture";

async function openRules(page: Page) {
    await page.goto("/rules");
    await expect(page.getByRole("heading", { name: "关注配置" })).toBeVisible();
}

test.describe("Story 2.1 device-local configuration safety", () => {
    test("[P0] valid configuration saves directly without confirmation", async ({
        page,
        demoApp,
    }) => {
        await openRules(page);
        await page.getByRole("button", { name: "保存配置" }).click();

        await expect(
            page.getByText(/已保存，将在下一轮同步生效/),
        ).toBeVisible();
        await expect(page.getByRole("alertdialog")).toHaveCount(0);
        const calls = await demoApp.invokeCalls();
        expect(
            calls
                .map((call) => call.command)
                .filter((command) => command.includes("configuration")),
        ).toEqual([
            "configuration_v1",
            "validate_configuration_v1",
            "save_configuration_v1",
        ]);
        expect(await demoApp.externalCalls()).toEqual([]);
    });

    test.describe("blocking channel", () => {
        test.use({
            tauriCommandOverrides: {
                validate_configuration_v1: response(
                    createConfigurationValidationResult({
                        blocking_errors: [
                            {
                                field_path: "tracks[0].name",
                                code: "value_out_of_range",
                                message_key:
                                    "configuration.fix.value_out_of_range",
                            },
                        ],
                    }),
                ),
            },
        });

        test("[P0] blocking error prevents write and focuses the field", async ({
            page,
            demoApp,
        }) => {
            await openRules(page);
            await page.getByRole("button", { name: "保存配置" }).click();

            await expect(page.getByRole("alert")).toContainText(
                "请输入允许范围内的值",
            );
            await expect(page.getByLabel("赛道名称 1")).toBeFocused();
            expect(
                (await demoApp.invokeCalls()).map((call) => call.command),
            ).not.toContain("save_configuration_v1");
        });
    });

    test.describe("narrowing channel", () => {
        test.use({
            tauriCommandOverrides: {
                validate_configuration_v1: response(
                    createConfigurationValidationResult({
                        narrowing_risks: [
                            {
                                code: "all_sources_disabled",
                                condition_key:
                                    "configuration.risk.all_sources_disabled.condition",
                                consequence_key:
                                    "configuration.risk.all_sources_disabled.consequence",
                            },
                        ],
                        validation_receipt: {
                            token: "A".repeat(43),
                            normalized_config_hash:
                                "bcd1f22825c821003d536ebee4afe3312740917f54a463d8e1a4fd8b90e8f7fe",
                            validator_version: "attention-configuration-v1",
                        },
                    }),
                ),
            },
        });

        test("[P0] narrowing warning requires explicit one-time confirmation", async ({
            page,
            demoApp,
        }) => {
            await openRules(page);
            await page.getByRole("button", { name: "保存配置" }).click();
            const dialog = page.getByRole("alertdialog");
            await expect(dialog).toContainText("可能造成漏报");
            expect(
                (await demoApp.invokeCalls()).map((call) => call.command),
            ).not.toContain("save_configuration_v1");

            await dialog
                .getByRole("button", { name: "理解风险并保存" })
                .click();
            await expect(
                page.getByText(/已保存，将在下一轮同步生效/),
            ).toBeVisible();
            const saveCall = (await demoApp.invokeCalls()).find(
                (call) => call.command === "save_configuration_v1",
            );
            expect(saveCall?.args).toMatchObject({
                input: {
                    validation_receipt: { token: "A".repeat(43) },
                },
            });
        });
    });

    test("[P1] track edits persist across a device-local reload with zero external calls", async ({
        page,
        demoApp,
    }) => {
        await openRules(page);
        await page.getByLabel("赛道名称 1").fill("AI 工程");
        await page.getByLabel("启用此赛道").first().uncheck();
        await page.getByRole("button", { name: "添加赛道" }).click();
        await page.getByLabel("赛道名称 2").fill("本地推理");
        await page.getByRole("button", { name: "保存配置" }).click();
        await expect(
            page.getByText(/已保存，将在下一轮同步生效/),
        ).toBeVisible();

        const saveCall = (await demoApp.invokeCalls()).find(
            (call) => call.command === "save_configuration_v1",
        );
        expect(saveCall?.args).toMatchObject({
            input: {
                configuration: {
                    tracks: expect.arrayContaining([
                        expect.objectContaining({
                            id: "ai_agents",
                            name: "AI 工程",
                            enabled: false,
                        }),
                        expect.objectContaining({
                            id: "custom_track_2",
                            name: "本地推理",
                            enabled: true,
                        }),
                    ]),
                },
            },
        });
        expect(
            (
                saveCall?.args?.input as {
                    expected_normalized_config_hash?: unknown;
                }
            )?.expected_normalized_config_hash,
        ).toMatch(/^[0-9a-f]{64}$/);

        await page.reload();
        await expect(page.getByLabel("赛道名称 1")).toHaveValue("AI 工程");
        await expect(page.getByLabel("启用此赛道").first()).not.toBeChecked();
        await expect(page.getByLabel("赛道名称 2")).toHaveValue("本地推理");
        expect(await demoApp.externalCalls()).toEqual([]);
    });
});
import type { Page } from "@playwright/test";
