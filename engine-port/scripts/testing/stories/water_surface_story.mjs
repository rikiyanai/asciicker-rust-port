import {
  captureScreenshot,
  clickElement,
  openUrl,
} from "../lib/browser_skill.mjs";
import { createVisualTrail } from "../lib/visual_trail.mjs";

export async function runWaterSurfaceStory({
  browser,
  baseUrl,
  runId,
  suite = "e2e",
}) {
  const context = await browser.newContext();
  const page = await context.newPage();
  const trail = await createVisualTrail({
    suite,
    story: "water-surface",
    runId,
  });

  try {
    await openUrl(page, trail, baseUrl);

    const gameState = await page.locator("#game-state").getAttribute("data-state");
    if (gameState !== "playing") {
      await clickElement(page, trail, "#start-game-button");
      await page.waitForFunction(
        () => {
          const node = document.querySelector("#game-state");
          return node && node.getAttribute("data-state") === "playing";
        },
        { timeout: 5000 },
      );
    }

    const waterState = await page.locator("#water-surface").getAttribute("data-state");
    if (waterState !== "active") {
      await clickElement(page, trail, "#toggle-water-button");
      await page.waitForFunction(
        () => {
          const node = document.querySelector("#water-surface");
          return node && node.getAttribute("data-state") === "active";
        },
        { timeout: 5000 },
      );
    }
    await captureScreenshot(page, trail, "water_active", {
      selector: "#water-surface",
    });

    const frameBefore = Number.parseInt(
      (await page.locator("#water-surface").getAttribute("data-frame")) || "0",
      10,
    );
    await clickElement(page, trail, "#advance-water-frame-button");
    await page.waitForFunction(
      (before) => {
        const node = document.querySelector("#water-surface");
        if (!node) {
          return false;
        }
        const current = Number.parseInt(node.getAttribute("data-frame") || "0", 10);
        return current > before;
      },
      frameBefore,
      { timeout: 5000 },
    );
    await captureScreenshot(page, trail, "water_frame_advanced", {
      selector: "#water-surface",
      frameBefore,
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
