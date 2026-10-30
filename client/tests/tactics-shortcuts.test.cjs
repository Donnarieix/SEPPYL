const test=require("node:test");
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const vm=require("node:vm");
const source=fs.readFileSync(path.join(__dirname,"../assets/tactics-board.js"),"utf8");

function fixture(){
  class FakeNode {
    constructor(){
      this.listeners={};this.dataset={};this.style={};this.value="";
      this.attributes={};this.classList={
        classes:new Set(),add(v){this.classes.add(v);},
        remove(v){this.classes.delete(v);},
        toggle(v,yes){if(yes)this.classes.add(v);else this.classes.delete(v);},
        contains(v){return this.classes.has(v);}
      };
    }
    addEventListener(event,fn){(this.listeners[event]??=[]).push(fn);}
    emit(event,data){for(const fn of this.listeners[event]||[])fn(data);}
    setAttribute(key,value){this.attributes[key]=String(value);}
    getAttribute(key){return this.attributes[key]??null;}
    append(){}
    replaceChildren(){}
    dispatchEvent(){}
    matches(){return false;}
    closest(){return null;}
    contains(){return false;}
    focus(){document.activeElement=this;}
  }
  const document={
    documentElement:new FakeNode(),
    activeElement:null,
    createElementNS(){return new FakeNode();},
    querySelectorAll(selector){return selector===".seppyl-tactics-board"?[root]:[];},
    addEventListener(){}
  };
  const svg=new FakeNode();
  svg.createSVGPoint=()=>({x:0,y:0,matrixTransform(){return {x:200,y:200};}});
  svg.getScreenCTM=()=>({inverse(){return {};}});
  svg.getBoundingClientRect=()=>({left:10,top:10,width:800,height:500});
  svg.setPointerCapture=()=>{};
  const palette=new FakeNode();
  const output=new FakeNode();
  const counter=new FakeNode();
  const quick=new FakeNode();quick.dataset.tacticQuickColor="#c6403c";
  const size=new FakeNode();size.value="22";
  const solid=new FakeNode();solid.dataset.tacticDash="solid";
  const dashed=new FakeNode();dashed.dataset.tacticDash="dashed";
  const root=new FakeNode();
  root.dataset.tacticsInitial=JSON.stringify([{
    id:"one",kind:"player",x:200,y:200,color:"#2455b6",size:22,
    end_x:null,end_y:null,label:null
  }]);
  root.dataset.tacticsRevision="0";
  root.getBoundingClientRect=()=>({left:0,top:0,width:900,height:730});
  root.querySelector=selector=>({
    "[data-tactics-svg]":svg,"[data-tactics-output]":output,
    "[data-tactic-menu]":palette,"[data-selection-count]":counter,
    "[data-tactic-size]":size
  })[selector]??null;
  root.querySelectorAll=selector=>
    selector==="[data-tactic-quick-color]"?[quick]:
    selector==="[data-tactic-dash]"?[solid,dashed]:[];
  const window={};
  vm.runInNewContext(source,{window,document,MutationObserver:class{observe(){}},Event,
    setTimeout:()=>1,clearTimeout:()=>{}});
  const marker={closest(selector){
    return selector==="[data-marker-id]"?{getAttribute:()=>"one"}:null;
  }};
  function select(){
    svg.emit("pointerdown",{button:0,clientX:200,clientY:200,pointerId:1,
      target:marker,shiftKey:false});
    svg.emit("pointerup",{});
  }
  return {svg,root,palette,output,quick,document,select,size,solid,dashed};
}

test("canvas wheel resizes selected object and right click changes its color",()=>{
  const f=fixture();
  f.select();
  f.svg.emit("wheel",{deltaY:-1,preventDefault(){}});
  assert.equal(JSON.parse(f.output.value)[0].size,24);
  assert.equal(f.size.value,"24");
  let prevented=false;
  f.svg.emit("contextmenu",{
    target:{closest(){return null;}},clientX:300,clientY:300,
    preventDefault(){prevented=true;}
  });
  assert.equal(prevented,true);
  assert.equal(f.palette.classList.contains("open"),true);
  f.quick.emit("click",{});
  assert.equal(f.palette.classList.contains("open"),false);
  assert.equal(JSON.parse(f.output.value)[0].color,"#c6403c");
});
test("focused input disables both mouse and keyboard shortcuts",()=>{
  const f=fixture();
  f.select();
  const before=f.output.value;
  f.document.activeElement={matches:s=>s==="input, textarea, select",isContentEditable:false};
  f.svg.emit("wheel",{deltaY:-1,preventDefault(){throw Error("wheel should not be intercepted");}});
  f.svg.emit("contextmenu",{preventDefault(){throw Error("contextmenu should not be intercepted");}});
  f.root.emit("keydown",{target:f.svg,key:"Backspace",ctrlKey:false,metaKey:false,altKey:false,
    preventDefault(){throw Error("keyboard should not be intercepted");}});
  assert.equal(f.output.value,before);
  assert.equal(f.palette.classList.contains("open"),false);
});
test("Delete and Escape handle selection only while keyboard focus is outside inputs",()=>{
  const f=fixture();
  f.select();
  f.root.emit("keydown",{target:f.svg,key:"Escape",ctrlKey:false,metaKey:false,altKey:false,preventDefault(){}});
  f.select();
  f.root.emit("keydown",{target:f.svg,key:"Delete",ctrlKey:false,metaKey:false,altKey:false,preventDefault(){}});
  assert.deepEqual(JSON.parse(f.output.value),[]);
});

test("selected arrows retain a dashed stroke on save, and solid mode reverses it",()=>{
  const f=fixture();
  f.root.dataset.tacticsInitial=JSON.stringify([{
    id:"one",kind:"arrow",x:200,y:200,end_x:340,end_y:240,
    size:22,color:"#2455b6",dashed:false,label:null
  }]);
  f.root.dataset.tacticsRevision="1";
  f.root.__tacticsReload();
  f.select();
  f.dashed.emit("click",{});
  let items=JSON.parse(f.output.value);
  assert.equal(items.length,1);
  assert.equal(items[0].dashed,true);
  assert.equal(f.dashed.classList.contains("selected"),true);
  f.solid.emit("click",{});
  items=JSON.parse(f.output.value);
  assert.equal(items[0].dashed,false);
  assert.equal(f.solid.classList.contains("selected"),true);
});
