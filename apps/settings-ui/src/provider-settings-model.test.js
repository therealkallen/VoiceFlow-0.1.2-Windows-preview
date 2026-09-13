const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const model = require("./provider-settings-model.js");

test("defines the built-in provider endpoints and keeps Custom explicit", () => {
  assert.equal(
    model.presets.volcengine_ark.baseUrl,
    "https://ark.cn-beijing.volces.com/api/v3",
  );
  assert.equal(
    model.presets.tencent_hunyuan.baseUrl,
    "https://tokenhub.tencentmaas.com/v1",
  );
  assert.equal(model.presets.tencent_hunyuan.modelPlaceholder, "hy3-preview");
  assert.equal(model.presets.custom_openai_compatible.baseUrl, null);
});

test("normalizes all supported provider IDs without changing unknown fallback", () => {
  assert.equal(model.normalizePreset("tencent-hunyuan"), "tencent_hunyuan");
  assert.equal(model.normalizePreset("volcengine_ark"), "volcengine_ark");
  assert.equal(model.normalizePreset("unknown"), "bailian");
});

test("shows preset endpoints in Advanced and Custom Base URL in the main form", () => {
  const ark = model.presentation("volcengine_ark", "", false);
  assert.equal(ark.showCustomBaseUrl, false);
  assert.equal(ark.showPresetEndpoint, true);
  assert.equal(ark.showOverrideBaseUrl, false);

  const tencentOverride = model.presentation(
    "tencent_hunyuan",
    "https://gateway.example/v1",
    true,
  );
  assert.equal(tencentOverride.showPresetEndpoint, true);
  assert.equal(tencentOverride.showOverrideBaseUrl, true);

  const custom = model.presentation("custom_openai_compatible", "", false);
  assert.equal(custom.showCustomBaseUrl, true);
  assert.equal(custom.showPresetEndpoint, false);
});

test("recognizes stored preset URLs and enables only real custom overrides", () => {
  assert.equal(
    model.shouldEnableEndpointOverride(
      "volcengine_ark",
      "https://ark.cn-beijing.volces.com/api/v3/",
    ),
    false,
  );
  assert.equal(
    model.shouldEnableEndpointOverride(
      "tencent_hunyuan",
      "https://gateway.example/v1",
    ),
    true,
  );
  assert.equal(
    model.shouldEnableEndpointOverride(
      "custom_openai_compatible",
      "https://gateway.example/v1",
    ),
    false,
  );

  assert.deepEqual(
    model.setEndpointOverride(
      "tencent_hunyuan",
      "https://tokenhub.tencentmaas.com/v1",
      true,
    ),
    { enabled: true, baseUrl: "" },
  );
  assert.deepEqual(
    model.setEndpointOverride(
      "tencent_hunyuan",
      "https://gateway.example/v1",
      false,
    ),
    { enabled: false, baseUrl: "" },
  );
  assert.equal(
    model.presentation("tencent_hunyuan", "", false).presetBaseUrl,
    "https://tokenhub.tencentmaas.com/v1",
  );
});

test("Settings owns English and Chinese provider labels", () => {
  const mainSource = fs.readFileSync(path.join(__dirname, "main.js"), "utf8");
  const htmlSource = fs.readFileSync(path.join(__dirname, "..", "index.html"), "utf8");

  for (const label of [
    "Alibaba Cloud Bailian",
    "Volcengine Ark",
    "Tencent Hunyuan",
    "Custom OpenAI-compatible",
    "阿里云百炼",
    "火山方舟",
    "腾讯混元",
    "自定义 OpenAI 兼容服务",
  ]) {
    assert.match(mainSource, new RegExp(label));
  }
  assert.match(htmlSource, /value="tencent_hunyuan"/);
  assert.match(mainSource, /Model ID, for example hy3-preview\./);
  assert.match(mainSource, /模型 ID，例如 hy3-preview。/);
});
