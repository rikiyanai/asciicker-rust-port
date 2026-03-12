import fs from "node:fs/promises";
import path from "node:path";

import { asBool, asInt, nonEmpty, parseArgs } from "./lib/cli.mjs";
import { startFixtureServer } from "./lib/fixture_server.mjs";
import { comparePngImages, readPng, writePng } from "./lib/png_diff.mjs";
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

const STORY_RUNNERS = {
  menu: (options) => runMenuStory({ ...options, suite: "watchdog" }),
  smoke: (options) => runSmokeStory({ ...options, suite: "watchdog" }),
  water: (options) => runWaterSurfaceStory({ ...options, suite: "watchdog" }),
};

function selectedStories(storyFlag) {
  const value = nonEmpty(storyFlag, "all").toLowerCase();
  if (value === "all") {
    return ["menu", "water", "smoke"];
  }
  if (!STORY_RUNNERS[value]) {
    throw new Error(`unknown story: ${value}`);
  }
  return [value];
}

async function readJson(filePath) {
  return JSON.parse(await fs.readFile(filePath, "utf8"));
}

async function collectStoryScreenshots(summary) {
  const trailEntries = (await fs.readFile(summary.trailPath, "utf8"))
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));

  return trailEntries
    .filter((entry) => entry.screenshot)
    .map((entry) => ({
      action: entry.action,
      screenshot: entry.screenshot,
      fileName: path.basename(entry.screenshot),
    }));
}

async function ensureDir(dirPath) {
  await fs.mkdir(dirPath, { recursive: true });
}

async function fileExists(filePath) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

async function updateBaseline({ baselineRoot, story, screenshots, summary }) {
  const storyDir = path.join(baselineRoot, story);
  await fs.rm(storyDir, { recursive: true, force: true });
  await ensureDir(storyDir);

  const manifest = {
    story,
    updatedAt: new Date().toISOString(),
    summary: {
      status: summary.status,
      stepCount: summary.stepCount,
      detail: summary.detail,
    },
    screenshots: [],
  };

  for (const shot of screenshots) {
    const target = path.join(storyDir, shot.fileName);
    await fs.copyFile(shot.screenshot, target);
    manifest.screenshots.push({
      action: shot.action,
      fileName: shot.fileName,
    });
  }

  await fs.writeFile(
    path.join(storyDir, "manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
    "utf8",
  );

  return manifest;
}

async function compareAgainstBaseline({
  baselineRoot,
  story,
  screenshots,
  artifactDir,
  mismatchThreshold,
}) {
  const storyDir = path.join(baselineRoot, story);
  const manifestPath = path.join(storyDir, "manifest.json");

  if (!(await fileExists(manifestPath))) {
    return {
      ok: false,
      reason: "missing_baseline",
      baselineDir: storyDir,
      comparisons: [],
    };
  }

  const manifest = await readJson(manifestPath);
  const expected = new Map(manifest.screenshots.map((entry) => [entry.fileName, entry]));
  const comparisons = [];
  let mismatchCount = 0;

  for (const shot of screenshots) {
    const baselinePath = path.join(storyDir, shot.fileName);
    if (!(await fileExists(baselinePath))) {
      mismatchCount += 1;
      comparisons.push({
        fileName: shot.fileName,
        action: shot.action,
        ok: false,
        reason: "missing_baseline_image",
        baselinePath,
        actualPath: shot.screenshot,
      });
      continue;
    }

    const [actual, baseline] = await Promise.all([readPng(shot.screenshot), readPng(baselinePath)]);
    const result = comparePngImages(actual, baseline);
    const { diffImage, ...serializableResult } = result;
    const ok =
      serializableResult.reason === "match" ||
      serializableResult.mismatchRatio <= mismatchThreshold;

    const comparison = {
      fileName: shot.fileName,
      action: shot.action,
      ok,
      baselinePath,
      actualPath: shot.screenshot,
      ...serializableResult,
    };

    if (!ok) {
      mismatchCount += 1;
      const diffPath = path.join(artifactDir, `${shot.fileName.replace(/\.png$/i, "")}-diff.png`);
      await writePng(diffPath, diffImage);
      comparison.diffPath = diffPath;
    }

    comparisons.push(comparison);
    expected.delete(shot.fileName);
  }

  for (const leftover of expected.values()) {
    mismatchCount += 1;
    comparisons.push({
      fileName: leftover.fileName,
      action: leftover.action,
      ok: false,
      reason: "missing_actual_image",
      baselinePath: path.join(storyDir, leftover.fileName),
    });
  }

  return {
    ok: mismatchCount === 0,
    reason: mismatchCount === 0 ? "match" : "baseline_mismatch",
    baselineDir: storyDir,
    mismatchCount,
    comparisons,
  };
}

async function sleep(ms) {
  if (ms <= 0) {
    return;
  }
  await new Promise((resolve) => setTimeout(resolve, ms));
}

const args = parseArgs();
const stories = selectedStories(args.story);
const cycles = Math.max(1, asInt(args.cycles, 3));
const delayMs = Math.max(0, asInt(args["delay-ms"], 250));
const headless = asBool(args.headless, true);
const debug = asBool(args.debug, false);
const slowMo = debug ? asInt(args.slowmo, 200) : 0;
const fixturePort = asInt(args.port, 4173);
const fixtureHost = nonEmpty(args.host, "127.0.0.1");
const runId = nonEmpty(args["run-id"], newRunId("watchdog"));
const updateBaselineFlag = asBool(args["update-baseline"], false);
const failFast = asBool(args["fail-fast"], false);
const mismatchThreshold = Number.parseFloat(nonEmpty(args["mismatch-threshold"], "0"));
const artifactsRoot = path.resolve(nonEmpty(args["artifacts-root"], "artifacts/testing"));
const baselineRoot = path.resolve(nonEmpty(args["baseline-root"], "tests/visual-baselines"));

let baseUrl = nonEmpty(args["base-url"], nonEmpty(process.env.TEST_BASE_URL, ""));
let fixtureServer;
if (!baseUrl) {
  fixtureServer = await startFixtureServer({ host: fixtureHost, port: fixturePort });
  baseUrl = fixtureServer.baseUrl;
  console.log(`[watchdog] No --base-url provided, using fixture server at ${baseUrl}`);
}

const browser = await chromium.launch({ headless, slowMo });
const runDir = path.join(artifactsRoot, runId, "watchdog");
await ensureDir(runDir);

const results = [];

try {
  for (const story of stories) {
    const runner = STORY_RUNNERS[story];
    for (let cycle = 1; cycle <= cycles; cycle += 1) {
      const storyRunId = `${runId}-${story}-cycle-${String(cycle).padStart(3, "0")}`;
      const cycleArtifactDir = path.join(runDir, story, `cycle-${String(cycle).padStart(3, "0")}`);
      await ensureDir(cycleArtifactDir);

      const execution = await runner({ browser, baseUrl, runId: storyRunId });
      const summary = execution.summary ?? (await readJson(path.join(cycleArtifactDir, "summary.json")));
      const screenshots = await collectStoryScreenshots(summary);

      let baselineResult;
      if (updateBaselineFlag && cycle === 1) {
        const manifest = await updateBaseline({
          baselineRoot,
          story,
          screenshots,
          summary,
        });
        baselineResult = {
          ok: true,
          reason: "baseline_updated",
          baselineDir: path.join(baselineRoot, story),
          manifest,
          comparisons: [],
        };
      } else {
        baselineResult = await compareAgainstBaseline({
          baselineRoot,
          story,
          screenshots,
          artifactDir: cycleArtifactDir,
          mismatchThreshold,
        });
      }

      const cycleResult = {
        story,
        cycle,
        ok: execution.ok && baselineResult.ok,
        executionOk: execution.ok,
        baselineOk: baselineResult.ok,
        summaryPath: summary.summaryPath,
        storyDir: summary.storyDir,
        baseline: baselineResult,
      };
      results.push(cycleResult);

      const cycleSummaryPath = path.join(cycleArtifactDir, "watchdog-cycle.json");
      await fs.writeFile(cycleSummaryPath, `${JSON.stringify(cycleResult, null, 2)}\n`, "utf8");
      console.log(
        `[watchdog] story=${story} cycle=${cycle} status=${cycleResult.ok ? "passed" : "failed"} summary=${cycleSummaryPath}`,
      );

      if (!cycleResult.ok && failFast) {
        break;
      }
      if (cycle < cycles) {
        await sleep(delayMs);
      }
    }
  }
} finally {
  await browser.close();
  if (fixtureServer) {
    await fixtureServer.close();
  }
}

const summary = {
  runId,
  baseUrl,
  cycles,
  stories,
  updateBaseline: updateBaselineFlag,
  mismatchThreshold,
  startedAt: new Date().toISOString(),
  results,
  failed: results.filter((entry) => !entry.ok).length,
};

const summaryPath = path.join(runDir, "watchdog-summary.json");
await fs.writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
console.log(`[watchdog] complete failed=${summary.failed} summary=${summaryPath}`);

if (summary.failed > 0) {
  process.exit(1);
}
