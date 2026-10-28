// Zod (inside the AI SDK) probes `new Function("")` to decide whether it can
// compile validators. The page's CSP forbids it and Trusted Types reports the
// attempt even though Zod catches it: tell Zod not to try. Imported first.
import { config } from "zod/v4/core";

config({ jitless: true });
