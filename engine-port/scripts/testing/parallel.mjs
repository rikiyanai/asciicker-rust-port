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

async function runWithConcurrency(items, limit, worker) {
  const results = [];
  let cursor = 0;

  async function runOne() {
    while (true) {
      const idx = cursor;
      cursor += 1;
      if (idx >= items.length) {
        return;
      }
      const item = items[idx];
      results[idx] = await worker(item);
    }
  }

  const workers = Array.from({ length: Math.max(1, limit) }, () => runOne());
  await Promise.all(workers);
  return results;
}

function storiesForSuite(suite) {
  if (suite === "smoke") {
    return ["smoke"];
  }
  if (suite === "menu") {
    return ["menu"];
  }
  if (suite === "water") {
    return ["water"];
  }
  return ["smoke", "menu", "water"];
}

const args = parseArgs();
const suite = nonEmpty(args.suite, "core").toLowerCase();
const workers = asInt(args.workers, 3);
const debugMode = asBool(args.debug, false);
const headedMode = asBool(args.headed, false);
const headless = debugMode || headedMode ? false : asBool(args.headless, true);
const slowMo = debugMode ? asInt(args.slowmo, 200) : asInt(args.slowmo, 0);
const runId = nonEmpty(args["run-id"], newRunId(`parallel-${suite}`));
const fixturePort = asInt(args.port, 4173);
const fixtureHost = nonEmpty(args.host, "127.0.0.1");

let baseUrl = nonEmpty(args["base-url"], nonEmpty(process.env.TEST_BASE_URL, ""));
let fixtureServer;
if (!baseUrl) {
  fixtureServer = await startFixtureServer({ host: fixtureHost, port: fixturePort });
  baseUrl = fixtureServer.baseUrl;
  console.log(`[parallel] No --base-url provided, using fixture server at ${baseUrl}`);
}

const stories = storiesForSuite(suite);
const browser = await chromium.launch({ headless, slowMo });
let results = [];

try {
  results = await runWithConcurrency(stories, workers, async (story) => {
    if (story === "menu") {
      return runMenuStory({ browser, baseUrl, runId, suite: "parallel" });
    }
    if (story === "water") {
      return runWaterSurfaceStory({ browser, baseUrl, runId, suite: "parallel" });
    }
    return runSmokeStory({ browser, baseUrl, runId, suite: "parallel" });
  });
} finally {
  await browser.close();
  if (fixtureServer) {
    await fixtureServer.close();
  }
}

const failed = results.filter((r) => !r.ok);
for (const result of results) {
  console.log(
    `[parallel] story=${result.summary.story} status=${result.ok ? "passed" : "failed"} summary=${result.summary.summaryPath}`,
  );
}

if (failed.length > 0) {
  process.exit(1);
}
