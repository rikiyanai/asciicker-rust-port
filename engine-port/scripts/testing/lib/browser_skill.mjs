export async function openUrl(page, trail, url) {
  await page.goto(url, { waitUntil: "domcontentloaded" });
  return trail.capture(page, "open_url", { url });
}

export async function clickElement(page, trail, selector, options = {}) {
  await page.locator(selector).click(options);
  return trail.capture(page, "click_element", { selector });
}

export async function typeText(page, trail, selector, text, options = {}) {
  await page.locator(selector).fill(text, options);
  return trail.capture(page, "type_text", {
    selector,
    chars: String(text).length,
  });
}

export async function captureScreenshot(page, trail, label, detail = {}) {
  return trail.capture(page, `capture_${label}`, detail);
}
