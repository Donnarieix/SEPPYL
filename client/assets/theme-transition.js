// SEPPYL — one click both selects the theme in Dioxus and reveals it.
// Starting a View Transition on pointerdown interferes with the later click,
// and can prevent the Dioxus handler from persisting the first selection.
(() => {
  if (window.__seppylThemeRevealInstalled) return;
  window.__seppylThemeRevealInstalled = true;

  function setTheme(next) {
    const root = document.querySelector(".site-root");
    if (root) root.dataset.theme = next;
  }

  function reveal(button, x, y) {
    const root = document.querySelector(".site-root");
    if (!root || button.disabled) return;
    const next = button.getAttribute("data-theme-option");
    if (!["system", "light", "dark", "contrast"].includes(next)) return;
    if (root.dataset.theme === next) return;

    // The browser paints; Dioxus owns selected state and persistence.
    const apply = () => setTheme(next);
    if (
      typeof document.startViewTransition !== "function" ||
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      apply();
      return;
    }
    try {
      if (document.activeViewTransition) {
        document.activeViewTransition.skipTransition();
      }
      const transition = document.startViewTransition(apply);
      const radius = Math.hypot(
        Math.max(x, innerWidth - x),
        Math.max(y, innerHeight - y),
      );
      transition.ready.then(() => {
        document.documentElement.animate(
          {
            clipPath: [
              "circle(0px at " + x + "px " + y + "px)",
              "circle(" + radius + "px at " + x + "px " + y + "px)",
            ],
          },
          {
            duration: 540,
            easing: "cubic-bezier(.2,.75,.25,1)",
            pseudoElement: "::view-transition-new(root)",
          },
        );
      }).catch(() => {
        apply(); // A canceled transition must never leave the wrong theme.
      });
    } catch (_) {
      apply();
    }
  }

  // Capture phase sees the native click before Dioxus does, without stopping
  // event propagation or replacing the clicked element.
  document.addEventListener("click", (event) => {
    const button = event.target?.closest?.("[data-theme-option]");
    if (!button) return;
    const rect = button.getBoundingClientRect();
    const keyboard = event.detail === 0;
    const x = keyboard ? rect.left + rect.width / 2 : event.clientX;
    const y = keyboard ? rect.top + rect.height / 2 : event.clientY;
    reveal(button, x, y);
  }, { capture: true });

  // Used when the API rejects a preference save, so the painted theme and
  // Dioxus-selected option both roll back to the previous value.
  window.__seppylRestoreTheme = setTheme;
})();
