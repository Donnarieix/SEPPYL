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
  const toggle = async () => {
    const studio = root();
    if(!studio)return;
    try {
      if(document.fullscreenElement===studio)await document.exitFullscreen();
      else if(typeof studio.requestFullscreen==="function")await studio.requestFullscreen();
    } catch(error) {console.warn("Le plein écran est indisponible",error);}
    refresh();
  };
  document.addEventListener("keydown",event=>{
    const active=document.activeElement;
    if(event.key.toLowerCase()!=="f"||event.repeat||event.ctrlKey||event.metaKey||event.altKey||
       active?.matches?.("input,textarea,select")||active?.isContentEditable||!root())return;
    event.preventDefault();
    void toggle();
  });
  document.addEventListener("click", async event => {
    const button = event.target?.closest?.("[data-tactics-fullscreen]");
    if (!button) return;
    await toggle();
  });
  document.addEventListener("fullscreenchange", refresh);
  document.addEventListener("fullscreenerror", refresh);
})();
