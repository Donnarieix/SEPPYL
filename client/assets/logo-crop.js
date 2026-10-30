// The framed image is a CSS transform, not a canvas export.
// HTTPS images work regardless of their CORS headers.
(() => {
  if(window.__seppylLogoCropReady)return;
  window.__seppylLogoCropReady=true;
  let gesture=null;
  document.addEventListener("pointerdown",event=>{
    const stage=event.target.closest?.("[data-logo-crop-stage]");
    if(!stage||event.button!==0)return;
    const dialog=stage.closest(".logo-crop-dialog");
    const x=dialog?.querySelector("[data-logo-crop-x]");
    const y=dialog?.querySelector("[data-logo-crop-y]");
    if(!x||!y)return;
    gesture={stage,dialog,startX:event.clientX,startY:event.clientY,
      x:Number(x.value),y:Number(y.value),xRange:x,yRange:y,pointer:event.pointerId};
    stage.setPointerCapture?.(event.pointerId);
    event.preventDefault();
  });
  document.addEventListener("pointermove",event=>{
    if(!gesture||event.pointerId!==gesture.pointer)return;
    const width=Math.max(1,gesture.stage.getBoundingClientRect().width);
    const height=Math.max(1,gesture.stage.getBoundingClientRect().height);
    const set=(element,value)=>{
      element.value=String(Math.max(-100,Math.min(100,Math.round(value))));
      element.dispatchEvent(new Event("input",{bubbles:true}));
    };
    set(gesture.xRange,gesture.x+(event.clientX-gesture.startX)*100/width);
    set(gesture.yRange,gesture.y+(event.clientY-gesture.startY)*100/height);
  });
  const end=event=>{if(!gesture)return;
    if(event.pointerId===undefined||event.pointerId===gesture.pointer)gesture=null;};
  document.addEventListener("pointerup",end);
  document.addEventListener("pointercancel",end);
  document.addEventListener("wheel",event=>{
    const stage=event.target.closest?.("[data-logo-crop-stage]");
    if(!stage)return;
    const slider=stage.closest(".logo-crop-dialog")?.querySelector("[data-logo-crop-zoom]");
    if(!slider)return;
    event.preventDefault();
    slider.value=String(Math.max(100,Math.min(300,Number(slider.value)+(event.deltaY<0?5:-5))));
    slider.dispatchEvent(new Event("input",{bubbles:true}));
  },{passive:false});
})();
