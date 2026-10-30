// Fullscreen Tactical Studio: retain every command inside the fullscreen root.
// Native Fullscreen API makes Escape and permissions follow browser semantics.
(() => {
  if (window.__seppylTacticalFullscreen) return;
  window.__seppylTacticalFullscreen = true;
  const root = () => document.getElementById("tactics-studio");
  const refresh = () => {
    const studio = root();
    if (!studio) return;
    const active = document.fullscreenElement === studio;
    const button = studio.querySelector("[data-tactics-fullscreen]");
    if (button) {
      button.textContent = active ? "Quitter le plein écran" : "Plein écran";
      button.setAttribute("aria-pressed", String(active));
      button.title = active ? "Quitter le plein écran (Échap)" : "Afficher le studio sur tout l'écran";
    }
  };
  document.addEventListener("click", async event => {
    const button = event.target?.closest?.("[data-tactics-fullscreen]");
    if (!button) return;
    const studio = root();
    if (!studio) return;
    try {
      if (document.fullscreenElement === studio) {
        await document.exitFullscreen();
      } else if (typeof studio.requestFullscreen === "function") {
        await studio.requestFullscreen();
      } else {
        button.title = "Le plein écran n'est pas pris en charge par ce navigateur";
      }
    } catch (error) {
      console.warn("Le plein écran est indisponible", error);
    }
    refresh();
  });
  document.addEventListener("fullscreenchange", refresh);
  document.addEventListener("fullscreenerror", refresh);
})();
