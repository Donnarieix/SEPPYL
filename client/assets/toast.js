// Standard non-blocking status: messages are fixed to the viewport and disappear.
// A new message in the same Dioxus node restarts the lifecycle.
(() => {
  if(window.__seppylToastsInstalled)return;
  window.__seppylToastsInstalled=true;
  const lifetimes=new WeakMap();
  const update=()=>{
    document.querySelectorAll(".seppyl-toast").forEach(node=>{
      const text=node.querySelector(".seppyl-toast-message")?.textContent?.trim()||"";
      const existing=lifetimes.get(node);
      if(existing?.text===text)return;
      if(existing?.timer)clearTimeout(existing.timer);
      node.classList.remove("seppyl-toast-hidden");
      const timer=setTimeout(()=>{
        node.classList.add("seppyl-toast-hidden");
      },4500);
      lifetimes.set(node,{text,timer});
    });
  };
  document.addEventListener("click",event=>{
    if(event.target.closest?.("[data-toast-dismiss]")){
      event.target.closest(".seppyl-toast")?.classList.add("seppyl-toast-hidden");
    }
  });
  const observe=new MutationObserver(update);
  observe.observe(document.documentElement,{childList:true,subtree:true,characterData:true});
  update();
})();
