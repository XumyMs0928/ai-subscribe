import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { useLocation } from "react-router";
import { Link } from "react-router";

import { useDesktopApi } from "../../app/providers/use-desktop-api";
import { desktopApiQueryKey, setupKeys } from "../../lib/query-client";
import { DeviceScopeNotice } from "../setup-guide/device-scope-notice";
import { SetupGuideEntry } from "../setup-guide/setup-guide-entry";

export function SettingsRoot() {
    const api = useDesktopApi();
    const location = useLocation();
    const apiKey = desktopApiQueryKey(api);
    const progress = useQuery({
        queryKey: setupKeys.progress(apiKey),
        queryFn: () => api.setupProgress(),
    });
    useEffect(() => {
        const state = location.state as { restoreFocusId?: unknown } | null;
        if (state?.restoreFocusId !== "setup-guide-entry") return;
        const frame = window.requestAnimationFrame(() => {
            document.getElementById("setup-guide-entry")?.focus();
        });
        return () => window.cancelAnimationFrame(frame);
    }, [location.key, location.state]);

    return (
        <main className="settings-page">
            <header>
                <p className="eyebrow">AI SUBSCRIBE · WINDOWS</p>
                <h1>设置</h1>
            </header>
            {progress.isPending && <p role="status">正在读取配置引导状态…</p>}
            {progress.isError && (
                <div role="alert" className="setup-error">
                    配置状态暂时不可用。
                    <code>
                        {progress.error instanceof Error &&
                        "code" in progress.error &&
                        typeof progress.error.code === "string"
                            ? progress.error.code
                            : "internal.desktop_contract_mismatch"}
                    </code>
                    <button onClick={() => void progress.refetch()}>
                        重试
                    </button>
                </div>
            )}
            <SetupGuideEntry
                status={
                    progress.data?.overall_status ??
                    (progress.isError ? "unavailable" : "loading")
                }
            />
            <section>
                <h2>关注配置</h2>
                <p>管理赛道、关键词、来源偏好和刷新策略。</p>
                <Link to="/rules">打开关注配置</Link>
            </section>
            <section>
                <h2>RSS / Atom 来源</h2>
                <p>添加经过共享核心安全校验的公开 HTTPS Feed。</p>
                <Link to="/sources">管理 RSS / Atom 来源</Link>
            </section>
            <DeviceScopeNotice />
        </main>
    );
}
