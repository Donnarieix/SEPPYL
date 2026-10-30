// Accessible dropdown overlay for desktop. The native select stays in the DOM
// for screen readers, keyboard semantics, form submission and Dioxus signals.
// Touch devices retain their OS picker for usability.
(() => {
  if (window.__seppylSelectOverlayInstalled) return;
  window.__seppylSelectOverlayInstalled = true;
  let active = null, panel = null, options = [], index = -1;
  const coarse = () => window.matchMedia?.("(pointer: coarse)").matches;
  const withinApp = el => !!el?.closest?.(".site-root");
  const selectable = el => el?.matches?.("select:not([multiple]):not([size])") &&
    !el.disabled && withinApp(el) && !coarse();
  const rootFor = el => document.fullscreenElement?.contains(el)
    ? document.fullscreenElement : el.closest(".site-root");
  function close(restoreFocus = false) {
    const previous = active;
    if (previous) previous.removeAttribute("aria-expanded");
    panel?.remove();
    panel = null; active = null; options = []; index = -1;
    if (restoreFocus && previous?.isConnected) previous.focus({preventScroll:true});
  }
  function place() {
    if (!active || !panel || !active.isConnected) { close(); return; }
    const r = active.getBoundingClientRect();
    const spaceBelow = innerHeight - r.bottom - 8;
    const height = Math.min(panel.scrollHeight, 340, innerHeight * .55);
    const below = spaceBelow >= Math.min(170, height) || r.top < height;
    const left = Math.max(8, Math.min(r.left, innerWidth - Math.max(180,r.width) - 8));
    panel.style.left = left + "px";
    panel.style.top = (below ? Math.min(innerHeight-height-8,r.bottom+5) :
      Math.max(8,r.top-height-5)) + "px";
    panel.style.width = Math.min(innerWidth-16,Math.max(180,r.width))+"px";
  }
  function choose(value) {
    if (!active) return;
    const el = active;
    // The value setter is the same one used by the real browser native picker.
    el.value = value;
    el.dispatchEvent(new Event("input",{bubbles:true}));
    el.dispatchEvent(new Event("change",{bubbles:true}));
    close(true);
  }
  function open(el) {
    if (active === el) {close();return;}
    close();
    active = el;
    const host=rootFor(el);
    if (!host) {close();return;}
    panel=document.createElement("div");
    panel.className="seppyl-select-popover";
    panel.setAttribute("role","listbox");
    panel.setAttribute("aria-label",el.getAttribute("aria-label") ||
      el.closest(".field")?.querySelector("label")?.textContent?.trim() || "Choisir une option");
    options = Array.from(el.options).filter(o=>!o.hidden);
    index=Math.max(0,options.findIndex(o=>o.value===el.value));
    if (!options.length) {
      const p=document.createElement("span");p.className="seppyl-select-empty";
      p.textContent="Aucune option";panel.append(p);
    } else for (const [i,option] of options.entries()){
      const item=document.createElement("button");
      item.type="button";
      item.className="seppyl-select-option";
      item.textContent=option.textContent;
      item.setAttribute("role","option");
      item.setAttribute("aria-selected",String(option.value===el.value));
      item.disabled=option.disabled;
      item.tabIndex=i===index?0:-1;
      item.addEventListener("click",()=>choose(option.value));
      panel.append(item);
    }
    host.append(panel);
    place();
    const current=panel.children[index];
    if(current?.scrollIntoView)current.scrollIntoView({block:"nearest"});
    el.setAttribute("aria-expanded","true");
  }
  document.addEventListener("pointerdown",e=>{
    const el=e.target;
    if (selectable(el) && e.button===0) {
      e.preventDefault();
      el.focus({preventScroll:true});
      open(el);
      return;
    }
    if(panel && !panel.contains(el))close();
  },true);
  document.addEventListener("click",e=>{
    if(selectable(e.target))e.preventDefault();
  },true);
  document.addEventListener("keydown",e=>{
    if (panel && active) {
      if (e.key==="Escape"){e.preventDefault();close(true);return;}
      if (e.key==="ArrowDown"||e.key==="ArrowUp"){
        e.preventDefault();
        const step=e.key==="ArrowDown"?1:-1;
        for(let n=0;n<options.length;n++){
          index=(index+step+options.length)%options.length;
          if(!options[index].disabled)break;
        }
        const item=panel.children[index];
        item?.focus({preventScroll:true});
        item?.scrollIntoView?.({block:"nearest"});
        return;
      }
      if (e.key==="Enter"||e.key===" "){
        e.preventDefault();
        if(options[index]&&!options[index].disabled)choose(options[index].value);
        return;
      }
      if (e.key==="Tab"){close();return;}
    }
    if(selectable(e.target) && (e.key==="Enter"||e.key===" ")){
      e.preventDefault();open(e.target);
    }
  },true);
  document.addEventListener("fullscreenchange",()=>close());
  window.addEventListener("resize",()=>{if(panel)place();});
  window.addEventListener("scroll",()=>{if(panel)place();},true);
})();
