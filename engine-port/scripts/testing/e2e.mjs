import { asBool, asInt, nonEmpty, parseArgs } from "./lib/cli.mjs";
import { startFixtureServer } from "./lib/fixture_server.mjs";
import { newRunId } from "./lib/visual_trail.mjs";
import { runMenuStory } from "./stories/menu_story.mjs";
import { runSmokeStory } from "./stories/smoke_story.mjs";
import { runWaterSurfaceStory } from "./stories/water_surface_story.mjs";

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
const feature = nonEmpty(args.feature, "full").toLowerCase();
const debugMode = asBool(args.debug, false);
const headedMode = asBool(args.headed, false);
const headless = debugMode || headedMode ? false : asBool(args.headless, true);
const slowMo = debugMode ? asInt(args.slowmo, 200) : asInt(args.slowmo, 0);
const runId = nonEmpty(args["run-id"], newRunId(`e2e-${feature}`));
const fixturePort = asInt(args.port, 4173);
const fixtureHost = nonEmpty(args.host, "127.0.0.1");

let baseUrl = nonEmpty(args["base-url"], nonEmpty(process.env.TEST_BASE_URL, ""));
let fixtureServer;
if (!baseUrl) {
  fixtureServer = await startFixtureServer({ host: fixtureHost, port: fixturePort });
  baseUrl = fixtureServer.baseUrl;
  console.log(`[e2e] No --base-url provided, using fixture server at ${baseUrl}`);
}

const browser = await chromium.launch({ headless, slowMo });
const results = [];

try {
  if (feature === "smoke") {
    results.push(await runSmokeStory({ browser, baseUrl, runId, suite: "e2e" }));
  } else if (feature === "menu") {
    results.push(await runMenuStory({ browser, baseUrl, runId, suite: "e2e" }));
  } else if (feature === "water") {
    results.push(await runWaterSurfaceStory({ browser, baseUrl, runId, suite: "e2e" }));
  } else {
    // full/default: project-relevant stories only
    results.push(await runMenuStory({ browser, baseUrl, runId, suite: "e2e" }));
    results.push(await runWaterSurfaceStory({ browser, baseUrl, runId, suite: "e2e" }));
    results.push(await runSmokeStory({ browser, baseUrl, runId, suite: "e2e" }));
  }
} finally {
  await browser.close();
  if (fixtureServer) {
    await fixtureServer.close();
  }
}

const failed = results.filter((r) => !r.ok);
for (const result of results) {
  console.log(
    `[e2e] story=${result.summary.story} status=${result.ok ? "passed" : "failed"} summary=${result.summary.summaryPath}`,
  );
}

if (failed.length > 0) {
  process.exit(1);
}
