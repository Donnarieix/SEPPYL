// Non-blocking, native in-place formatting. The server sanitizes all saved HTML.
(() => {
  if (window.__seppylRichEditorReady) return;
  window.__seppylRichEditorReady = true;

  function sync(root) {
    const surface = root.querySelector("[data-rich-input]");
    const output = root.querySelector("[data-rich-output]");
    if (!surface || !output) return;
    output.value = surface.innerHTML;
    output.dispatchEvent(new Event("input", { bubbles: true }));
  }

  document.addEventListener("input", event => {
    const surface = event.target.closest?.("[data-rich-input]");
    if (surface) sync(surface.closest("[data-rich-root]"));
  });

  // Preserve the text selection when a formatting button is pressed.
  document.addEventListener("pointerdown", event => {
    if (event.target.closest?.("[data-rich-command]")) event.preventDefault();
  });

  document.addEventListener("click", event => {
    const command = event.target.closest?.("[data-rich-command]");
    if (!command) return;
    const root = command.closest("[data-rich-root]");
    const surface = root?.querySelector("[data-rich-input]");
    if (!surface) return;
    event.preventDefault();
    surface.focus();
    const name = command.getAttribute("data-rich-command");
    const value = command.getAttribute("data-rich-value") || null;
    document.execCommand(name, false, value);
    sync(root);
  });

  document.addEventListener("paste", event => {
    const surface = event.target.closest?.("[data-rich-input]");
    if (!surface) return;
    event.preventDefault();
    const plain = event.clipboardData?.getData("text/plain") || "";
    document.execCommand("insertText", false, plain);
    sync(surface.closest("[data-rich-root]"));
  });
})();
