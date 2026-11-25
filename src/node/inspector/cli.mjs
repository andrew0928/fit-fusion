import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import info from "./package.json" with { type: "json" };

const local = fileURLToPath(new URL("../../../runtime/build/rust/release/fit-inspector-bridge", import.meta.url));
const worker = fileURLToPath(new URL("./worker.mjs", import.meta.url));
const result = spawnSync(existsSync(local) ? local : "fit-inspector-bridge",
  ["node", `fit-inspector ${info.version} (Node.js)`, process.execPath, worker, "--", ...process.argv.slice(2)],
  { stdio: "inherit" });
if (result.error) console.error("Build fit-inspector-bridge with src/rust/inspector/Cargo.toml:", result.error.message);
process.exitCode = result.status ?? 1;
