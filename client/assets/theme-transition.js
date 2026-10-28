// SEPPYL — browser-only visual effects.
// The real theme and its persistence are owned by Rust/Dioxus.
// We start the visual transition on pointerdown (before Dioxus's click
// handler rerenders), so the browser can capture the OLD theme reliably.
(() => {
  if (window.__seppylThemeRevealInstalled) return;
  window.__seppylThemeRevealInstalled = true;

  let lastPointerChange = 0;

  function beginThemeReveal(button, x, y) {
    const root = document.querySelector(".site-root");
    if (!root || button.disabled) return;
    const next = button.getAttribute("data-theme-option");
    if (!["system", "light", "dark"].includes(next)) return;
    if (root.dataset.theme === next) return;

    const changeTheme = () => {
      // This is immediate and doesn't wait for Rust, the API or any message.
      root.dataset.theme = next;
    };

    if (
      !document.startViewTransition ||
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      changeTheme();
      return;
    }

    // Keep only one theme transition active at a time.
    if (document.activeViewTransition) {
      document.activeViewTransition.skipTransition();
    }

    try {
      // The browser snapshots the old appearance first; the DOM update is
      // synchronous inside this callback, not a cross-language handshake.
      const transition = document.startViewTransition(changeTheme);
      const maxRadius = Math.hypot(
        Math.max(x, innerWidth - x),
        Math.max(y, innerHeight - y),
      );
      transition.ready.then(() => {
        document.documentElement.animate(
          {
            clipPath: [
              "circle(0px at " + x + "px " + y + "px)",
              "circle(" + maxRadius + "px at " + x + "px " + y + "px)",
            ],
          },
          {
            duration: 540,
            easing: "cubic-bezier(.2,.75,.25,1)",
            pseudoElement: "::view-transition-new(root)",
          },
        );
      }).catch(() => {
        // A canceled transition shouldn't interrupt the actual theme change.
        changeTheme();
      });
    } catch (_) {
      changeTheme();
    }
  }

  // Mouse, pen and touch: trigger before the Dioxus click listener updates
  // the reactive state, ensuring the old frame is still visible.
  document.addEventListener("pointerdown", (event) => {
    if (event.button !== 0) return;
    const button = event.target.closest("[data-theme-option]");
    if (!button) return;
    lastPointerChange = performance.now();
    beginThemeReveal(button, event.clientX, event.clientY);
  }, { capture: true });

  // Keyboard and assistive-technology activation has no pointerdown.
  document.addEventListener("click", (event) => {
    if (performance.now() - lastPointerChange < 800) return;
    const button = event.target.closest("[data-theme-option]");
    if (!button) return;
    const rect = button.getBoundingClientRect();
    beginThemeReveal(button, rect.left + rect.width / 2, rect.top + rect.height / 2);
  }, { capture: true });
})();
