import {
  captureScreenshot,
  clickElement,
  openUrl,
} from "../lib/browser_skill.mjs";
import { createVisualTrail } from "../lib/visual_trail.mjs";

export async function runMenuStory({ browser, baseUrl, runId, suite = "e2e" }) {
  const context = await browser.newContext();
  const page = await context.newPage();
  const trail = await createVisualTrail({
    suite,
    story: "menu-start",
    runId,
  });

  try {
    await openUrl(page, trail, baseUrl);
    await clickElement(page, trail, "#start-game-button");
    await page.waitForFunction(
      () => {
        const node = document.querySelector("#game-state");
        return node && node.getAttribute("data-state") === "playing";
      },
      { timeout: 5000 },
    );
    await captureScreenshot(page, trail, "state_playing", { selector: "#game-state" });

    await clickElement(page, trail, "#pan-camera-button");
    await captureScreenshot(page, trail, "camera_panned", { selector: "#camera-readout" });

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
