// Lightweight browser-event tests, with no third-party JS dependencies.
// Run with: node --test client/tests/theme-transition.test.cjs
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const test = require("node:test");

const source = fs.readFileSync(
  path.join(__dirname, "../assets/theme-transition.js"),
  "utf8"
);

function harness({ supported = true, reduced = false } = {}) {
  const listeners = new Map();
  const animations = [];
  const snapshots = [];
  const root = { dataset: { theme: "light" } };
  const documentElement = {
    animate(keyframes, options) {
      animations.push({ keyframes, options });
      return { finished: Promise.resolve() };
    },
  };
  const document = {
    documentElement,
    querySelector(selector) { return selector === ".site-root" ? root : null; },
    addEventListener(name, callback) { listeners.set(name, callback); },
    activeViewTransition: null,
  };
  if (supported) {
    document.startViewTransition = callback => {
      const before = root.dataset.theme;
      callback(); // Browser captures old snapshot before updating DOM.
      snapshots.push({ before, after: root.dataset.theme });
      return { ready: Promise.resolve(), skipTransition() {} };
    };
  }
  const window = {
    matchMedia() { return { matches: reduced }; },
    innerWidth: 1000,
    innerHeight: 700,
  };
  const context = {
    document, window,
    innerWidth: 1000, innerHeight: 700,
    performance: { now() { return 1000; } },
    console: { debug() { throw new Error("No browser object logging expected"); } },
  };
  vm.runInNewContext(source, context);
  const button = {
    disabled: false,
    getAttribute(name) { return name === "data-theme-option" ? "dark" : null; },
    getBoundingClientRect() { return { left: 20, top: 20, width: 100, height: 40 }; },
  };
  const target = { closest() { return button; } };
  return { root, listeners, button, target, animations, snapshots };
}

test("pointer transition applies the new theme before any server response", async () => {
  const h = harness();
  h.listeners.get("pointerdown")({
    button: 0, target: h.target, clientX: 150, clientY: 100,
  });
  assert.equal(h.root.dataset.theme, "dark");
  assert.equal(h.snapshots.length, 1);
  assert.equal(h.snapshots[0].before, "light");
  assert.equal(h.snapshots[0].after, "dark");
  await Promise.resolve();
  assert.equal(h.animations.length, 1);
  assert.match(h.animations[0].keyframes.clipPath[0], /circle\(0px at 150px 100px\)/);
  assert.equal(h.animations[0].options.pseudoElement, "::view-transition-new(root)");
});

test("fallback applies immediately on unsupported browsers", () => {
  const h = harness({ supported: false });
  h.listeners.get("pointerdown")({
    button: 0, target: h.target, clientX: 300, clientY: 300,
  });
  assert.equal(h.root.dataset.theme, "dark");
  assert.equal(h.animations.length, 0);
});

test("reduced-motion preference skips animation", () => {
  const h = harness({ reduced: true });
  h.listeners.get("pointerdown")({
    button: 0, target: h.target, clientX: 300, clientY: 300,
  });
  assert.equal(h.root.dataset.theme, "dark");
  assert.equal(h.snapshots.length, 0);
});

test("keyboard-triggered click also changes the theme", () => {
  const h = harness();
  h.listeners.get("click")({ target: h.target });
  assert.equal(h.root.dataset.theme, "dark");
});
