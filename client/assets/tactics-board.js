// Built-in SVG tactical board. No external embed, no unsafe SVG markup.
// Pointer tools: selection (click/shift-click/marquee), grouped dragging,
// placement, erase and two-click arrows. Color is stored per object.
(() => {
  if (window.__seppylTacticsInstalled) return;
  window.__seppylTacticsInstalled = true;
  const NS = "http://www.w3.org/2000/svg";
  const TOOLS = new Set(["select", "player", "opponent", "ball", "cone", "goal", "arrow", "curve", "erase"]);
  const DEFAULTS = {
    player: "#2455b6", opponent: "#c6403c",
    ball: "#f8f8f5", cone: "#ee9228", arrow: "#202f43"
  };
  const validColor = value => typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value);
  const clamp = (v, max) => Math.min(max, Math.max(0, v));
  const make = (tag, attributes = {}) => {
    const item = document.createElementNS(NS, tag);
    for (const [name, value] of Object.entries(attributes)) item.setAttribute(name, String(value));
    return item;
  };
  const add = (root, tag, attrs = {}) => {
    const item = make(tag, attrs);
    root.append(item);
    return item;
  };
  const inRect = (point, box) => (
    point.x >= box.left && point.x <= box.right &&
    point.y >= box.top && point.y <= box.bottom
  );
  const rectangle = (a, b) => ({
    left: Math.min(a.x, b.x), top: Math.min(a.y, b.y),
    right: Math.max(a.x, b.x), bottom: Math.max(a.y, b.y)
  });

  function setup(root) {
    if (root.dataset.tacticsInitialized) {
      if (root.__tacticsRevision !== root.dataset.tacticsRevision)
        root.__tacticsReload?.();
      return;
    }
    const svg = root.querySelector("[data-tactics-svg]");
    const output = root.querySelector("[data-tactics-output]");
    if (!svg || !output) return;
    root.dataset.tacticsInitialized = "true";
    let objects;
    try { objects = JSON.parse(root.dataset.tacticsInitial || "[]"); }
    catch (_) { objects = []; }
    if (!Array.isArray(objects)) objects = [];
    let tool = "select";
    let selected = new Set();
    let dragging = null;
    let marquee = null;
    let arrowStart = null;
    let color = DEFAULTS.player;
    let brushSize=22;
    let draggingCurve=null;

    const position = event => {
      const point = svg.createSVGPoint();
      point.x = event.clientX; point.y = event.clientY;
      const matrix = svg.getScreenCTM();
      if (!matrix) return null;
      const transformed = point.matrixTransform(matrix.inverse());
      return { x: Math.round(clamp(transformed.x, 800)), y: Math.round(clamp(transformed.y, 500)) };
    };
    const sync = () => {
      output.value = JSON.stringify(objects);
      output.dispatchEvent(new Event("input", { bubbles: true }));
    };
    const activate = next => {
      tool = next;
      arrowStart = null;
      for (const button of root.querySelectorAll("[data-tactic-tool]")) {
        button.classList.toggle("selected", button.dataset.tacticTool === tool);
        button.setAttribute("aria-pressed", String(button.dataset.tacticTool === tool));
      }
      render();
    };
    const paint = next => {
      if (!validColor(next)) return;
      color = next;
      for (const item of objects) {
        if (selected.has(item.id)) item.color = next;
      }
      for (const swatch of root.querySelectorAll("[data-tactic-color]")) {
        swatch.classList.toggle("selected", swatch.dataset.tacticColor.toLowerCase() === color.toLowerCase());
      }
      const custom = root.querySelector("[data-tactic-custom]");
      if (custom) custom.value = next;
      if (selected.size) { sync(); render(); }
    };
    const field = () => {
      add(svg, "rect", { x: 0, y: 0, width: 800, height: 500, fill: "#4b8a62" });
      const line = { stroke: "#ffffffbb", "stroke-width": 3, fill: "none", "pointer-events": "none" };
      add(svg, "rect", { x: 35, y: 35, width: 730, height: 430, ...line });
      add(svg, "line", { x1: 400, y1: 35, x2: 400, y2: 465, ...line });
      add(svg, "circle", { cx: 400, cy: 250, r: 70, ...line });
      add(svg, "rect", { x: 35, y: 140, width: 110, height: 220, ...line });
      add(svg, "rect", { x: 655, y: 140, width: 110, height: 220, ...line });
    };
    const drawArrow = (group, item, fill, active) => {
      const x1=item.x,y1=item.y,x2=item.end_x??x1,y2=item.end_y??y1;
      const curve=item.kind==="curve";
      const cx=item.control_x??(x1+x2)/2,cy=item.control_y??(y1+y2)/2;
      const dx=curve?x2-cx:x2-x1,dy=curve?y2-cy:y2-y1;
      const length=Math.hypot(dx,dy)||1,ux=dx/length,uy=dy/length;
      const head=Math.min(27,Math.max(11,(item.size??20)*.7));
      const bx=x2-ux*head*.65,by=y2-uy*head*.65;
      const path=curve
        ?"M"+x1+" "+y1+" Q"+cx+" "+cy+" "+bx+" "+by
        :"M"+x1+" "+y1+" L"+bx+" "+by;
      add(group,"path",{d:path,stroke:fill,fill:"none",
        "stroke-width":Math.max(3,(item.size??20)*.23),"stroke-linecap":"round",
        "pointer-events":"stroke"});
      add(group,"polygon",{points:[
        [x2,y2],[x2-ux*head-uy*head*.5,y2-uy*head+ux*head*.5],
        [x2-ux*head+uy*head*.5,y2-uy*head-ux*head*.5]
      ].map(p=>p.join(",")).join(" "),fill,
        stroke:active?"#ffe59a":fill,"stroke-width":active?2:0});
      if(curve&&active){
        add(group,"line",{x1:(x1+x2)/2,y1:(y1+y2)/2,x2:cx,y2:cy,
          stroke:"#ffffff99","stroke-width":2,"stroke-dasharray":"5 4",
          "pointer-events":"none"});
        add(group,"circle",{cx,cy,r:10,fill:"#ffefaf",stroke:"#253243",
          "stroke-width":2,cursor:"move","data-curve-handle":item.id});
      }
    };
    function render() {
      svg.replaceChildren();
      field();
      for (const item of objects) {
        const active = selected.has(item.id);
        const group = add(svg, "g", { "data-marker-id": item.id, cursor: "grab" });
        const fill = validColor(item.color) ? item.color : DEFAULTS[item.kind] || "#2455b6";
        const r=item.size??(item.kind==="ball"?16:22);
        if (item.kind==="arrow"||item.kind==="curve") {
          drawArrow(group,item,fill,active);continue;
        }
        if (item.kind==="cone") {
          add(group,"polygon",{points:item.x+","+(item.y-r)+" "+
            (item.x-r*.8)+","+(item.y+r*.75)+" "+(item.x+r*.8)+","+(item.y+r*.75),
            fill,stroke:active?"#ffe59a":"#433f28","stroke-width":active?4:2});
        }else if(item.kind==="goal"){
          add(group,"rect",{x:item.x-r,y:item.y-r*.6,width:2*r,height:r*1.2,
            rx:2,fill:"#ffffff22",stroke:fill,"stroke-width":active?5:3});
          for(let i=-2;i<=2;i++)
            add(group,"line",{x1:item.x+i*r*.35,y1:item.y-r*.6,
              x2:item.x+i*r*.35,y2:item.y+r*.6,stroke:fill,
              "stroke-width":1,"pointer-events":"none"});
        }else {
          add(group,"circle",{cx:item.x,cy:item.y,r,fill:item.kind==="ball"?"#f9f9f6":fill,
            stroke:active?"#ffe59a":"#253243","stroke-width":active?5:2});
          if(item.kind==="ball"){
            const pentagon=(x,y,radius)=>{
              const coords=[];
              for(let i=0;i<5;i++){
                const a=-Math.PI/2+i*2*Math.PI/5;
                coords.push((x+Math.cos(a)*radius)+","+(y+Math.sin(a)*radius));
              }
              add(group,"polygon",{points:coords.join(" "),fill:"#24303a","pointer-events":"none"});
            };
            pentagon(item.x,item.y,r*.42);
            for(let i=0;i<5;i++){
              const a=-Math.PI/2+i*2*Math.PI/5;
              const x=item.x+Math.cos(a)*r*.76,y=item.y+Math.sin(a)*r*.76;
              pentagon(x,y,r*.21);
              add(group,"line",{x1:item.x+Math.cos(a)*r*.4,y1:item.y+Math.sin(a)*r*.4,
                x2:x,y2:y,stroke:"#24303a","stroke-width":1,
                "pointer-events":"none"});
            }
          }
        }
        if (item.label) {
          const text = add(group, "text", {
            x: item.x, y: item.y + 4, fill: "#ffffff",
            "text-anchor": "middle", "font-size": 12,
            "font-weight": 700, "pointer-events": "none"
          });
          text.textContent = item.label;
        }
      }
      if (arrowStart) {
        add(svg, "circle", {
          cx: arrowStart.x, cy: arrowStart.y, r: 7,
          fill: "#fff6b0", "pointer-events": "none"
        });
      }
      if (marquee) {
        const box = rectangle(marquee.start, marquee.end);
        add(svg, "rect", {
          x: box.left, y: box.top, width: box.right - box.left,
          height: box.bottom - box.top, fill: "#b6d6ff44",
          stroke: "#f8f8f5", "stroke-dasharray": "8 5", "stroke-width": 2,
          "pointer-events": "none"
        });
      }
      const count = root.querySelector("[data-selection-count]");
      if (count) count.textContent = selected.size + " sélectionné(s)";
    }
    function dropSelection() {
      objects = objects.filter(item => !selected.has(item.id));
      selected.clear();
      render(); sync();
    }
    function loadDiagram() {
      let loaded=[];
      try {
        const decoded=JSON.parse(root.dataset.tacticsInitial||"[]");
        if(Array.isArray(decoded))loaded=decoded;
      }catch(_){loaded=[];}
      objects=loaded;
      selected.clear();dragging=null;lineStart=null;draggingCurve=null;
      root.__tacticsRevision=root.dataset.tacticsRevision;
      render();
    }
    root.__tacticsReload=loadDiagram;
    root.querySelectorAll("[data-tactic-tool]").forEach(button => {
      button.addEventListener("click", () => {
        if (TOOLS.has(button.dataset.tacticTool)) activate(button.dataset.tacticTool);
      });
    });
    root.querySelectorAll("[data-tactic-color]").forEach(button =>
      button.addEventListener("click", () => paint(button.dataset.tacticColor))
    );
    root.querySelector("[data-tactic-custom]")?.addEventListener("input", event => paint(event.target.value));
    root.querySelector("[data-tactic-remove]")?.addEventListener("click", dropSelection);
    const resize = value => {
      const n=Number(value);
      if(!Number.isFinite(n))return;
      brushSize=Math.min(65,Math.max(8,n));
      for(const item of objects)if(selected.has(item.id))item.size=brushSize;
      const label=root.querySelector("[data-tactic-size-label]");
      if(label)label.textContent=String(Math.round(brushSize));
      if(selected.size)sync();
      render();
    };
    root.querySelector("[data-tactic-size]")?.addEventListener("input",event=>resize(event.target.value));

    root.addEventListener("keydown", event => {
      if (event.target.matches("input, textarea, select")) return;
      if (event.key === "Escape") {
        selected.clear(); marquee = null; arrowStart = null; render();
      } else if (event.key === "Delete" || event.key === "Backspace") {
        event.preventDefault(); dropSelection();
      }
    });
    svg.addEventListener("pointerdown", event => {
      if (event.button !== 0) return;
      const p = position(event);
      if (!p) return;
      const handle=event.target.closest("[data-curve-handle]");
      if(handle&&tool==="select"){
        draggingCurve=handle.getAttribute("data-curve-handle");return;
      }
      const marked = event.target.closest("[data-marker-id]");
      const hitId = marked?.getAttribute("data-marker-id");
      svg.setPointerCapture(event.pointerId);
      if (tool === "erase" && hitId) {
        selected = new Set([hitId]); dropSelection(); return;
      }
      if (tool === "select") {
        if (hitId) {
          if (event.shiftKey) {
            if (selected.has(hitId)) selected.delete(hitId);
            else selected.add(hitId);
            render(); return;
          }
          if (!selected.has(hitId)) selected = new Set([hitId]);
          dragging = {
            start: p, pointerId: event.pointerId,
            original: objects.filter(item => selected.has(item.id)).map(item => ({ ...item }))
          };
          render();
        } else {
          if (!event.shiftKey) selected.clear();
          marquee = { start: p, end: p, addTo: new Set(selected) };
          render();
        }
        return;
      }
      if (!TOOLS.has(tool) || tool === "erase" || objects.length >= 250) return;
      if ((tool === "arrow"||tool === "curve") && !arrowStart) {
        arrowStart = p; render(); return;
      }
      const next = {
        id: "m" + Date.now().toString(36) + Math.random().toString(36).slice(2, 9),
        kind: tool, x: (tool === "arrow"||tool === "curve") ? arrowStart.x : p.x,
        y: (tool === "arrow"||tool === "curve") ? arrowStart.y : p.y,
        end_x: (tool === "arrow"||tool === "curve") ? p.x : null,
        end_y: (tool === "arrow"||tool === "curve") ? p.y : null,
        size:brushSize,
        control_x:tool==="curve"?clamp((arrowStart.x+p.x)/2-(p.y-arrowStart.y)*.3,800):null,
        control_y:tool==="curve"?clamp((arrowStart.y+p.y)/2+(p.x-arrowStart.x)*.3,500):null,
        label: null, color
      };
      objects.push(next);
      selected = new Set([next.id]);
      arrowStart = null;
      render(); sync();
    });
    svg.addEventListener("pointermove", event => {
      if(draggingCurve){
        const p=position(event);
        const obj=objects.find(o=>o.id===draggingCurve);
        if(p&&obj){obj.control_x=p.x;obj.control_y=p.y;render();}
        return;
      }
      if (dragging) {
        const p = position(event);
        if (!p) return;
        const dx = p.x - dragging.start.x, dy = p.y - dragging.start.y;
        for (const original of dragging.original) {
          const item = objects.find(candidate => candidate.id === original.id);
          if (!item) continue;
          item.x = clamp(original.x + dx, 800);
          item.y = clamp(original.y + dy, 500);
          if (item.kind === "arrow"||item.kind === "curve") {
            item.end_x = clamp((original.end_x ?? original.x) + dx, 800);
            item.end_y = clamp((original.end_y ?? original.y) + dy, 500);
            if(item.kind==="curve"){
              item.control_x=clamp((original.control_x??original.x)+dx,800);
              item.control_y=clamp((original.control_y??original.y)+dy,500);
            }
          }
        }
        render();
      } else if (marquee) {
        const p = position(event);
        if (!p) return;
        marquee.end = p;
        const box = rectangle(marquee.start, p);
        selected = new Set(marquee.addTo);
        for (const item of objects) {
          if (inRect({ x: item.x, y: item.y }, box) ||
              ((item.kind === "arrow"||item.kind === "curve") && inRect({
                x: item.end_x ?? item.x, y: item.end_y ?? item.y
              }, box))) selected.add(item.id);
        }
        render();
      }
    });
    const finish = () => {
      if (dragging||draggingCurve) { dragging = null;draggingCurve=null;sync(); }
      if (marquee) { marquee = null; render(); }
    };
    svg.addEventListener("pointerup", finish);
    svg.addEventListener("pointercancel", finish);
    paint(color); loadDiagram();
  }
  const scan = () => document.querySelectorAll(".seppyl-tactics-board").forEach(setup);
  scan();
  new MutationObserver(scan).observe(document.documentElement, { childList: true, subtree: true, attributes:true, attributeFilter:["data-tactics-revision"] });
})();
