import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { Link, useNavigate } from "react-router";

import { useDesktopApi } from "../../app/providers/use-desktop-api";
import { Button } from "../../components/ui/button";
import type {
    SaveSetupStepInputV1,
    SetupProgressV1,
    SetupStepIdV1,
} from "../../lib/desktop-api/desktop-api";
import { desktopApiQueryKey, setupKeys } from "../../lib/query-client";
import { DeviceScopeNotice } from "./device-scope-notice";

const STEP_LABELS: Record<SetupStepIdV1, string> = {
    tracks: "选择关注赛道",
    source_examples: "查看来源示例",
    refresh_cadence: "选择刷新节奏",
    ai_data_disclosure: "查看 AI 数据说明",
};

function savedValues(progress: SetupProgressV1, step: SetupStepIdV1) {
    switch (step) {
        case "tracks":
            return [...progress.saved_config.track_ids];
        case "source_examples":
            return [...progress.saved_config.source_example_ids];
        case "refresh_cadence":
            return progress.saved_config.refresh_cadence
                ? [progress.saved_config.refresh_cadence]
                : [];
        case "ai_data_disclosure":
            return progress.saved_config.ai_data_disclosure_acknowledged
                ? ["acknowledged"]
                : [];
    }
}

export function ProgressiveSetupGuide() {
    const api = useDesktopApi();
    const apiKey = desktopApiQueryKey(api);
    const queryClient = useQueryClient();
    const navigate = useNavigate();
    const mounted = useRef(false);
    const lastIntent = useRef<{
        signature: string;
        input: SaveSetupStepInputV1;
    } | null>(null);
    const [draftState, setDraftState] = useState<{
        step: SetupStepIdV1 | null;
        values: string[];
    }>({ step: null, values: [] });
    const progress = useQuery({
        queryKey: setupKeys.progress(apiKey),
        queryFn: () => api.setupProgress(),
    });
    const step = progress.data?.next_step_id ?? null;

    const draft =
        progress.data && step && draftState.step !== step
            ? savedValues(progress.data, step)
            : draftState.values;
    const persisted =
        progress.data && step ? savedValues(progress.data, step) : [];
    const isDirty =
        progress.data !== undefined &&
        step !== null &&
        (draft.length !== persisted.length ||
            draft.some((value, index) => value !== persisted[index]));

    const save = useMutation({
        mutationFn: (input: SaveSetupStepInputV1) => api.saveSetupStep(input),
        onMutate: async () => {
            await queryClient.cancelQueries({
                queryKey: setupKeys.progress(apiKey),
            });
        },
        onSuccess: (next, input) => {
            lastIntent.current = null;
            queryClient.setQueryData(setupKeys.progress(apiKey), next);
            if (
                mounted.current &&
                (input.action === "later" || input.action === "skip")
            ) {
                void navigate("/");
            }
        },
        onError: () => {
            void queryClient.invalidateQueries({
                queryKey: setupKeys.progress(apiKey),
            });
        },
    });

    useEffect(() => {
        mounted.current = true;
        return () => {
            mounted.current = false;
        };
    }, []);

    useEffect(() => {
        const protectNavigation = isDirty || save.isPending;
        const beforeUnload = (event: BeforeUnloadEvent) => {
            if (!protectNavigation) return;
            event.preventDefault();
            event.returnValue = "";
        };
        const interceptLink = (event: MouseEvent) => {
            const target = event.target;
            const link =
                target instanceof Element ? target.closest("a[href]") : null;
            if (!link || !protectNavigation) return;
            if (
                save.isPending ||
                !window.confirm("配置引导中的选择尚未保存，确认离开吗？")
            ) {
                event.preventDefault();
                event.stopPropagation();
                return;
            }
        };
        const interceptHistory = () => {
            if (!protectNavigation) return;
            if (
                save.isPending ||
                !window.confirm("配置引导中的选择尚未保存，确认离开吗？")
            ) {
                window.history.go(1);
                return;
            }
        };
        window.addEventListener("beforeunload", beforeUnload);
        document.addEventListener("click", interceptLink, true);
        window.addEventListener("popstate", interceptHistory);
        return () => {
            window.removeEventListener("beforeunload", beforeUnload);
            document.removeEventListener("click", interceptLink, true);
            window.removeEventListener("popstate", interceptHistory);
        };
    }, [isDirty, save.isPending]);

    function submit(action: SaveSetupStepInputV1["action"]) {
        if (!progress.data || !step || save.isPending) return;
        const selectedValues = action === "save" ? draft : [];
        const signature = JSON.stringify([
            progress.data.revision,
            progress.data.configuration_revision,
            step,
            action,
            selectedValues,
        ]);
        let intent = lastIntent.current;
        if (intent?.signature !== signature) {
            intent = {
                signature,
                input: {
                    contract_version: 1,
                    step_id: step,
                    action,
                    selected_values: selectedValues,
                    expected_revision: progress.data.revision,
                    expected_configuration_revision:
                        progress.data.configuration_revision,
                    idempotency_key: `setup:${crypto.randomUUID()}`,
                },
            };
            lastIntent.current = intent;
        }
        save.mutate(intent.input);
    }

    return (
        <main className="settings-page setup-guide-page">
            <header>
                <p className="eyebrow">可跳过 · 可稍后恢复</p>
                <h1>配置引导</h1>
                <p>先体验演示情报，再按自己的节奏完成当前设备配置。</p>
            </header>
            <DeviceScopeNotice />
            {progress.isPending && <p role="status">正在恢复配置进度…</p>}
            {progress.isError && (
                <div role="alert" className="setup-error">
                    无法读取配置进度，主情报流仍可使用。
                    <Button onClick={() => void progress.refetch()}>
                        重试
                    </Button>
                </div>
            )}
            {progress.data && !step && (
                <section className="setup-panel">
                    <h2>配置引导已完成</h2>
                    <p>
                        {progress.data.overall_status === "completed"
                            ? "已保存的选择保存在当前设备。"
                            : "你可以随时返回继续配置当前设备。"}
                    </p>
                    <Link to="/">返回主情报流</Link>
                </section>
            )}
            {progress.data && step && (
                <section className="setup-panel" aria-busy={save.isPending}>
                    <p className="setup-progress-label">
                        步骤{" "}
                        {progress.data.steps.findIndex(
                            (item) => item.step_id === step,
                        ) + 1}{" "}
                        / {progress.data.steps.length} · {STEP_LABELS[step]}
                    </p>
                    <StepOptions
                        progress={progress.data}
                        step={step}
                        values={draft}
                        onChange={(values) => setDraftState({ step, values })}
                    />
                    {save.isError && (
                        <p role="alert" className="setup-error">
                            保存失败，当前选择仍保留。请重试。
                        </p>
                    )}
                    <div className="setup-actions">
                        <Button
                            id="setup-save"
                            disabled={draft.length === 0 || save.isPending}
                            onClick={() => submit("save")}
                        >
                            保存并继续
                        </Button>
                        <Button
                            id="setup-skip"
                            variant="secondary"
                            disabled={save.isPending}
                            onClick={() => submit("skip")}
                        >
                            跳过此步
                        </Button>
                        <Button
                            id="setup-later"
                            variant="secondary"
                            disabled={save.isPending}
                            onClick={() => submit("later")}
                        >
                            稍后继续
                        </Button>
                        <Link
                            id="setup-return-settings"
                            to="/settings"
                            aria-disabled={save.isPending || undefined}
                            state={{ restoreFocusId: "setup-guide-entry" }}
                        >
                            返回设置
                        </Link>
                    </div>
                </section>
            )}
        </main>
    );
}

function StepOptions({
    progress,
    step,
    values,
    onChange,
}: {
    progress: SetupProgressV1;
    step: SetupStepIdV1;
    values: string[];
    onChange: (values: string[]) => void;
}) {
    if (step === "ai_data_disclosure") {
        return (
            <label className="setup-option">
                <input
                    type="checkbox"
                    checked={values.includes("acknowledged")}
                    onChange={(event) =>
                        onChange(event.target.checked ? ["acknowledged"] : [])
                    }
                />
                我了解示例 AI 内容为固定演示，不会发送数据给 AI 服务。
            </label>
        );
    }
    const options =
        step === "tracks"
            ? progress.defaults.tracks
            : step === "source_examples"
              ? progress.defaults.source_examples
              : progress.defaults.refresh_cadences;
    const single = step === "refresh_cadence";
    return (
        <fieldset>
            <legend>{STEP_LABELS[step]}</legend>
            {options.map((option) => (
                <label className="setup-option" key={option.id}>
                    <input
                        id={`setup-option-${option.id}`}
                        type={single ? "radio" : "checkbox"}
                        name={single ? step : undefined}
                        checked={values.includes(option.id)}
                        onChange={(event) =>
                            onChange(
                                single
                                    ? [option.id]
                                    : event.target.checked
                                      ? [...values, option.id]
                                      : values.filter(
                                            (value) => value !== option.id,
                                        ),
                            )
                        }
                    />
                    <span>
                        {option.label}
                        {option.is_demo ? " · 示例/演示" : ""}
                    </span>
                </label>
            ))}
        </fieldset>
    );
}
