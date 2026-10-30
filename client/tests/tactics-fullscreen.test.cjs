const test=require("node:test");
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const vm=require("node:vm");
const code=fs.readFileSync(path.join(__dirname,"../assets/tactics-fullscreen.js"),"utf8");
function fixture(){
 const handlers={};
 const toggle={
   textContent:"Plein écran",title:"",
   setAttribute(key,value){this[key]=value;}
 };
 const document={
   fullscreenElement:null,
   getElementById(name){return name==="tactics-studio"?studio:null;},
   addEventListener(name,listener){handlers[name]=listener;},
   async exitFullscreen(){this.fullscreenElement=null;handlers.fullscreenchange();}
 };
 const studio={
   querySelector(){return toggle;},
   async requestFullscreen(){document.fullscreenElement=studio;handlers.fullscreenchange();}
 };
 const event={target:{closest:()=>toggle}};
 const window={};
 vm.runInNewContext(code,{window,document,console});
 return {document,studio,toggle,click:async()=>handlers.click(event)};
}
test("fullscreen keeps studio as the fullscreen root, not just the pitch",async()=>{
 const x=fixture();
 await x.click();
 assert.equal(x.document.fullscreenElement,x.studio);
 assert.equal(x.toggle.textContent,"Quitter le plein écran");
 assert.equal(x.toggle["aria-pressed"],"true");
 await x.click();
 assert.equal(x.document.fullscreenElement,null);
 assert.equal(x.toggle.textContent,"Plein écran");
 assert.equal(x.toggle["aria-pressed"],"false");
});
