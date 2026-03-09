import {
  captureScreenshot,
  clickElement,
  openUrl,
} from "../lib/browser_skill.mjs";
import { createVisualTrail } from "../lib/visual_trail.mjs";

export async function runSmokeStory({ browser, baseUrl, runId, suite = "smoke" }) {
  const context = await browser.newContext();
  const page = await context.newPage();
  const trail = await createVisualTrail({
    suite,
    story: "smoke",
    runId,
  });

  try {
    await openUrl(page, trail, baseUrl);
    await page.waitForSelector("#app-root", { timeout: 5000 });
    await captureScreenshot(page, trail, "app_root_visible", {
      selector: "#app-root",
    });

    await clickElement(page, trail, "#learn-more-button");
    await page.waitForSelector("#details-panel", { state: "visible", timeout: 3000 });
    await captureScreenshot(page, trail, "details_visible", {
      selector: "#details-panel",
    });

    const summary = await trail.complete("passed", { baseUrl });
    return { ok: true, summary };
  } catch (error) {
    await trail.capture(page, "error", { message: error.message });
    const summary = await trail.complete("failed", {
      baseUrl,
      error: error.stack || error.message,
    });
    return { ok: false, summary, error };
  } finally {
    await context.close();
  }
}
