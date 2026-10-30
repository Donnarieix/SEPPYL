// Minimal dependency-free rich text editor. Persist only sanitized HTML from API.
// Reactivity stays in Dioxus via real <textarea> input events.
(() => {
  if (window.__seppylRichInstalled) return;
  window.__seppylRichInstalled = true;

  function initialize(container) {
    if (container.dataset.richInitialized) return;
    const area = container.querySelector("[data-rich-area]");
    const output = container.querySelector("[data-rich-html]");
    const textOutput = container.querySelector("[data-rich-plain]");
    if (!area || !output || !textOutput) return;
    container.dataset.richInitialized = "true";

    function emit(target, value) {
      if (target.value === value) return;
      target.value = value;
      target.dispatchEvent(new Event("input", { bubbles: true }));
    }
    function sync() {
      emit(output, area.innerHTML);
      emit(textOutput, area.innerText || area.textContent || "");
    }
    area.addEventListener("input", sync);
    area.addEventListener("blur", sync);
    // Strip pasted HTML rather than accepting foreign styles/scripts.
    area.addEventListener("paste", event => {
      event.preventDefault();
      const plain = event.clipboardData?.getData("text/plain") || "";
      document.execCommand("insertText", false, plain);
      sync();
    });
    for (const button of container.querySelectorAll("[data-rich-command]")) {
      button.addEventListener("mousedown", event => event.preventDefault());
      button.addEventListener("click", () => {
        area.focus();
        const command = button.dataset.richCommand;
        if (!["bold", "italic", "underline", "insertUnorderedList",
          "insertOrderedList", "formatBlock", "removeFormat"].includes(command)) return;
        document.execCommand(command, false,
          command === "formatBlock" ? (button.dataset.richValue || "p") : undefined);
        sync();
      });
    }
  }

  function scan() {
    document.querySelectorAll(".seppyl-rich-editor").forEach(initialize);
  }
  scan();
  new MutationObserver(scan).observe(document.documentElement, { childList: true, subtree: true });
})();
