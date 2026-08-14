import { mergeTests } from "@playwright/test";

import { test as demoAppTest } from "./demo-app.fixture";

export const test = mergeTests(demoAppTest);

export { expect } from "./demo-app.fixture";
export type {
    DemoAppFixture,
    ExternalCall,
    TauriCommand,
    TauriCommandOverrides,
    TauriInvokeCall,
} from "./demo-app.fixture";
