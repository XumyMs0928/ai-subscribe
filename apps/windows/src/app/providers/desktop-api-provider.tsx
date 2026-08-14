import type { ReactNode } from "react";

import type { DesktopApi } from "../../lib/desktop-api/desktop-api";
import { DesktopApiContext } from "./desktop-api-context";

export function DesktopApiProvider({
    api,
    children,
}: {
    readonly api: DesktopApi;
    readonly children: ReactNode;
}) {
    return <DesktopApiContext value={api}>{children}</DesktopApiContext>;
}
