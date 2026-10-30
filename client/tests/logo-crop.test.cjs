const test=require("node:test");
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const vm=require("node:vm");
const script=fs.readFileSync(path.join(__dirname,"../assets/logo-crop.js"),"utf8");
function setup(){
  const handlers={},events=[];
  const x={value:"0",dispatchEvent:e=>events.push(["x",e.type,x.value])};
  const y={value:"0",dispatchEvent:e=>events.push(["y",e.type,y.value])};
  const zoom={value:"100",dispatchEvent:e=>events.push(["zoom",e.type,zoom.value])};
  const dialog={querySelector:s=>({
    "[data-logo-crop-x]":x,"[data-logo-crop-y]":y,"[data-logo-crop-zoom]":zoom
  })[s]};
  const stage={closest:s=>s===".logo-crop-dialog"?dialog:null,
    setPointerCapture(){},getBoundingClientRect:()=>({width:200,height:200})};
  const target={closest:s=>s==="[data-logo-crop-stage]"?stage:null};
  const document={addEventListener:(type,fn)=>{handlers[type]=fn;}};
  const window={};
  vm.runInNewContext(script,{window,document,Event:class{constructor(type){this.type=type;}}});
  return {handlers,events,x,y,zoom,target};
}
test("dragging on an image changes crop coordinates",()=>{
  const f=setup();
  f.handlers.pointerdown({target:f.target,button:0,pointerId:12,
    clientX:30,clientY:30,preventDefault(){}});
  f.handlers.pointermove({pointerId:12,clientX:110,clientY:90});
  assert.equal(f.x.value,"40");assert.equal(f.y.value,"30");
  assert.equal(f.events.length,2);
  f.handlers.pointerup({pointerId:12});
  f.handlers.pointermove({pointerId:12,clientX:150,clientY:150});
  assert.equal(f.x.value,"40");
});
test("wheel zoom stays bounded and prevents page scrolling only over crop",()=>{
  const f=setup();
  let prevented=false;
  f.handlers.wheel({target:f.target,deltaY:-100,preventDefault(){prevented=true;}});
  assert.equal(prevented,true);
  assert.equal(f.zoom.value,"105");
  f.zoom.value="300";
  f.handlers.wheel({target:f.target,deltaY:-100,preventDefault(){}});
  assert.equal(f.zoom.value,"300");
  f.handlers.wheel({target:{closest:()=>null},deltaY:100,
    preventDefault(){throw Error("not in editor");}});
});
