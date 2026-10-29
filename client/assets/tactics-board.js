// Native SEPPYL tactical canvas, independent of external services.
// Coordinates are normalized so drawings remain editable on tablet/phone.
(() => {
  if (window.__seppylTacticsReady) return;
  window.__seppylTacticsReady = true;
  const validKinds = ["player", "opponent", "ball", "cone"];

  function mount(root) {
    if (root.dataset.mounted) return;
    root.dataset.mounted = "true";
    const canvas = root.querySelector("[data-tactic-canvas]");
    const output = root.querySelector("[data-tactic-output]");
    if (!canvas || !output) return;
    let drawing;
    try { drawing = JSON.parse(root.getAttribute("data-initial") || ""); }
    catch (_) { drawing = null; }
    if (!drawing || !Array.isArray(drawing.markers) || !Array.isArray(drawing.arrows))
      drawing = { pitch: "football", markers: [], arrows: [] };
    let tool = "player";
    let drag = null;
    let pointerStart = null;
    const ctx = canvas.getContext("2d");

    const pos = event => {
      const rect = canvas.getBoundingClientRect();
      return {
        x: Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)),
        y: Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height)),
      };
    };
    const same = (a, b) => Math.hypot(a.x - b.x, a.y - b.y) < .035;
    const nearest = at => drawing.markers.findIndex(m => same(m, at));
    function commit() {
      output.value = JSON.stringify(drawing);
      output.dispatchEvent(new Event("input", { bubbles: true }));
    }
    function line(x1, y1, x2, y2) {
      ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke();
    }
    function arrow(a) {
      const angle = Math.atan2(a.y2-a.y1, a.x2-a.x1);
      ctx.strokeStyle="#ffe095";ctx.fillStyle="#ffe095";ctx.lineWidth=.005;
      line(a.x1,a.y1,a.x2,a.y2);
      ctx.beginPath();ctx.moveTo(a.x2,a.y2);
      ctx.lineTo(a.x2-.022*Math.cos(angle-.5),a.y2-.022*Math.sin(angle-.5));
      ctx.lineTo(a.x2-.022*Math.cos(angle+.5),a.y2-.022*Math.sin(angle+.5));
      ctx.closePath();ctx.fill();
    }
    function pitch() {
      ctx.fillStyle = drawing.pitch === "blank" ? "#f0f2f4" : "#39754b";
      ctx.fillRect(0,0,1,1);
      if (drawing.pitch === "blank") return;
      ctx.strokeStyle = "#ffffffc0";ctx.lineWidth=.003;
      ctx.strokeRect(.025,.025,.95,.95);
      line(.5,.025,.5,.975);
      ctx.beginPath();ctx.ellipse(.5,.5,.11,.17,0,0,Math.PI*2);ctx.stroke();
      ctx.strokeRect(.025,.3,.16,.4);ctx.strokeRect(.815,.3,.16,.4);
      ctx.strokeRect(.025,.41,.05,.18);ctx.strokeRect(.925,.41,.05,.18);
    }
    function render() {
      const rect=canvas.getBoundingClientRect();
      if (!rect.width) return;
      const ratio = Math.min(window.devicePixelRatio || 1,2);
      const w=Math.floor(rect.width*ratio),h=Math.floor(rect.height*ratio);
      if(canvas.width!==w||canvas.height!==h){canvas.width=w;canvas.height=h}
      ctx.setTransform(canvas.width,0,0,canvas.height,0,0);
      pitch();
      for(const a of drawing.arrows) arrow(a);
      for(const m of drawing.markers) {
        ctx.save();
        ctx.translate(m.x,m.y);
        if(m.kind==="cone"){
          ctx.fillStyle="#f6a44c";ctx.beginPath();
          ctx.moveTo(0,-.024);ctx.lineTo(.02,.022);ctx.lineTo(-.02,.022);ctx.closePath();ctx.fill();
        } else {
          ctx.beginPath();ctx.arc(0,0,m.kind==="ball"?.017:.026,0,Math.PI*2);
          ctx.fillStyle=m.kind==="player"?"#2259bb":m.kind==="opponent"?"#e44848":"#fff";
          ctx.fill();ctx.strokeStyle="#fff";ctx.lineWidth=.003;ctx.stroke();
          if(m.kind!=="ball"){ctx.fillStyle="#fff";ctx.font="bold .024px sans-serif";ctx.textAlign="center";ctx.textBaseline="middle";ctx.fillText("●",0,0);}
        }
        ctx.restore();
      }
    }

    canvas.addEventListener("pointerdown", event => {
      if(event.button!==0)return;
      canvas.setPointerCapture(event.pointerId);
      pointerStart=pos(event);
      if(tool==="select"){
        const index=nearest(pointerStart);
        if(index>=0)drag=index;
      }
    });
    canvas.addEventListener("pointermove", event => {
      if(drag===null)return;
      const p=pos(event);drawing.markers[drag].x=p.x;drawing.markers[drag].y=p.y;
      render();
    });
    canvas.addEventListener("pointerup", event => {
      if(!pointerStart)return;
      const end=pos(event);
      if(tool==="arrow" && Math.hypot(end.x-pointerStart.x,end.y-pointerStart.y)>.025)
        drawing.arrows.push({x1:pointerStart.x,y1:pointerStart.y,x2:end.x,y2:end.y});
      else if(tool==="erase"){
        const index=nearest(end);
        if(index>=0)drawing.markers.splice(index,1);
        else if(drawing.arrows.length)drawing.arrows.pop();
      } else if(validKinds.includes(tool))
        drawing.markers.push({kind:tool,x:end.x,y:end.y});
      pointerStart=null;drag=null;render();commit();
    });
    root.addEventListener("click",event=>{
      const button=event.target.closest?.("[data-tactic-tool]");
      if(button){
        tool=button.getAttribute("data-tactic-tool");
        root.querySelectorAll("[data-tactic-tool]").forEach(b=>b.classList.toggle("active",b===button));
      }
      if(event.target.closest?.("[data-tactic-clear]")){
        if(window.confirm("Effacer tous les éléments de ce schéma ?")){
          drawing.markers=[];drawing.arrows=[];commit();render();
        }
      }
      if(event.target.closest?.("[data-tactic-export]")){
        const a=document.createElement("a");
        a.download="seppyl-tactique.png";a.href=canvas.toDataURL("image/png");
        document.body.appendChild(a);a.click();a.remove();
      }
    });
    root.querySelector("[data-tactic-pitch]")?.addEventListener("change",event=>{
      drawing.pitch=event.target.value;commit();render();
    });
    const observer=new ResizeObserver(render);observer.observe(canvas);
    commit();render();
  }
  function scan(){document.querySelectorAll("[data-tactics-editor]").forEach(mount)}
  new MutationObserver(scan).observe(document.documentElement,{childList:true,subtree:true});
  if(document.readyState==="loading")document.addEventListener("DOMContentLoaded",scan);
  else scan();
})();
