// Builds the single-page app for the desktop shell (no Nitro server bundle).
// Cross-platform env var for the vite config's MARGINALIA_DESKTOP switch.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const viteBin = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "node_modules",
  "vite",
  "bin",
  "vite.js",
);

const child = spawn(process.execPath, [viteBin, "build"], {
  stdio: "inherit",
  env: { ...process.env, MARGINALIA_DESKTOP: "1" },
});

child.on("exit", (code) => process.exit(code ?? 1));
