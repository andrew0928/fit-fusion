import { linkSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { createHash } from "node:crypto";
import { isDeepStrictEqual } from "node:util";
import { encode, frames, json } from "./wire.mjs";
import { inventory } from "./inventory.mjs";
import packageInfo from "./package.json" with { type: "json" };
const identity = `fit-inspector ${packageInfo.version} (Node.js)`;
const commands = ["inspect", "raw", "roundtrip", "sdk-check"];
function guide(file) {
  return readFileSync(new URL("../../python/fit_fusion/guides/" + file, import.meta.url), "utf8")
    .replaceAll("{cli}", "node src/node/inspector/cli.mjs")
    .replaceAll("{identity}", identity)
    .replaceAll("{launch_note}", "Run the Node.js inspector from the project root.")
    .replaceAll("{decoder_args}", "")
    .replaceAll("{decoder_note}", "This command uses the Node.js wire decoder. Use sdk-check to compare the Garmin SDK.")
    .replaceAll("{sdk_row}", "  sdk-check     Compare Garmin SDK decode and encode results\n")
    .replaceAll("{sdk_topic}", "- Check official SDK compatibility: `node src/node/inspector/cli.mjs --skill sdk-check`\n");
}
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
function content(f) {
  if (f.kind === "header") return [f.kind, f.protocol, f.profile];
  if (f.kind === "crc") return [f.kind];
  return [f.kind, f.local, f.global_id, f.endian, f.fields, f.timestamp_offset, f.hex.slice(2)];
}
async function run(args) {
  const [command, file, flag, out] = args;
  const help = (value) => value === "--help" || value === "-h";
  if (args.length === 1 && (command === "--version" || command === "-V")) {
    console.log(identity);
    console.log(`Runtime   Node.js ${process.version}`);
    console.log(`Platform  ${process.platform} ${process.arch}`);
    console.log(`FIT       Wire reader, sdk-check requires @garmin/fitsdk ${packageInfo.dependencies["@garmin/fitsdk"]}`);
    return;
  }
  if (help(command) && args.length === 1) {
    process.stdout.write(guide("help.txt"));
    return;
  }
  if (commands.includes(command) && help(file) && args.length === 2) {
    process.stdout.write(guide(command + ".txt"));
    return;
  }
  if (command === "--skill") {
    const topic = file ?? "index";
    if (args.length > 2 || (topic !== "index" && !commands.includes(topic))) {
      console.error("Use --skill index for available topics (" + commands.join(", ") + ").");
      process.exitCode = 2;
      return;
    }
    process.stdout.write(guide(topic + ".md"));
    return;
  }
  if (
    !commands.includes(command) || !file ||
    (command === "roundtrip" ? args.length !== 4 || flag !== "--out" : args.length !== 2)
  ) {
    process.stderr.write(guide("help.txt"));
    process.exitCode = 2;
    return;
  }
  const bytes = readFileSync(file);
  if (command === "raw") {
    for (const f of frames(bytes)) void f; // Validate the entire chain before emitting any lines.
    for (const f of frames(bytes)) console.log(json(f));
  } else if (command === "inspect") console.log(json(inventory(file, bytes)));
  else if (command === "sdk-check") {
    const { sdkCheck } = await import("./sdk.mjs");
    console.log(json(sdkCheck(bytes)));
  }
  else {
    const result = encode(bytes), output = frames(result.bytes);
    for (const f of frames(bytes)) {
      const next = output.next();
      if (next.done || !isDeepStrictEqual(content(f), content(next.value))) {
        throw Error("read-back differs at " + f.offset);
      }
    }
    if (!output.next().done) throw Error("extra output frames");
    const temp = mkdtempSync(join(dirname(out), ".fit-inspector-"));
    try {
      const path = join(temp, "new.fit");
      writeFileSync(path, result.bytes, { mode: 0o600 });
      linkSync(path, out);
    } finally {
      rmSync(temp, { recursive: true });
    }
    console.log(
      json({
        backend: "Node.js wire",
        output: out,
        members: result.members,
        messages: result.messages,
        semantic_equal: true,
        byte_equal: bytes.equals(result.bytes),
        input_sha256: hash(bytes),
        output_sha256: hash(result.bytes),
      }),
    );
  }
}
try {
  await run(process.argv.slice(2));
} catch (e) {
  console.error("fit-inspector:", process.argv[3] ?? "", e.message);
  process.exitCode = 1;
}
