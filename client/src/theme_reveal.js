// Circular reveal of the new theme from the click coordinates.
const x = Number("__X__");
const y = Number("__Y__");
const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
if (reduced || typeof document.startViewTransition !== "function") {
  dioxus.send("apply");
} else {
  const transition = document.startViewTransition(() => {
    dioxus.send("apply");
    return new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  });
  try {
    await transition.ready;
    const radius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
    await document.documentElement.animate(
      {clipPath: ["circle(0px at " + x + "px " + y + "px)", "circle(" + radius + "px at " + x + "px " + y + "px)"]},
      {duration: 650, easing: "cubic-bezier(0.22, 1, 0.36, 1)", pseudoElement: "::view-transition-new(root)"}
    ).finished;
  } catch (error) {
    console.debug("SEPPYL theme reveal fallback", error);
  }
}
dioxus.send("finished");
