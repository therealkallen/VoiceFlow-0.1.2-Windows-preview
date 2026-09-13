const assert = require("node:assert/strict");
const path = require("node:path");
const { chromium } = require("playwright");

const expectedSectionOrder = [
  "general",
  "voice-output",
  "history-privacy",
  "ai-provider",
  "advanced",
];

function assertSameBox(actual, expected, message) {
  for (const key of ["x", "y", "width", "height"]) {
    assert.ok(
      Math.abs(actual[key] - expected[key]) <= 3,
      `${message}: ${key} changed from ${expected[key]} to ${actual[key]}`,
    );
  }
}

async function documentBox(locator) {
  return locator.evaluate((element) => {
    const box = element.getBoundingClientRect();
    return {
      x: box.x + window.scrollX,
      y: box.y + window.scrollY,
      width: box.width,
      height: box.height,
    };
  });
}

async function focusPresentation(locator) {
  await locator.focus();
  return locator.evaluate((control) => {
    const shell = control.closest(".input-shell");
    const controlStyle = getComputedStyle(control);
    const shellStyle = getComputedStyle(shell);
    const beforeStyle = getComputedStyle(shell, "::before");
    const afterStyle = getComputedStyle(shell, "::after");
    return {
      outlineStyle: controlStyle.outlineStyle,
      outlineWidth: controlStyle.outlineWidth,
      shellShadow: shellStyle.boxShadow,
      shellRadius: shellStyle.borderRadius,
      beforeContent: beforeStyle.content,
      beforePointerEvents: beforeStyle.pointerEvents,
      afterContent: afterStyle.content,
      afterPointerEvents: afterStyle.pointerEvents,
    };
  });
}

async function run() {
  const browser = await chromium.launch({ channel: "msedge", headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1180, height: 900 } });
    const indexPath = path.resolve(__dirname, "..", "index.html");
    // Test data must not depend on a developer's generated history or server URL.
    const liveRuntime = { settings_warnings: [], settings_source: "defaults" };
    const settingsControlUrl = "http://127.0.0.1:47831";
    let runtimeRevision = Date.now();
    liveRuntime.generated_at_epoch_ms = runtimeRevision;
    liveRuntime.settings_control_url = settingsControlUrl;
    liveRuntime.system_language = "English";
    liveRuntime.provider = {
      saved_preset: "bailian",
      saved_base_url: null,
      saved_active_model: "qwen3.7-plus",
      saved_request_timeout_ms: null,
      preset: "bailian",
      preset_source: "built_in",
      display_label: "Alibaba Cloud Bailian",
      configured: true,
      key_present: true,
      key_source: "credential_store",
      stored_credential_present: true,
      effective_key_present: true,
      effective_key_source: "credential_store",
      credential_store_status: "available",
      credential_status_by_preset: {
        bailian: {
          stored_credential_present: true,
          credential_store_status: "available",
        },
        custom_openai_compatible: {
          stored_credential_present: false,
          credential_store_status: "missing",
        },
        tencent_hunyuan: {
          stored_credential_present: false,
          credential_store_status: "missing",
        },
        volcengine_ark: {
          stored_credential_present: false,
          credential_store_status: "missing",
        },
      },
      env_key_override: false,
      base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      base_url_source: "built_in",
      model_code: "qwen3.7-plus",
      model_source: "settings",
      request_timeout_ms: 12000,
      request_timeout_source: "built_in",
      enable_thinking: false,
      enable_thinking_source: "built_in",
      supports_dashscope_enable_thinking: true,
    };

    await page.route(`${settingsControlUrl}/**`, async (route) => {
      const request = route.request();
      const url = new URL(request.url());
      if (url.pathname === "/src/runtime-state.js") {
        await route.fulfill({ contentType: "text/javascript", body: `window.__VOICEFLOW_SETTINGS_RUNTIME__ = ${JSON.stringify(liveRuntime)};` });
        return;
      }
      if (url.pathname !== "/runtime-state" && url.pathname !== "/update-settings") {
        const asset = path.resolve(path.dirname(indexPath), url.pathname === "/" ? "index.html" : `.${url.pathname}`);
        assert.ok(asset.startsWith(path.dirname(indexPath) + path.sep));
        await route.fulfill({ path: asset });
        return;
      }
      const corsHeaders = {
        "access-control-allow-origin": "*",
        "access-control-allow-headers": "*",
        "access-control-allow-methods": "GET, POST, OPTIONS",
      };
      if (request.method() === "OPTIONS") {
        await route.fulfill({ status: 204, headers: corsHeaders });
        return;
      }
      if (url.pathname === "/update-settings" && request.method() === "POST") {
        const update = request.postDataJSON().provider ?? {};
        if (Object.hasOwn(update, "preset")) {
          liveRuntime.provider.saved_preset = update.preset;
        }
        if (Object.hasOwn(update, "base_url")) {
          liveRuntime.provider.saved_base_url = update.base_url;
        }
        if (Object.hasOwn(update, "active_model")) {
          liveRuntime.provider.saved_active_model = update.active_model;
        }
        if (Object.hasOwn(update, "request_timeout_ms")) {
          liveRuntime.provider.saved_request_timeout_ms = update.request_timeout_ms;
        }
      }
      runtimeRevision += 1;
      liveRuntime.generated_at_epoch_ms = runtimeRevision;
      await route.fulfill({ json: liveRuntime, headers: corsHeaders });
    });
    await page.addInitScript(() => {
      window.__VOICEFLOW_SETTINGS_BRIDGE__ = { controlToken: "provider-ui-test" };
    });
    await page.goto(settingsControlUrl);
    await page.locator('[data-section-link="settings"]').click();

    const sectionButtons = page.locator(".settings-subnav [data-settings-section]");
    assert.deepEqual(
      await sectionButtons.evaluateAll((buttons) =>
        buttons.map((button) => button.dataset.settingsSection),
      ),
      expectedSectionOrder,
    );
    await sectionButtons.first().focus();
    for (const expectedSection of expectedSectionOrder.slice(1)) {
      await page.keyboard.press("Tab");
      assert.equal(
        await page.evaluate(() => document.activeElement?.dataset.settingsSection),
        expectedSection,
      );
    }

    await page.locator('[data-settings-section="general"]').click();
    await page.locator('[data-segment="systemLanguage"] [data-value="English"]').click();
    await page.locator('[data-settings-section="ai-provider"]').click();
    await page.locator(".provider-advanced").evaluate((element) => {
      element.open = true;
    });

    const preset = page.locator("#provider-preset");
    const modelField = page.locator("#provider-model-field");
    const modelInput = page.locator("#provider-active-model");
    const timeoutInput = page.locator("#provider-timeout-seconds");
    const customBaseUrlField = page.locator("#provider-custom-base-url-field");
    const customBaseUrl = page.locator("#provider-base-url");
    const baseUrlMessage = page.locator("#provider-base-url-message");
    const presetEndpoint = page.locator("#provider-preset-endpoint-settings");
    const presetBaseUrl = page.locator("#provider-preset-base-url");
    const overrideToggle = page.locator("#provider-endpoint-override");
    const overrideBaseUrlField = page.locator("#provider-override-base-url-field");
    const overrideBaseUrl = page.locator("#provider-override-base-url");

    assert.equal(await preset.inputValue(), "bailian");
    await overrideToggle.check();
    assert.equal(await overrideToggle.isChecked(), true);
    await page.waitForTimeout(2500);
    assert.equal(
      await overrideToggle.isChecked(),
      true,
      "Periodic runtime refreshes must not revert a dirty endpoint override",
    );
    const savedOverrideUrl = "https://saved-gateway.example/v1";
    await overrideBaseUrl.fill(savedOverrideUrl);
    liveRuntime.provider.base_url = "https://environment-gateway.example/v1";
    liveRuntime.provider.base_url_source = "env";
    liveRuntime.provider.credential_status_by_preset.bailian = {
      stored_credential_present: false,
      credential_store_status: "missing",
    };
    await page.waitForTimeout(2500);
    assert.equal(await overrideToggle.isChecked(), true);
    assert.equal(await overrideBaseUrl.inputValue(), savedOverrideUrl);
    assert.match(
      await page.locator("#provider-effective-base-url").textContent(),
      /environment-gateway\.example\/v1/,
    );
    assert.equal(await page.locator("#provider-stored-credential-status").textContent(), "Missing");

    await page.locator('[data-settings-section="advanced"]').click();
    await page.locator('[data-settings-section="ai-provider"]').click();
    assert.equal(await overrideToggle.isChecked(), true);
    assert.equal(await overrideBaseUrl.inputValue(), savedOverrideUrl);

    const saveResponse = page.waitForResponse(
      (response) => response.url() === `${settingsControlUrl}/update-settings`,
    );
    await page.locator("#apply-settings").click();
    await saveResponse;
    assert.equal(await overrideToggle.isChecked(), true);
    assert.equal(await overrideBaseUrl.inputValue(), savedOverrideUrl);
    await page.waitForTimeout(2500);
    assert.equal(await overrideToggle.isChecked(), true);
    assert.equal(await overrideBaseUrl.inputValue(), savedOverrideUrl);
    assert.match(
      await page.locator("#provider-effective-base-url").textContent(),
      /environment-gateway\.example\/v1/,
    );

    const unsavedOverrideUrl = "https://unsaved-gateway.example/v1";
    await preset.selectOption("tencent_hunyuan");
    await overrideToggle.check();
    await overrideBaseUrl.fill(unsavedOverrideUrl);
    await modelInput.fill("hunyuan-draft-model");
    await timeoutInput.fill("23");
    await page.waitForTimeout(2500);
    assert.equal(await preset.inputValue(), "tencent_hunyuan");
    assert.equal(await overrideToggle.isChecked(), true);
    assert.equal(await overrideBaseUrl.inputValue(), unsavedOverrideUrl);
    assert.equal(await modelInput.inputValue(), "hunyuan-draft-model");
    assert.equal(await timeoutInput.inputValue(), "23");
    await page.locator("#revert-settings").click();
    assert.equal(await preset.inputValue(), "bailian");
    assert.equal(await overrideToggle.isChecked(), true);
    assert.equal(await overrideBaseUrl.inputValue(), savedOverrideUrl);
    assert.equal(await modelInput.inputValue(), "qwen3.7-plus");
    assert.equal(await timeoutInput.inputValue(), "");

    await page.locator("#provider-endpoint-reset").click();
    assert.equal(await overrideToggle.isChecked(), false);
    await page.waitForTimeout(2500);
    assert.equal(
      await overrideToggle.isChecked(),
      false,
      "Reset to provider default should remain intentional during refresh",
    );
    await page.locator("#revert-settings").click();
    assert.equal(await overrideToggle.isChecked(), true);
    assert.equal(await overrideBaseUrl.inputValue(), savedOverrideUrl);

    await preset.selectOption("volcengine_ark");
    // Help text can wrap differently per provider; the typing target must stay stable.
    const stableModelBox = await documentBox(modelInput);
    assert.ok(stableModelBox, "Current model should be visible");
    for (const provider of [
      "tencent_hunyuan",
      "custom_openai_compatible",
      "bailian",
      "volcengine_ark",
    ]) {
      await preset.selectOption(provider);
      const nextBox = await documentBox(modelInput);
      assert.ok(nextBox, `Current model should remain visible for ${provider}`);
      assertSameBox(nextBox, stableModelBox, `Current model layout for ${provider}`);
    }

    await preset.selectOption("custom_openai_compatible");
    assert.equal(await customBaseUrlField.isVisible(), true);
    assert.equal(await presetEndpoint.isHidden(), true);
    const [modelBox, baseUrlBox] = await Promise.all([
      modelField.boundingBox(),
      customBaseUrlField.boundingBox(),
    ]);
    assert.ok(modelBox && baseUrlBox, "Custom provider fields should be measurable");
    assert.ok(
      baseUrlBox.y >= modelBox.y + modelBox.height,
      "Custom Base URL should render beneath Current model",
    );
    assert.equal(await baseUrlMessage.evaluate((element) => element.classList.contains("is-error")), false);
    assert.equal(await customBaseUrl.getAttribute("aria-invalid"), "false");

    const credentialBox = page.locator(".provider-credential-box");
    const [messageBoxBefore, credentialBoxBefore] = await Promise.all([
      baseUrlMessage.boundingBox(),
      credentialBox.boundingBox(),
    ]);
    await page.evaluate(() => window.applySettingsThroughHost());
    assert.equal(await baseUrlMessage.evaluate((element) => element.classList.contains("is-error")), true);
    assert.equal(await customBaseUrl.getAttribute("aria-invalid"), "true");
    assert.equal((await baseUrlMessage.textContent()).includes("require"), true);
    const [messageBoxAfter, credentialBoxAfter] = await Promise.all([
      baseUrlMessage.boundingBox(),
      credentialBox.boundingBox(),
    ]);
    assertSameBox(messageBoxAfter, messageBoxBefore, "Base URL message slot");
    assert.ok(
      Math.abs(credentialBoxAfter.y - credentialBoxBefore.y) <= 1,
      "Validation should not move the credential area",
    );

    await customBaseUrl.fill("https://gateway.example/v1");
    assert.equal(await baseUrlMessage.evaluate((element) => element.classList.contains("is-error")), false);
    assert.equal(await customBaseUrl.getAttribute("aria-invalid"), "false");
    assert.equal((await baseUrlMessage.textContent()).includes("require"), true);

    await modelInput.fill("draft-model-id");
    await page.locator('[data-settings-section="advanced"]').click();
    await page.locator('[data-settings-section="ai-provider"]').click();
    assert.equal(await modelInput.inputValue(), "draft-model-id");

    await preset.selectOption("volcengine_ark");
    assert.equal(await customBaseUrlField.isHidden(), true);
    assert.equal(await presetEndpoint.isVisible(), true);
    assert.match(await presetBaseUrl.textContent(), /ark\.cn-beijing\.volces\.com\/api\/v3/);
    await preset.selectOption("custom_openai_compatible");
    assert.equal(await baseUrlMessage.evaluate((element) => element.classList.contains("is-error")), false);

    const focusControls = [
      preset,
      modelInput,
      customBaseUrl,
      page.locator("#provider-api-key"),
      page.locator("#provider-timeout-seconds"),
    ];
    for (const control of focusControls) {
      const focus = await focusPresentation(control);
      assert.ok(
        focus.outlineStyle === "none" || focus.outlineWidth === "0px",
        "Native control should not draw a competing focus outline",
      );
      assert.notEqual(focus.shellShadow, "none", "Input shell should own the focus ring");
      assert.notEqual(focus.shellRadius, "0px", "Focus ring should follow rounded shell");
      for (const [content, pointerEvents] of [
        [focus.beforeContent, focus.beforePointerEvents],
        [focus.afterContent, focus.afterPointerEvents],
      ]) {
        assert.ok(
          content === "none" || content === "normal" || pointerEvents === "none",
          "Decorative focus pseudo-elements must not intercept input",
        );
      }
    }
    await preset.focus();
    await page.keyboard.press("Tab");
    assert.equal(await page.evaluate(() => document.activeElement?.id), "provider-active-model");
    assert.notEqual(
      await modelInput.evaluate((control) => getComputedStyle(control.closest(".input-shell")).boxShadow),
      "none",
      "Keyboard focus should retain the visible shell ring",
    );

    await preset.selectOption("tencent_hunyuan");
    assert.match(
      await presetBaseUrl.textContent(),
      /tokenhub\.tencentmaas\.com\/v1/,
    );
    assert.equal(await modelInput.getAttribute("placeholder"), "hy3-preview");
    assert.match(await page.locator("#provider-active-model-help").textContent(), /hy3-preview/);
    assert.notEqual(await modelInput.inputValue(), "hy3-preview");
    await overrideToggle.check();
    assert.equal(await overrideBaseUrlField.isVisible(), true);
    await overrideBaseUrl.fill("https://gateway.example/v1");
    await page.locator('[data-settings-section="history-privacy"]').click();
    await page.locator('[data-settings-section="ai-provider"]').click();
    assert.equal(await overrideToggle.isChecked(), true);
    assert.equal(await overrideBaseUrl.inputValue(), "https://gateway.example/v1");
    const overrideFocus = await focusPresentation(overrideBaseUrl);
    assert.ok(overrideFocus.outlineStyle === "none" || overrideFocus.outlineWidth === "0px");
    assert.notEqual(overrideFocus.shellShadow, "none");
    await page.locator("#provider-endpoint-reset").click();
    assert.equal(await overrideToggle.isChecked(), false);
    assert.equal(await overrideBaseUrlField.isHidden(), true);

    const effectiveGrid = page.locator(".provider-effective-grid");
    const effectiveRows = effectiveGrid.locator(".provider-status-row");
    assert.equal(await effectiveRows.count(), 4);
    const gridBox = await effectiveGrid.boundingBox();
    assert.ok(gridBox, "Effective configuration grid should be visible");
    for (const row of await effectiveRows.all()) {
      const rowBox = await row.boundingBox();
      assert.ok(rowBox && Math.abs(rowBox.width - gridBox.width) <= 1);
    }
    await page.locator("#provider-effective-base-url").evaluate((element) => {
      element.textContent = `https://gateway.example/${"very-long-provider-path/".repeat(8)}`;
    });
    assert.equal(
      await page.locator("#provider-effective-base-url").evaluate(
        (element) => element.scrollWidth <= element.clientWidth,
      ),
      true,
    );

    await page.setViewportSize({ width: 540, height: 900 });
    const narrowEffectiveRow = effectiveRows.first();
    const [narrowLabel, narrowValue] = await Promise.all([
      narrowEffectiveRow.locator("span").boundingBox(),
      narrowEffectiveRow.locator("strong").boundingBox(),
    ]);
    assert.ok(narrowLabel && narrowValue && narrowValue.y >= narrowLabel.y + narrowLabel.height);
    assert.equal(
      await page.locator(".provider-settings-card").evaluate(
        (element) => element.scrollWidth <= element.clientWidth,
      ),
      true,
    );

    await page.locator('[data-settings-section="general"]').click();
    await page.locator('[data-segment="systemLanguage"] [data-value="Chinese"]').click();
    assert.deepEqual(
      await sectionButtons.evaluateAll((buttons) => buttons.map((button) => button.textContent)),
      ["通用", "语音与输出", "历史与隐私", "AI 服务", "高级设置"],
    );

    console.log("provider settings UI test passed");
  } finally {
    await browser.close();
  }
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
