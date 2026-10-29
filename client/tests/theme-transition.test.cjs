// Browser-event contract tests for the single-click theme change.
// Run: node --test client/tests/theme-transition.test.cjs
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const test = require("node:test");

const source = fs.readFileSync(path.join(__dirname, "../assets/theme-transition.js"), "utf8");

function harness({ supported = true, reduced = false, oldTheme = "light" } = {}) {
  const listeners = new Map();
  const animations = [];
  const snapshots = [];
  const root = { dataset: { theme: oldTheme } };
  const document = {
    documentElement: {
      animate(keyframes, options) {
        animations.push({ keyframes, options });
        return { finished: Promise.resolve() };
      },
    },
    querySelector(selector) { return selector === ".site-root" ? root : null; },
    addEventListener(name, callback) { listeners.set(name, callback); },
    activeViewTransition: null,
  };
  if (supported) {
    document.startViewTransition = callback => {
      const before = root.dataset.theme;
      callback();
      snapshots.push({ before, after: root.dataset.theme });
      return { ready: Promise.resolve(), skipTransition() {} };
    };
  }
  const window = {
    matchMedia() { return { matches: reduced }; },
    innerWidth: 1000,
    innerHeight: 700,
  };
  const context = { document, window, innerWidth: 1000, innerHeight: 700 };
  vm.runInNewContext(source, context);

  let rustOnClickCalls = 0;
  let savedChoice = oldTheme;
  let selectedChoice = oldTheme;
  const button = {
    disabled: false,
    getAttribute(name) { return name === "data-theme-option" ? "dark" : null; },
    getBoundingClientRect() { return { left: 20, top: 20, width: 100, height: 40 }; },
  };
  const target = { closest() { return button; } };

  // Native event propagation: document capture listener runs before the
  // framework's button click handler. Starting a transition in pointerdown
  // previously prevented the click from reaching the component.
  function click({ keyboard = false } = {}) {
    const event = {
      target, detail: keyboard ? 0 : 1, clientX: 150, clientY: 100,
      preventDefault() { throw new Error("Theme reveal must not cancel click"); },
      stopPropagation() { throw new Error("Theme reveal must not stop click"); },
      stopImmediatePropagation() { throw new Error("Theme reveal must not stop click"); },
    };
    listeners.get("click")(event);
    rustOnClickCalls++;
    savedChoice = "dark";
    selectedChoice = "dark";
  }

  return {
    root, listeners, button, target, animations, snapshots, window, click,
    get selectedChoice() { return selectedChoice; },
    get savedChoice() { return savedChoice; },
    get rustOnClickCalls() { return rustOnClickCalls; },
  };
}

test("one click selects the option, saves it, and starts the circular reveal", async () => {
  const h = harness();
  assert.equal(h.listeners.has("pointerdown"), false);
  h.click();
  assert.equal(h.rustOnClickCalls, 1);
  assert.equal(h.selectedChoice, "dark");
  assert.equal(h.savedChoice, "dark");
  assert.equal(h.root.dataset.theme, "dark");
  assert.equal(h.snapshots.length, 1);
  assert.equal(h.snapshots[0].before, "light");
  assert.equal(h.snapshots[0].after, "dark");
  await Promise.resolve();
  assert.equal(h.animations.length, 1);
  assert.match(h.animations[0].keyframes.clipPath[0], /circle\(0px at 150px 100px\)/);
  assert.equal(h.animations[0].options.pseudoElement, "::view-transition-new(root)");
});

test("unsupported browser still changes and selects with one click", () => {
  const h = harness({ supported: false });
  h.click();
  assert.equal(h.root.dataset.theme, "dark");
  assert.equal(h.selectedChoice, "dark");
  assert.equal(h.rustOnClickCalls, 1);
  assert.equal(h.animations.length, 0);
});

test("reduced-motion avoids animation but preserves one-click selection", () => {
  const h = harness({ reduced: true });
  h.click();
  assert.equal(h.root.dataset.theme, "dark");
  assert.equal(h.selectedChoice, "dark");
  assert.equal(h.snapshots.length, 0);
});

test("keyboard click selects and reveals from the button center", async () => {
  const h = harness();
  h.click({ keyboard: true });
  assert.equal(h.selectedChoice, "dark");
  await Promise.resolve();
  assert.match(h.animations[0].keyframes.clipPath[0], /circle\(0px at 70px 40px\)/);
});

test("failed API persistence can restore the old painted theme", () => {
  const h = harness();
  h.click();
  h.window.__seppylRestoreTheme("light");
  assert.equal(h.root.dataset.theme, "light");
});
