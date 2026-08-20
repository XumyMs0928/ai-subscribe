import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter } from "react-router";

import { DesktopApiProvider } from "./app/providers/desktop-api-provider";
import { AppShell } from "./app/shell/app-shell";
import { createTauriDesktopApi } from "./lib/desktop-api/tauri-desktop-api";
import { createAppQueryClient } from "./lib/query-client";
import "./styles/globals.css";

const root = document.getElementById("root");
if (!root) throw new Error("Application root is missing");
const queryClient = createAppQueryClient();

createRoot(root).render(
    <StrictMode>
        <QueryClientProvider client={queryClient}>
            <DesktopApiProvider api={createTauriDesktopApi()}>
                <BrowserRouter>
                    <AppShell />
                </BrowserRouter>
            </DesktopApiProvider>
        </QueryClientProvider>
    </StrictMode>,
);
