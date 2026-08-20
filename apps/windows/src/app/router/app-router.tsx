import { useEffect, useState } from "react";
import { Link, Navigate, Route, Routes, useLocation } from "react-router";

import { DemoIntelligence } from "../../features/demo-intelligence/demo-intelligence";
import { ProgressiveSetupGuide } from "../../features/setup-guide/progressive-setup-guide";
import { SettingsRoot } from "../../features/settings/settings-root";
import { ConfigurationEditor } from "../../features/configuration-validation/configuration-editor";
import { SourcesPage } from "../../features/sources/sources-page";
import { SyncResultPage } from "../../features/sync-results/sync-result-page";
import { IntelFeed } from "../../features/intel-feed/intel-feed";

export function AppRouter() {
    const location = useLocation();
    const isFeed = location.pathname === "/" || location.pathname === "/intel";
    const [feedMounted, setFeedMounted] = useState(isFeed);

    useEffect(() => {
        // Keep the feed mounted only after its first real visit so direct deep links stay side-effect free.
        // eslint-disable-next-line react-hooks/set-state-in-effect
        if (isFeed) setFeedMounted(true);
    }, [isFeed]);

    return (
        <div className="app-frame">
            <nav className="app-navigation" aria-label="应用导航">
                <Link aria-current={isFeed ? "page" : undefined} to="/">
                    情报
                </Link>
                <Link
                    id="app-nav-sources"
                    aria-current={
                        location.pathname === "/sources" ? "page" : undefined
                    }
                    to="/sources"
                >
                    来源
                </Link>
                <Link
                    id="app-nav-rules"
                    aria-current={
                        location.pathname === "/rules" ? "page" : undefined
                    }
                    to="/rules"
                >
                    雷达规则
                </Link>
                <Link
                    id="app-nav-settings"
                    aria-current={
                        location.pathname.startsWith("/settings")
                            ? "page"
                            : undefined
                    }
                    to="/settings"
                >
                    设置
                </Link>
            </nav>
            {feedMounted && (
                <div hidden={!isFeed} aria-hidden={!isFeed}>
                    <IntelFeed />
                </div>
            )}
            <Routes>
                <Route path="/" element={null} />
                <Route path="/intel" element={null} />
                <Route path="/demo" element={<DemoIntelligence />} />
                <Route path="/settings" element={<SettingsRoot />} />
                <Route path="/rules" element={<ConfigurationEditor />} />
                <Route path="/sources" element={<SourcesPage />} />
                <Route path="/sync/:syncRunId" element={<SyncResultPage />} />
                <Route
                    path="/settings/setup"
                    element={<ProgressiveSetupGuide />}
                />
                <Route path="*" element={<Navigate replace to="/" />} />
            </Routes>
        </div>
    );
}
