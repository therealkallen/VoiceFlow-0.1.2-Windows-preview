const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");

// Unit-level DOM doubles; live visual/browser verification is a separate check.
class Element {
  constructor(tag) { this.tag = tag; this.children = []; this.events = {}; this.hidden = false; this.value = ""; this.classList = { add() {}, remove() {} }; }
  append(...nodes) { nodes.forEach(n => { n.parent = this; this.children.push(n); }); }
  replaceChildren(...nodes) { this.children = []; this.append(...nodes); }
  setAttribute() {}
  addEventListener(type, callback) { this.events[type] = callback; }
  click() { if (!this.disabled) return this.events.click?.(); }
  focus() {} showModal() {} close() {}
  remove() { this.parent.children = this.parent.children.filter(n => n !== this); }
}
function fixture() {
  const source = fs.readFileSync(`${__dirname}/main.js`, "utf8");
  const fn = source.slice(source.indexOf("function openPromptEditor(slot)"), source.indexOf("function promptOverrideSlotLabel(slot)"));
  const body = new Element("body"), calls = [], slot = { slot_id: "dictation_light_cleanup" };
  let stored = "builtin prompt", fail = false;
  const context = {
    document: { body, createElement: tag => new Element(tag) },
    window: { addEventListener() {}, removeEventListener() {}, __VOICEFLOW_SETTINGS_RUNTIME__: { prompt_overrides: [slot] } },
    currentLanguage: () => "English", promptOverrideSlotLabel: () => "Light cleanup", renderPromptOverrides() {},
    getSettingsControlUrl: () => "http://localhost:1", buildMutationHeaders: () => ({ "X-VoiceFlow-Control-Token": "test" }),
    TextEncoder, AbortController, setTimeout, clearTimeout,
    fetch: async (_, opts) => {
      const request = JSON.parse(opts.body); calls.push(request);
      if (fail) throw new Error("offline");
      if (request.action === "save") stored = request.text;
      if (request.action === "reset") stored = "builtin prompt";
      return { ok: true, json: async () => ({ text: stored, builtin_text: "builtin prompt", source: stored === "builtin prompt" ? "builtin" : "custom", max_bytes: 32000 }) };
    },
  };
  vm.runInNewContext(fn + "\nopenPromptEditor({slot_id:'dictation_light_cleanup'});", context);
  const nodes = () => { const all = []; const walk = n => { all.push(n); n.children.forEach(walk); }; walk(body); return all; };
  return { calls, body, nodes, fail: () => { fail = true; }, find: text => nodes().find(n => n.textContent === text), input: () => nodes().find(n => n.tag === "textarea") };
}
const settle = () => new Promise(resolve => setImmediate(resolve));

test("warning gates access; edits require save; reset confirmation only stages default", async () => {
  const f = fixture();
  assert.equal(f.calls.length, 0); assert.equal(f.input().hidden, true);
  await f.find("I understand — edit").click();
  assert.equal(f.calls[0].action, "read"); assert.equal(f.input().hidden, false);
  f.input().value = "custom prompt"; f.input().events.input();
  assert.equal(f.calls.length, 1);
  f.find("Save prompt").click(); await settle();
  assert.equal(f.calls[1].text, "custom prompt");
  f.find("Reset to default").click();
  assert.equal(f.input().value, "custom prompt");
  f.find("Confirm").click();
  assert.equal(f.input().value, "builtin prompt"); assert.equal(f.calls.length, 2);
  f.find("Save prompt").click(); await settle();
  assert.equal(f.calls[2].action, "reset");
  f.find("Close").click(); assert.equal(f.body.children.length, 0);
});
test("empty input never writes and network errors preserve drafts", async () => {
  const f = fixture(); await f.find("I understand — edit").click();
  f.input().value = " "; f.input().events.input(); f.find("Save prompt").click(); await settle();
  assert.equal(f.calls.length, 1);
  f.input().value = "keep draft"; f.input().events.input(); f.fail();
  f.find("Save prompt").click(); await settle(); assert.equal(f.input().value, "keep draft");
  f.find("Close").click(); assert.equal(f.body.children.length, 1);
  f.find("Confirm").click(); assert.equal(f.body.children.length, 0);
});
