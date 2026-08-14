import { useContext } from "react";

import type { DesktopApi } from "../../lib/desktop-api/desktop-api";
import { DesktopApiContext } from "./desktop-api-context";

export function useDesktopApi(): DesktopApi {
    const api = useContext(DesktopApiContext);
    if (!api) throw new Error("DesktopApiProvider is required");
    return api;
}
