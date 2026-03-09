import { asBool, asInt, nonEmpty, parseArgs } from "./lib/cli.mjs";
import { startFixtureServer } from "./lib/fixture_server.mjs";
import { newRunId } from "./lib/visual_trail.mjs";
import { runSmokeStory } from "./stories/smoke_story.mjs";

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch (_error) {
  console.error(
    "Playwright module not found. Run `just install-testing-deps` or `npm install` in engine-port.",
  );
  process.exit(2);
}

const args = parseArgs();
const debugMode = asBool(args.debug, false);
const headedMode = asBool(args.headed, false);
const headless = debugMode || headedMode ? false : asBool(args.headless, true);
const slowMo = debugMode ? asInt(args.slowmo, 200) : asInt(args.slowmo, 0);

let baseUrl = nonEmpty(args["base-url"], nonEmpty(process.env.TEST_BASE_URL, ""));
const runId = nonEmpty(args["run-id"], newRunId("smoke"));
const fixturePort = asInt(args.port, 4173);
const fixtureHost = nonEmpty(args.host, "127.0.0.1");

let fixtureServer;
if (!baseUrl) {
  fixtureServer = await startFixtureServer({ host: fixtureHost, port: fixturePort });
  baseUrl = fixtureServer.baseUrl;
  console.log(`[smoke] No --base-url provided, using fixture server at ${baseUrl}`);
}

const browser = await chromium.launch({ headless, slowMo });
let result;
try {
  result = await runSmokeStory({ browser, baseUrl, runId, suite: "smoke" });
} finally {
  await browser.close();
  if (fixtureServer) {
    await fixtureServer.close();
  }
}

console.log(
  `[smoke] status=${result.ok ? "passed" : "failed"} summary=${result.summary.summaryPath}`,
);

if (!result.ok) {
  process.exit(1);
}
