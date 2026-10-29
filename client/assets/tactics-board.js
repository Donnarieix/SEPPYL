// Embedded tactical drawing tool (no third-party iframe).
(() => {
 if(window.__seppylTacticsInstalled) return;
 window.__seppylTacticsInstalled=true;
 const ns="http://www.w3.org/2000/svg";
 const element=(tag,attributes={})=>{const n=document.createElementNS(ns,tag);for(const [k,v] of Object.entries(attributes))n.setAttribute(k,String(v));return n;};
 const add=(parent,tag,attrs={})=>{const n=element(tag,attrs);parent.appendChild(n);return n;};
 const clamp=(v,max)=>Math.max(0,Math.min(v,max));
 const tools=new Set(["player","opponent","ball","cone","arrow"]);
 function init(container){
   if(container.dataset.tacticsInitialized)return;
   const svg=container.querySelector("[data-tactics-svg]");
   const out=container.querySelector("[data-tactics-output]");
   if(!svg||!out)return;
   container.dataset.tacticsInitialized="true";
   let objects;try{objects=JSON.parse(container.dataset.tacticsInitial||"[]");if(!Array.isArray(objects))objects=[];}catch(_){objects=[];}
   let currentTool="player",selected=null,drag=null,lineStart=null;
   const layout=container.dataset.tacticsLayout||"football";
   function pos(event){const pt=svg.createSVGPoint();pt.x=event.clientX;pt.y=event.clientY;const m=svg.getScreenCTM();if(!m)return null;const p=pt.matrixTransform(m.inverse());return {x:Math.round(clamp(p.x,800)),y:Math.round(clamp(p.y,500))};}
   function sync(){out.value=JSON.stringify(objects);out.dispatchEvent(new Event("input",{bubbles:true}));}
   function background(){
     add(svg,"rect",{x:0,y:0,width:800,height:500,fill:layout==="football"?"#508b65":layout==="volleyball"?"#c4a46d":"#e6edf1"});
     const line={stroke:"#ffffffbb","stroke-width":3,fill:"none"};
     if(layout==="football"){
       add(svg,"rect",{x:35,y:35,width:730,height:430,...line});
       add(svg,"line",{x1:400,y1:35,x2:400,y2:465,...line});
       add(svg,"circle",{cx:400,cy:250,r:70,...line});
       add(svg,"rect",{x:35,y:140,width:110,height:220,...line});
       add(svg,"rect",{x:655,y:140,width:110,height:220,...line});
     }else if(layout==="volleyball"){
       add(svg,"rect",{x:75,y:75,width:650,height:350,...line});
       add(svg,"line",{x1:400,y1:75,x2:400,y2:425,stroke:"#fff","stroke-width":5});
       add(svg,"line",{x1:250,y1:75,x2:250,y2:425,...line});
       add(svg,"line",{x1:550,y1:75,x2:550,y2:425,...line});
     }else{
       for(let x=0;x<=800;x+=40)add(svg,"line",{x1:x,y1:0,x2:x,y2:500,stroke:"#cad4db"});
       for(let y=0;y<=500;y+=40)add(svg,"line",{x1:0,y1:y,x2:800,y2:y,stroke:"#cad4db"});
     }
   }
   function render(){
     svg.replaceChildren();background();
     const defs=add(svg,"defs");
     const marker=add(defs,"marker",{id:"seppyl-arrow-head",markerWidth:8,markerHeight:8,refX:7,refY:4,orient:"auto",markerUnits:"strokeWidth"});
     add(marker,"path",{d:"M0 0 L8 4 L0 8 z",fill:"#24384c"});
     for(const item of objects){
       const group=add(svg,"g",{"data-marker-id":item.id,cursor:"grab"});
       const active=item.id===selected;
       if(item.kind==="arrow"){add(group,"line",{x1:item.x,y1:item.y,x2:item.end_x??item.x,y2:item.end_y??item.y,stroke:active?"#ffe28b":"#24384c","stroke-width":active?7:5,"marker-end":"url(#seppyl-arrow-head)"});continue;}
       const palette={player:"#2455b6",opponent:"#c6403c",ball:"#f8f8f5",cone:"#ee9228"};
       if(item.kind==="cone"){
          add(group,"polygon",{points:item.x+","+(item.y-15)+" "+(item.x-14)+","+(item.y+12)+" "+(item.x+14)+","+(item.y+12),fill:palette.cone,stroke:active?"#fff":"#533f28","stroke-width":3});
       }else{
          add(group,"circle",{cx:item.x,cy:item.y,r:item.kind==="ball"?11:19,fill:palette[item.kind]||"#aaa",stroke:active?"#ffe28b":"#253243","stroke-width":active?4:2});
       }
       if(item.label){const txt=add(group,"text",{x:item.x,y:item.y+4,fill:"#fff","text-anchor":"middle","font-size":12,"font-weight":700,"pointer-events":"none"});txt.textContent=item.label;}
     }
     if(lineStart)add(svg,"circle",{cx:lineStart.x,cy:lineStart.y,r:7,fill:"#ffe28b","pointer-events":"none"});
   }
   container.querySelectorAll("[data-tactic-tool]").forEach(button=>button.addEventListener("click",()=>{
     currentTool=button.dataset.tacticTool;lineStart=null;
     container.querySelectorAll("[data-tactic-tool]").forEach(b=>b.classList.toggle("selected",b===button));render();
   }));
   container.querySelector("[data-tactic-remove]")?.addEventListener("click",()=>{if(!selected)return;objects=objects.filter(o=>o.id!==selected);selected=null;render();sync();});
   svg.addEventListener("pointerdown",event=>{
     if(event.button!==0)return;
     const p=pos(event);if(!p)return;
     const marker=event.target.closest("[data-marker-id]");
     if(marker){
       selected=marker.getAttribute("data-marker-id");
       if(currentTool==="erase"){objects=objects.filter(o=>o.id!==selected);selected=null;render();sync();return;}
       drag={id:selected,start:p};svg.setPointerCapture(event.pointerId);render();return;
     }
     if(!tools.has(currentTool)||objects.length>=250)return;
     if(currentTool==="arrow"&&!lineStart){lineStart=p;render();return;}
     const item={id:"m"+Date.now().toString(36)+Math.random().toString(36).slice(2,9),kind:currentTool,x:currentTool==="arrow"?lineStart.x:p.x,y:currentTool==="arrow"?lineStart.y:p.y,end_x:currentTool==="arrow"?p.x:null,end_y:currentTool==="arrow"?p.y:null,label:null};
     objects.push(item);selected=item.id;lineStart=null;render();sync();
   });
   svg.addEventListener("pointermove",event=>{if(!drag)return;const p=pos(event);if(!p)return;const o=objects.find(o=>o.id===drag.id);if(!o)return;
     const dx=p.x-drag.start.x,dy=p.y-drag.start.y;
     o.x=clamp(o.x+dx,800);o.y=clamp(o.y+dy,500);
     if(o.kind==="arrow"){o.end_x=clamp((o.end_x??o.x)+dx,800);o.end_y=clamp((o.end_y??o.y)+dy,500);}
     drag.start=p;render();
   });
   svg.addEventListener("pointerup",()=>{if(drag){drag=null;sync();}});
   svg.addEventListener("pointercancel",()=>drag=null);
   render();
 }
 const scan=()=>document.querySelectorAll(".seppyl-tactics-board").forEach(init);
 scan();new MutationObserver(scan).observe(document.documentElement,{childList:true,subtree:true});
})();
