import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { Link } from "react-router";

import { useDesktopApi } from "../../app/providers/use-desktop-api";
import { Button } from "../../components/ui/button";
import type {
    AttentionConfigurationV1,
    ConfigurationValidationResultV1,
    DesktopApi,
    SaveConfigurationInputV1,
} from "../../lib/desktop-api/desktop-api";
import { DesktopCommandError } from "../../lib/desktop-api/desktop-api";
import {
    configurationKeys,
    desktopApiQueryKey,
    setupKeys,
    sourceKeys,
} from "../../lib/query-client";

type FormState =
    | "pristine"
    | "dirty"
    | "validating"
    | "blocking_invalid"
    | "narrowing_risk"
    | "saving"
    | "saved"
    | "save_error";

const FIELD_LABELS: Record<string, string> = {
    include_expression: "包含表达式",
    exclude_expression: "排除表达式",
    refresh_interval_minutes: "刷新周期",
    minimum_trust: "最低可信度",
    maximum_trust: "最高可信度",
    alert_threshold: "提醒阈值",
    schedule: "免打扰或提醒频率",
    notification_frequency: "提醒频率上限",
    active_from: "规则生效时间",
    active_until: "规则结束时间",
};

function fieldLabel(path: string) {
    const track = /^tracks\[(\d+)]/.exec(path);
    if (track) return `赛道名称 ${Number(track[1]) + 1}`;
    const source = /^source_preferences\[(\d+)](?:\.(\w+))?/.exec(path);
    if (source)
        return `来源 ${Number(source[1]) + 1}${source[2] === "trust" ? "可信度" : "地址或标识"}`;
    return FIELD_LABELS[path] ?? "配置字段";
}

function correctionFor(code: string) {
    switch (code) {
        case "expression_unparseable":
            return "请检查括号、引号和 AND/OR/NOT 运算符。";
        case "lower_bound_above_upper_bound":
            return "请让下限不高于上限。";
        case "invalid_source_or_unsupported_protocol":
            return "请填写受支持的 RSS、GitHub 或 arXiv 地址或标识。";
        default:
            return "请输入允许范围内的值。";
    }
}

function riskCopy(code: string) {
    return code === "all_sources_disabled"
        ? "所有来源均已停用，下一轮同步不会获取新情报，可能造成漏报。"
        : "当前筛选会排除本机全部高可信候选，下一轮同步可能漏掉重要情报。";
}

function historyStateIndex(state: unknown): number | null {
    if (!state || typeof state !== "object" || !("idx" in state)) return null;
    const index = (state as { idx?: unknown }).idx;
    return typeof index === "number" && Number.isInteger(index) ? index : null;
}

export function ConfigurationEditor() {
    const api = useDesktopApi();
    const apiKey = desktopApiQueryKey(api);
    return (
        <ConfigurationEditorSession key={apiKey} api={api} apiKey={apiKey} />
    );
}

function ConfigurationEditorSession({
    api,
    apiKey,
}: {
    api: DesktopApi;
    apiKey: number;
}) {
    const queryClient = useQueryClient();
    const current = useQuery({
        queryKey: configurationKeys.current(apiKey),
        queryFn: () => api.configuration(),
    });
    const [draft, setDraft] = useState<AttentionConfigurationV1 | null>(null);
    const [state, setState] = useState<FormState>("pristine");
    const [validation, setValidation] =
        useState<ConfigurationValidationResultV1 | null>(null);
    const [saveInput, setSaveInput] = useState<SaveConfigurationInputV1 | null>(
        null,
    );
    const [baseRevision, setBaseRevision] = useState<number | null>(null);
    const [numericText, setNumericText] = useState<Record<string, string>>({});
    const inFlight = useRef(false);
    const saveTrigger = useRef<HTMLButtonElement>(null);
    const riskDialog = useRef<HTMLElement>(null);
    const riskReturn = useRef<HTMLButtonElement>(null);
    const restoreSaveFocus = useRef(false);
    const historyIndex = useRef<number>(
        historyStateIndex(window.history.state as unknown) ?? 0,
    );
    const restoringHistory = useRef(false);

    const effectiveDraft = draft ?? current.data?.configuration ?? null;
    const hasUnsavedChanges = [
        "dirty",
        "blocking_invalid",
        "narrowing_risk",
        "validating",
        "saving",
        "save_error",
    ].includes(state);
    useEffect(() => {
        const warn = (event: BeforeUnloadEvent) => {
            if (!hasUnsavedChanges) return;
            event.preventDefault();
        };
        window.addEventListener("beforeunload", warn);
        return () => window.removeEventListener("beforeunload", warn);
    }, [hasUnsavedChanges]);
    useEffect(() => {
        if (!hasUnsavedChanges) return;
        const interceptLink = (event: MouseEvent) => {
            const target = event.target;
            const link =
                target instanceof Element ? target.closest("a[href]") : null;
            if (
                link &&
                !window.confirm("配置尚未保存，确认离开并放弃这些修改吗？")
            ) {
                event.preventDefault();
                event.stopPropagation();
            }
        };
        const interceptHistory = (event: PopStateEvent) => {
            const nextIndex =
                historyStateIndex(event.state as unknown) ??
                historyIndex.current - 1;
            if (restoringHistory.current) {
                restoringHistory.current = false;
                historyIndex.current = nextIndex;
                return;
            }
            if (window.confirm("配置尚未保存，确认离开并放弃这些修改吗？")) {
                historyIndex.current = nextIndex;
                return;
            }
            restoringHistory.current = true;
            window.history.go(historyIndex.current - nextIndex);
        };
        document.addEventListener("click", interceptLink, true);
        window.addEventListener("popstate", interceptHistory);
        return () => {
            document.removeEventListener("click", interceptLink, true);
            window.removeEventListener("popstate", interceptHistory);
        };
    }, [hasUnsavedChanges]);
    useEffect(() => {
        if (state !== "blocking_invalid") return;
        const path = validation?.blocking_errors[0]?.field_path ?? "";
        const track = /^tracks\[(\d+)]/.exec(path);
        const source = /^source_preferences\[(\d+)](?:\.(\w+))?/.exec(path);
        const fixedIds: Record<string, string> = {
            include_expression: "include-expression",
            exclude_expression: "exclude-expression",
            refresh_interval_minutes: "refresh-minutes",
            minimum_trust: "minimum-trust",
            maximum_trust: "maximum-trust",
            alert_threshold: "alert-threshold",
            schedule: "quiet-start",
            notification_frequency: "notification-cap",
            active_from: "active-from",
            active_until: "active-until",
        };
        const id = track
            ? `track-name-${track[1]}`
            : source
              ? source[2] === "identifier"
                  ? `source-id-${source[1]}`
                  : source[2] === "trust"
                    ? `source-trust-${source[1]}`
                    : `source-kind-${source[1]}`
              : fixedIds[path];
        if (id) document.getElementById(id)?.focus();
    }, [state, validation]);
    useEffect(() => {
        if (state !== "narrowing_risk") return;
        riskReturn.current?.focus();
        const onKeyDown = (event: KeyboardEvent) => {
            if (event.key === "Escape") {
                event.preventDefault();
                setValidation(null);
                setSaveInput(null);
                restoreSaveFocus.current = true;
                setState("dirty");
                return;
            }
            if (event.key !== "Tab" || !riskDialog.current) return;
            const controls = [
                ...riskDialog.current.querySelectorAll<HTMLElement>("button"),
            ];
            if (controls.length === 0) return;
            const first = controls[0];
            const last = controls.at(-1)!;
            if (event.shiftKey && document.activeElement === first) {
                event.preventDefault();
                last.focus();
            } else if (!event.shiftKey && document.activeElement === last) {
                event.preventDefault();
                first.focus();
            }
        };
        document.addEventListener("keydown", onKeyDown);
        return () => document.removeEventListener("keydown", onKeyDown);
    }, [state]);
    useEffect(() => {
        if (
            (state === "dirty" || state === "save_error") &&
            restoreSaveFocus.current
        ) {
            restoreSaveFocus.current = false;
            saveTrigger.current?.focus();
        }
    }, [state]);

    const validate = useMutation({
        mutationFn: (configuration: AttentionConfigurationV1) =>
            api.validateConfiguration({ contract_version: 1, configuration }),
    });
    const save = useMutation({
        mutationFn: (input: SaveConfigurationInputV1) =>
            api.saveConfiguration(input),
        onSuccess: (view) => {
            queryClient.setQueryData(configurationKeys.current(apiKey), view);
            void queryClient.invalidateQueries({
                queryKey: setupKeys.progress(apiKey),
            });
            void queryClient.invalidateQueries({
                queryKey: sourceKeys.root(apiKey),
            });
            setDraft(view.configuration);
            setBaseRevision(view.revision);
            setNumericText({});
            setValidation(null);
            setSaveInput(null);
            setState("saved");
        },
        onError: (failure) => {
            if (
                failure instanceof DesktopCommandError &&
                (failure.code === "validation.stale_validation_receipt" ||
                    failure.code === "conflict.configuration_revision")
            ) {
                setValidation(null);
                setSaveInput(null);
                if (failure.code === "conflict.configuration_revision") {
                    void current.refetch().then(({ data }) => {
                        if (data) setBaseRevision(data.revision);
                    });
                }
            }
            restoreSaveFocus.current = true;
            setState("save_error");
        },
    });

    function update(next: AttentionConfigurationV1) {
        if (baseRevision === null)
            setBaseRevision(current.data?.revision ?? null);
        setDraft(next);
        setValidation(null);
        setSaveInput(null);
        setState("dirty");
    }

    function numberValue(id: string, value: number | null) {
        return numericText[id] ?? (value === null ? "" : String(value));
    }

    function rememberNumber(id: string, text: string) {
        setNumericText((currentText) => ({ ...currentText, [id]: text }));
    }

    async function prepareSave() {
        if (
            !effectiveDraft ||
            !current.data ||
            current.isFetching ||
            inFlight.current
        )
            return;
        inFlight.current = true;
        try {
            if (state === "save_error" && saveInput) {
                setState("saving");
                await save.mutateAsync(saveInput).catch(() => undefined);
                return;
            }
            setState("validating");
            const result = await validate.mutateAsync(effectiveDraft);
            setValidation(result);
            if (result.blocking_errors.length > 0) {
                setState("blocking_invalid");
                return;
            }
            const input: SaveConfigurationInputV1 = {
                contract_version: 1,
                configuration: effectiveDraft,
                expected_revision: baseRevision ?? current.data.revision,
                expected_normalized_config_hash: result.normalized_config_hash,
                idempotency_key: crypto.randomUUID(),
                validation_receipt: result.validation_receipt,
            };
            setSaveInput(input);
            if (result.narrowing_risks.length > 0) {
                setState("narrowing_risk");
                return;
            }
            setState("saving");
            await save.mutateAsync(input);
        } catch {
            setState("save_error");
        } finally {
            inFlight.current = false;
        }
    }

    if (current.isError)
        return (
            <main>
                <div role="alert">
                    配置暂时不可用。
                    <button onClick={() => void current.refetch()}>重试</button>
                </div>
            </main>
        );
    if (current.isPending || effectiveDraft === null)
        return (
            <main>
                <p role="status">正在读取当前设备配置…</p>
            </main>
        );
    const firstBlocking = validation?.blocking_errors[0];
    const blockingFor = (path: string) =>
        validation?.blocking_errors.some((error) =>
            error.field_path.startsWith(path),
        ) ?? false;
    const blockingProps = (path: string) => ({
        "aria-invalid": blockingFor(path) || undefined,
        "aria-describedby": blockingFor(path)
            ? "configuration-blocking-errors"
            : undefined,
    });
    return (
        <main className="settings-page configuration-editor">
            <header>
                <p className="eyebrow">AI SUBSCRIBE · WINDOWS</p>
                <h1>关注配置</h1>
            </header>
            <p>这些规则只影响当前 Windows 设备，可离线保存，不会同步到云端。</p>
            <form
                inert={state === "narrowing_risk"}
                onSubmit={(event) => {
                    event.preventDefault();
                    void prepareSave();
                }}
            >
                <fieldset
                    disabled={state === "validating" || state === "saving"}
                >
                    <legend>赛道与筛选</legend>
                    <div className="configuration-list" aria-label="关注赛道">
                        {effectiveDraft.tracks.map((track, index) => (
                            <div className="configuration-row" key={track.id}>
                                <label htmlFor={`track-name-${index}`}>
                                    赛道名称 {index + 1}
                                </label>
                                <input
                                    data-automation-id={`track-name-${index}`}
                                    id={`track-name-${index}`}
                                    value={track.name}
                                    {...blockingProps(`tracks[${index}]`)}
                                    onChange={(event) => {
                                        const tracks = [
                                            ...effectiveDraft.tracks,
                                        ];
                                        tracks[index] = {
                                            ...track,
                                            name: event.target.value,
                                        };
                                        update({ ...effectiveDraft, tracks });
                                    }}
                                />
                                <label>
                                    <input
                                        type="checkbox"
                                        checked={track.enabled}
                                        onChange={(event) => {
                                            const tracks = [
                                                ...effectiveDraft.tracks,
                                            ];
                                            tracks[index] = {
                                                ...track,
                                                enabled: event.target.checked,
                                            };
                                            update({
                                                ...effectiveDraft,
                                                tracks,
                                            });
                                        }}
                                    />
                                    启用此赛道
                                </label>
                                <Button
                                    type="button"
                                    variant="secondary"
                                    disabled={
                                        effectiveDraft.tracks.length === 1
                                    }
                                    onClick={() =>
                                        update({
                                            ...effectiveDraft,
                                            tracks: effectiveDraft.tracks.filter(
                                                (_, candidate) =>
                                                    candidate !== index,
                                            ),
                                        })
                                    }
                                >
                                    删除赛道
                                </Button>
                            </div>
                        ))}
                        <Button
                            type="button"
                            variant="secondary"
                            disabled={effectiveDraft.tracks.length >= 32}
                            onClick={() => {
                                let sequence = effectiveDraft.tracks.length + 1;
                                let id = `custom_track_${sequence}`;
                                while (
                                    effectiveDraft.tracks.some(
                                        (track) => track.id === id,
                                    )
                                ) {
                                    sequence += 1;
                                    id = `custom_track_${sequence}`;
                                }
                                update({
                                    ...effectiveDraft,
                                    tracks: [
                                        ...effectiveDraft.tracks,
                                        { id, name: "新赛道", enabled: true },
                                    ],
                                });
                            }}
                        >
                            添加赛道
                        </Button>
                    </div>
                    <label htmlFor="include-expression">包含表达式</label>
                    <input
                        id="include-expression"
                        {...blockingProps("include_expression")}
                        value={effectiveDraft.include_expression}
                        onChange={(event) =>
                            update({
                                ...effectiveDraft,
                                include_expression: event.target.value,
                            })
                        }
                    />
                    <label htmlFor="exclude-expression">排除表达式</label>
                    <input
                        id="exclude-expression"
                        {...blockingProps("exclude_expression")}
                        value={effectiveDraft.exclude_expression}
                        onChange={(event) =>
                            update({
                                ...effectiveDraft,
                                exclude_expression: event.target.value,
                            })
                        }
                    />
                </fieldset>
                <fieldset
                    disabled={state === "validating" || state === "saving"}
                >
                    <legend>来源与节奏</legend>
                    {effectiveDraft.source_preferences.map((source, index) => (
                        <div
                            className="configuration-row"
                            key={`source-${index}`}
                        >
                            <label htmlFor={`source-kind-${index}`}>
                                来源类型 {index + 1}
                            </label>
                            <select
                                id={`source-kind-${index}`}
                                {...blockingProps(
                                    `source_preferences[${index}]`,
                                )}
                                value={source.source_kind}
                                onChange={(event) => {
                                    const sources = [
                                        ...effectiveDraft.source_preferences,
                                    ];
                                    sources[index] = {
                                        ...source,
                                        source_kind: event.target
                                            .value as typeof source.source_kind,
                                    };
                                    update({
                                        ...effectiveDraft,
                                        source_preferences: sources,
                                    });
                                }}
                            >
                                <option value="rss">RSS / Atom</option>
                                <option value="github">GitHub</option>
                                <option value="arxiv">arXiv</option>
                            </select>
                            <label htmlFor={`source-id-${index}`}>
                                来源地址或标识
                            </label>
                            <input
                                id={`source-id-${index}`}
                                {...blockingProps(
                                    `source_preferences[${index}].identifier`,
                                )}
                                value={source.identifier}
                                onChange={(event) => {
                                    const sources = [
                                        ...effectiveDraft.source_preferences,
                                    ];
                                    sources[index] = {
                                        ...source,
                                        identifier: event.target.value,
                                    };
                                    update({
                                        ...effectiveDraft,
                                        source_preferences: sources,
                                    });
                                }}
                            />
                            <label htmlFor={`source-trust-${index}`}>
                                来源可信度（0–100）
                            </label>
                            <input
                                id={`source-trust-${index}`}
                                type="number"
                                {...blockingProps(
                                    `source_preferences[${index}].trust`,
                                )}
                                min={0}
                                max={100}
                                value={numberValue(
                                    `source-trust-${index}`,
                                    source.trust,
                                )}
                                onChange={(event) => {
                                    rememberNumber(
                                        `source-trust-${index}`,
                                        event.target.value,
                                    );
                                    const sources = [
                                        ...effectiveDraft.source_preferences,
                                    ];
                                    sources[index] = {
                                        ...source,
                                        trust:
                                            event.target.value === ""
                                                ? 101
                                                : Number(event.target.value),
                                    };
                                    update({
                                        ...effectiveDraft,
                                        source_preferences: sources,
                                    });
                                }}
                            />
                            <label>
                                <input
                                    id={`source-enabled-${index}`}
                                    type="checkbox"
                                    checked={source.enabled}
                                    onChange={(event) => {
                                        const sources = [
                                            ...effectiveDraft.source_preferences,
                                        ];
                                        sources[index] = {
                                            ...source,
                                            enabled: event.target.checked,
                                        };
                                        update({
                                            ...effectiveDraft,
                                            source_preferences: sources,
                                        });
                                    }}
                                />
                                启用此来源
                            </label>
                            <Button
                                type="button"
                                variant="secondary"
                                disabled={
                                    effectiveDraft.source_preferences.length ===
                                    1
                                }
                                onClick={() =>
                                    update({
                                        ...effectiveDraft,
                                        source_preferences:
                                            effectiveDraft.source_preferences.filter(
                                                (_, candidate) =>
                                                    candidate !== index,
                                            ),
                                    })
                                }
                            >
                                删除来源
                            </Button>
                        </div>
                    ))}
                    <Button
                        type="button"
                        variant="secondary"
                        disabled={
                            effectiveDraft.source_preferences.length >= 64
                        }
                        onClick={() =>
                            update({
                                ...effectiveDraft,
                                source_preferences: [
                                    ...effectiveDraft.source_preferences,
                                    {
                                        source_kind: "rss",
                                        identifier: "",
                                        enabled: true,
                                        trust: 50,
                                    },
                                ],
                            })
                        }
                    >
                        添加来源
                    </Button>
                    <label>
                        <input
                            type="checkbox"
                            checked={effectiveDraft.refresh_enabled}
                            onChange={(event) =>
                                update({
                                    ...effectiveDraft,
                                    refresh_enabled: event.target.checked,
                                })
                            }
                        />
                        启用定时刷新；关闭后仅手动刷新
                    </label>
                    <label htmlFor="refresh-minutes">刷新周期（分钟）</label>
                    <input
                        id="refresh-minutes"
                        {...blockingProps("refresh_interval_minutes")}
                        type="number"
                        min={15}
                        max={10080}
                        value={numberValue(
                            "refresh-minutes",
                            effectiveDraft.refresh_interval_minutes,
                        )}
                        onChange={(event) => {
                            rememberNumber(
                                "refresh-minutes",
                                event.target.value,
                            );
                            update({
                                ...effectiveDraft,
                                refresh_interval_minutes:
                                    event.target.value === ""
                                        ? 0
                                        : Number(event.target.value),
                            });
                        }}
                    />
                    <label htmlFor="minimum-trust">最低可信度（0–100）</label>
                    <input
                        id="minimum-trust"
                        {...blockingProps("minimum_trust")}
                        type="number"
                        min={0}
                        max={100}
                        value={numberValue(
                            "minimum-trust",
                            effectiveDraft.minimum_trust,
                        )}
                        onChange={(event) => {
                            rememberNumber("minimum-trust", event.target.value);
                            update({
                                ...effectiveDraft,
                                minimum_trust:
                                    event.target.value === ""
                                        ? 101
                                        : Number(event.target.value),
                            });
                        }}
                    />
                    <label htmlFor="maximum-trust">最高可信度（0–100）</label>
                    <input
                        id="maximum-trust"
                        {...blockingProps("maximum_trust")}
                        type="number"
                        min={0}
                        max={100}
                        value={numberValue(
                            "maximum-trust",
                            effectiveDraft.maximum_trust,
                        )}
                        onChange={(event) => {
                            rememberNumber("maximum-trust", event.target.value);
                            update({
                                ...effectiveDraft,
                                maximum_trust:
                                    event.target.value === ""
                                        ? 101
                                        : Number(event.target.value),
                            });
                        }}
                    />
                    <label htmlFor="alert-threshold">提醒阈值（0–100）</label>
                    <input
                        id="alert-threshold"
                        {...blockingProps("alert_threshold")}
                        type="number"
                        min={0}
                        max={100}
                        value={numberValue(
                            "alert-threshold",
                            effectiveDraft.alert_threshold,
                        )}
                        onChange={(event) => {
                            rememberNumber(
                                "alert-threshold",
                                event.target.value,
                            );
                            update({
                                ...effectiveDraft,
                                alert_threshold:
                                    event.target.value === ""
                                        ? 101
                                        : Number(event.target.value),
                            });
                        }}
                    />
                </fieldset>
                <fieldset
                    disabled={state === "validating" || state === "saving"}
                >
                    <legend>提醒规则（本 Story 仅保存，不发送通知）</legend>
                    <label>
                        <input
                            type="checkbox"
                            checked={effectiveDraft.quiet_hours.enabled}
                            onChange={(event) =>
                                update({
                                    ...effectiveDraft,
                                    quiet_hours: {
                                        ...effectiveDraft.quiet_hours,
                                        enabled: event.target.checked,
                                    },
                                })
                            }
                        />
                        启用免打扰时段
                    </label>
                    <label htmlFor="quiet-start">开始时间</label>
                    <input
                        id="quiet-start"
                        {...blockingProps("schedule")}
                        type="time"
                        value={effectiveDraft.quiet_hours.start}
                        onChange={(event) =>
                            update({
                                ...effectiveDraft,
                                quiet_hours: {
                                    ...effectiveDraft.quiet_hours,
                                    start: event.target.value,
                                },
                            })
                        }
                    />
                    <label htmlFor="quiet-end">结束时间</label>
                    <input
                        id="quiet-end"
                        type="time"
                        value={effectiveDraft.quiet_hours.end}
                        onChange={(event) =>
                            update({
                                ...effectiveDraft,
                                quiet_hours: {
                                    ...effectiveDraft.quiet_hours,
                                    end: event.target.value,
                                },
                            })
                        }
                    />
                    <label>
                        <input
                            type="checkbox"
                            checked={
                                effectiveDraft.notification_frequency.enabled
                            }
                            onChange={(event) =>
                                update({
                                    ...effectiveDraft,
                                    notification_frequency: {
                                        enabled: event.target.checked,
                                        max_per_24h: event.target.checked
                                            ? (effectiveDraft
                                                  .notification_frequency
                                                  .max_per_24h ?? 5)
                                            : null,
                                    },
                                })
                            }
                        />
                        启用 24 小时提醒频率上限
                    </label>
                    <label htmlFor="notification-cap">最多提醒次数</label>
                    <input
                        id="notification-cap"
                        {...blockingProps("notification_frequency")}
                        type="number"
                        min={1}
                        max={100}
                        disabled={
                            !effectiveDraft.notification_frequency.enabled
                        }
                        value={numberValue(
                            "notification-cap",
                            effectiveDraft.notification_frequency.max_per_24h,
                        )}
                        onChange={(event) => {
                            rememberNumber(
                                "notification-cap",
                                event.target.value,
                            );
                            update({
                                ...effectiveDraft,
                                notification_frequency: {
                                    enabled: true,
                                    max_per_24h:
                                        event.target.value === ""
                                            ? 0
                                            : Number(event.target.value),
                                },
                            });
                        }}
                    />
                    <label htmlFor="active-from">
                        规则生效时间（UTC，可留空）
                    </label>
                    <input
                        id="active-from"
                        {...blockingProps("active_from")}
                        value={effectiveDraft.active_from ?? ""}
                        onChange={(event) =>
                            update({
                                ...effectiveDraft,
                                active_from: event.target.value || null,
                            })
                        }
                    />
                    <label htmlFor="active-until">
                        规则结束时间（UTC，可留空）
                    </label>
                    <input
                        id="active-until"
                        {...blockingProps("active_until")}
                        value={effectiveDraft.active_until ?? ""}
                        onChange={(event) =>
                            update({
                                ...effectiveDraft,
                                active_until: event.target.value || null,
                            })
                        }
                    />
                </fieldset>
                {firstBlocking && (
                    <div id="configuration-blocking-errors" role="alert">
                        <p>无法保存，请修正以下字段：</p>
                        <ul>
                            {validation?.blocking_errors.map((error) => (
                                <li key={`${error.field_path}:${error.code}`}>
                                    {fieldLabel(error.field_path)}：
                                    {correctionFor(error.code)}
                                </li>
                            ))}
                        </ul>
                    </div>
                )}
                <Button
                    id="configuration-save"
                    ref={saveTrigger}
                    type="submit"
                    disabled={
                        state === "validating" ||
                        state === "saving" ||
                        current.isFetching
                    }
                >
                    {state === "validating"
                        ? "正在校验…"
                        : state === "saving"
                          ? "正在保存…"
                          : "保存配置"}
                </Button>
                {state === "save_error" && (
                    <p role="alert">
                        保存失败，所有输入仍保留。请重新校验后重试。
                    </p>
                )}
                <Link to="/settings">返回设置</Link>
            </form>
            {state === "narrowing_risk" && validation && saveInput && (
                <section
                    ref={riskDialog}
                    role="alertdialog"
                    aria-modal="true"
                    aria-labelledby="risk-title"
                >
                    <h2 id="risk-title">这项配置可能造成漏报</h2>
                    {validation.narrowing_risks.map((risk) => (
                        <p key={risk.code}>
                            {riskCopy(risk.code)}历史情报不会改写。
                        </p>
                    ))}
                    <Button
                        id="configuration-risk-return"
                        ref={riskReturn}
                        variant="secondary"
                        onClick={() => {
                            setValidation(null);
                            setSaveInput(null);
                            restoreSaveFocus.current = true;
                            setState("dirty");
                        }}
                    >
                        返回修改
                    </Button>
                    <Button
                        id="configuration-risk-confirm"
                        onClick={() => {
                            if (inFlight.current) return;
                            inFlight.current = true;
                            setState("saving");
                            void save
                                .mutateAsync(saveInput)
                                .catch(() => undefined)
                                .finally(() => {
                                    inFlight.current = false;
                                });
                        }}
                    >
                        理解风险并保存
                    </Button>
                </section>
            )}
            <p id="configuration-state" role="status">
                状态：{state}
                {state === "saved"
                    ? "。已保存，将在下一轮同步生效；历史情报未改写。"
                    : ""}
            </p>
        </main>
    );
}
