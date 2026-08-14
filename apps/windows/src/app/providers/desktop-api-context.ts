import { createContext } from "react";

import type { DesktopApi } from "../../lib/desktop-api/desktop-api";

export const DesktopApiContext = createContext<DesktopApi | null>(null);
