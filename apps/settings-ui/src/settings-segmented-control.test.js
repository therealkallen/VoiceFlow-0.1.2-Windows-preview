const assert = require("node:assert/strict");
const path = require("node:path");
const { pathToFileURL } = require("node:url");
const { chromium } = require("playwright");

async function run() {
  const browser = await chromium.launch({ channel: "msedge", headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1180, height: 820 } });
    const indexPath = path.resolve(__dirname, "..", "index.html");
    await page.goto(pathToFileURL(indexPath).href);
    await page.locator('[data-section-link="settings"]').click();
    await page.locator('[data-settings-section="voice-output"]').click();

    const group = page.locator('.settings-segmented[data-segment="mode"]');
    const pushToTalk = group.locator('[data-value="PushToTalk"]');
    const toggle = group.locator('[data-value="Toggle"]');
    const [pushBox, toggleBox] = await Promise.all([
      pushToTalk.boundingBox(),
      toggle.boundingBox(),
    ]);
    assert.ok(pushBox && toggleBox, "both mode options should be visible");
    assert.ok(
      Math.abs(pushBox.width - toggleBox.width) <= 1,
      `mode hitboxes should be equal: ${pushBox.width} vs ${toggleBox.width}`,
    );

    await pushToTalk.hover({ position: { x: pushBox.width / 2, y: pushBox.height / 2 } });
    assert.equal(await pushToTalk.evaluate((element) => element.matches(":hover")), true);
    assert.equal(await toggle.evaluate((element) => element.matches(":hover")), false);

    await toggle.hover({ position: { x: toggleBox.width / 2, y: toggleBox.height / 2 } });
    assert.equal(await pushToTalk.evaluate((element) => element.matches(":hover")), false);
    assert.equal(await toggle.evaluate((element) => element.matches(":hover")), true);

    const currentToggleBox = await toggle.boundingBox();
    assert.ok(currentToggleBox);
    await page.mouse.move(currentToggleBox.x + 6, currentToggleBox.y + currentToggleBox.height / 2);
    assert.equal(await pushToTalk.evaluate((element) => element.matches(":hover")), false);
    assert.equal(await toggle.evaluate((element) => element.matches(":hover")), true);

    await toggle.click();
    assert.equal(await toggle.getAttribute("aria-checked"), "true");
    assert.equal(await pushToTalk.getAttribute("aria-checked"), "false");
    await page.mouse.move(2, 2);
    assert.equal(await toggle.evaluate((element) => element.matches(":hover")), false);
    assert.equal(await toggle.evaluate((element) => element.classList.contains("is-active")), true);
    assert.equal(await pushToTalk.evaluate((element) => element.classList.contains("is-active")), false);

    await group.evaluate((element) => {
      const tabStart = document.createElement("button");
      tabStart.id = "segmented-control-tab-start";
      element.before(tabStart);
      tabStart.focus();
    });
    await page.keyboard.press("Tab");
    assert.equal(
      await page.evaluate(() => document.activeElement?.getAttribute("data-value")),
      "Toggle",
    );
    await page.keyboard.press("ArrowLeft");
    assert.equal(await pushToTalk.getAttribute("aria-checked"), "true");
    assert.equal(await toggle.getAttribute("aria-checked"), "false");
    await page.keyboard.press("ArrowRight");
    assert.equal(await toggle.getAttribute("aria-checked"), "true");
    await page.keyboard.press("Space");
    assert.equal(await toggle.getAttribute("aria-checked"), "true");
    assert.equal(await toggle.getAttribute("tabindex"), "0");
    assert.equal(await pushToTalk.getAttribute("tabindex"), "-1");

    console.log("settings segmented-control hitbox test passed");
  } finally {
    await browser.close();
  }
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
