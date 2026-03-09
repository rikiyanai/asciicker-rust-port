import fs from "node:fs/promises";
import path from "node:path";

function slugify(input) {
  return String(input)
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function nowIso() {
  return new Date().toISOString();
}

export function newRunId(prefix = "run") {
  const stamp = nowIso().replace(/[:.]/g, "-");
  return `${slugify(prefix)}-${stamp}`;
}

export async function createVisualTrail({
  suite,
  story,
  runId,
  artifactsRoot = "artifacts/testing",
}) {
  const safeSuite = slugify(suite || "suite");
  const safeStory = slugify(story || "story");
  const safeRunId = slugify(runId || newRunId("test"));

  const runDir = path.resolve(artifactsRoot, safeRunId);
  const storyDir = path.join(runDir, safeStory);
  const trailPath = path.join(storyDir, "trail.jsonl");
  const summaryPath = path.join(storyDir, "summary.json");

  await fs.mkdir(storyDir, { recursive: true });

  let stepIndex = 0;
  const startTs = Date.now();

  async function appendEntry(entry) {
    await fs.appendFile(trailPath, `${JSON.stringify(entry)}\n`, "utf8");
    const relPath = path.relative(process.cwd(), entry.screenshot || storyDir);
    console.log(
      `[visual-trail] story=${safeStory} step=${entry.step} action=${entry.action} artifact=${relPath}`,
    );
  }

  async function capture(page, action, detail = {}) {
    stepIndex += 1;
    const safeAction = slugify(action || "step");
    const screenshotPath = path.join(
      storyDir,
      `${String(stepIndex).padStart(3, "0")}-${safeAction}.png`,
    );

    await page.screenshot({ path: screenshotPath, fullPage: true });
    const title = await page.title().catch(() => "");
    const entry = {
      runId: safeRunId,
      suite: safeSuite,
      story: safeStory,
      step: stepIndex,
      action,
      detail,
      url: page.url(),
      title,
      screenshot: screenshotPath,
      timestamp: nowIso(),
    };
    await appendEntry(entry);
    return entry;
  }

  async function note(action, detail = {}) {
    stepIndex += 1;
    const entry = {
      runId: safeRunId,
      suite: safeSuite,
      story: safeStory,
      step: stepIndex,
      action,
      detail,
      timestamp: nowIso(),
    };
    await fs.appendFile(trailPath, `${JSON.stringify(entry)}\n`, "utf8");
    console.log(
      `[visual-trail] story=${safeStory} step=${entry.step} action=${entry.action}`,
    );
    return entry;
  }

  async function complete(status, detail = {}) {
    const summary = {
      runId: safeRunId,
      suite: safeSuite,
      story: safeStory,
      status,
      stepCount: stepIndex,
      startedAt: new Date(startTs).toISOString(),
      finishedAt: nowIso(),
      durationMs: Date.now() - startTs,
      detail,
      trailPath,
      summaryPath,
      storyDir,
    };
    await fs.writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
    return summary;
  }

  return {
    runId: safeRunId,
    suite: safeSuite,
    story: safeStory,
    runDir,
    storyDir,
    trailPath,
    summaryPath,
    capture,
    note,
    complete,
  };
}
