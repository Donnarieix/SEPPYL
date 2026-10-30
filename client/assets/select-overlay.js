// Searchable selection is opt-in (large entities) or automatically enabled
// above a threshold. Finite enumerations keep their compact original menu.
// The native select remains the source of truth for forms and Dioxus signals.
(() => {
  if (window.__seppylSelectOverlayInstalled) return;
  window.__seppylSelectOverlayInstalled = true;
  let active=null, panel=null, list=null, input=null, options=[], visible=[], index=-1;
  const coarse=()=>!!window.matchMedia?.("(pointer: coarse)").matches;
  const withinApp=el=>!!el?.closest?.(".site-root");
  const searchable=el=>el?.getAttribute?.("data-seppyl-searchable")==="true" ||
    Array.from(el?.options||[]).filter(o=>!o.hidden&&o.value!=="").length>=8;
  const selectable=el=>el?.matches?.("select:not([multiple]):not([size])") &&
    !el.disabled && withinApp(el) && (!coarse()||searchable(el));
  const rootFor=el=>document.fullscreenElement?.contains(el)
    ?document.fullscreenElement:el.closest(".site-root");
  const normalize=value=>String(value||"").normalize("NFD")
    .replace(/[\u0300-\u036f]/g,"").toLocaleLowerCase().trim();
  function close(restoreFocus=false){
    const previous=active;
    if(previous)previous.removeAttribute("aria-expanded");
    panel?.remove();
    panel=null;list=null;input=null;active=null;options=[];visible=[];index=-1;
    if(restoreFocus&&previous?.isConnected)previous.focus({preventScroll:true});
  }
  function place(){
    if(!active||!panel||!active.isConnected){close();return;}
    const rect=active.getBoundingClientRect();
    const height=Math.min(panel.scrollHeight,380,innerHeight*.65);
    const below=innerHeight-rect.bottom-8>=Math.min(height,190)||rect.top<height;
    const width=Math.min(innerWidth-16,Math.max(210,rect.width));
    panel.style.left=Math.max(8,Math.min(rect.left,innerWidth-width-8))+"px";
    panel.style.top=(below?Math.min(innerHeight-height-8,rect.bottom+5):
      Math.max(8,rect.top-height-5))+"px";
    panel.style.width=width+"px";
  }
  function choose(option){
    if(!active||!option||option.disabled)return;
    const el=active;
    el.value=option.value;
    el.dispatchEvent(new Event("input",{bubbles:true}));
    el.dispatchEvent(new Event("change",{bubbles:true}));
    close(true);
  }
  function render(){
    if(!panel||!list)return;
    const term=normalize(input?.value);
    visible=options.filter(option=>!term||normalize(option.textContent).includes(term));
    const selected=visible.findIndex(o=>o.value===active.value&&!o.disabled);
    if(index>=visible.length||index<0||visible[index]?.disabled)index=selected>=0?selected:
      visible.findIndex(o=>!o.disabled);
    list.replaceChildren?.();
    // Separate listbox holds only options, never the search field.
    if(!visible.length){
      const empty=document.createElement("span");empty.className="seppyl-select-empty";
      empty.textContent=options.length?"Aucun résultat":"Aucune option";
      list.append(empty);return;
    }
    for(const [i,option] of visible.entries()){
      const button=document.createElement("button");
      button.type="button";button.className="seppyl-select-option";
      button.textContent=option.textContent;
      button.setAttribute("role","option");
      button.setAttribute("aria-selected",String(option.value===active.value));
      button.setAttribute("data-select-index",String(i));
      button.disabled=option.disabled;
      button.tabIndex=-1;
      if(i===index)button.classList.add("is-active");
      button.addEventListener("click",()=>choose(option));
      list.append(button);
    }
  }
  function setIndex(next){
    if(!visible.length)return;
    for(let n=0;n<visible.length;n++){
      const i=(next+n+visible.length)%visible.length;
      if(!visible[i].disabled){index=i;break;}
    }
    Array.from(list.children).forEach((item,i)=>{
      item.classList?.toggle("is-active",i===index);
    });
    list.children[index]?.scrollIntoView?.({block:"nearest"});
    if(input)input.setAttribute("aria-activedescendant",String(index));
  }
  function open(el){
    if(active===el){close();return;}
    close();active=el;
    const host=rootFor(el);
    if(!host){close();return;}
    const isSearch=searchable(el);
    options=Array.from(el.options).filter(o=>!o.hidden&&(!isSearch||o.value!==""));
    index=Math.max(0,options.findIndex(o=>o.value===el.value&&!o.disabled));
    panel=document.createElement("div");panel.className="seppyl-select-popover";
    panel.setAttribute("role",isSearch?"dialog":"listbox");
    panel.setAttribute("aria-label",el.getAttribute("aria-label")||
      el.closest(".field")?.querySelector("label")?.textContent?.trim()||"Choisir une option");
    if(isSearch){
      input=document.createElement("input");
      input.type="search";input.className="seppyl-select-search";
      input.placeholder="Rechercher…";
      input.setAttribute("aria-label","Rechercher une option");
      input.setAttribute("aria-autocomplete","list");
      input.setAttribute("autocomplete","off");
      input.addEventListener("input",()=>{
        index=-1;render();place();
      });
      panel.append(input);
      list=document.createElement("div");list.className="seppyl-select-options";
      list.setAttribute("role","listbox");
      panel.append(list);
    }else{
      list=panel;
    }
    host.append(panel);
    render();place();
    if(isSearch)input.focus({preventScroll:true});
    else list.children[index]?.scrollIntoView?.({block:"nearest"});
    el.setAttribute("aria-expanded","true");
  }
  document.addEventListener("pointerdown",event=>{
    const el=event.target;
    if(event.button===0&&selectable(el)){
      event.preventDefault();
      if(!coarse()||!searchable(el))el.focus({preventScroll:true});
      open(el);return;
    }
    if(panel&&!panel.contains(el))close();
  },true);
  document.addEventListener("click",event=>{
    if(selectable(event.target))event.preventDefault();
  },true);
  document.addEventListener("keydown",event=>{
    if(panel&&active){
      if(event.key==="Escape"){event.preventDefault();close(true);return;}
      if(event.key==="Tab"){close();return;}
      if(event.key==="ArrowDown"||event.key==="ArrowUp"){
        event.preventDefault();setIndex(index+(event.key==="ArrowDown"?1:-1));return;
      }
      if(event.key==="Enter"||(event.key===" "&&!input)){
        // Search input may contain spaces; only Enter submits selection.
        event.preventDefault();choose(visible[index]);return;
      }
    }
    if(selectable(event.target)&&(event.key==="Enter"||event.key===" ")){
      event.preventDefault();open(event.target);
    }
  },true);
  document.addEventListener("fullscreenchange",()=>close());
  window.addEventListener("resize",()=>{if(panel)place();});
  window.addEventListener("scroll",()=>{if(panel)place();},true);
})();
