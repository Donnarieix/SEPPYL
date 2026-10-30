const test=require("node:test");
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const vm=require("node:vm");
const src=fs.readFileSync(path.join(__dirname,"../assets/toast.js"),"utf8");
test("toast replaces previous message and disappears after idle without layout changes",()=>{
 let clock=0;const timers=new Map();
 const classes=new Set();
 const span={textContent:"Joueuse ajoutée."};
 const node={querySelector:()=>span,
   classList:{add:name=>classes.add(name),remove:name=>classes.delete(name)}};
 let notify;
 const document={
   documentElement:{},querySelectorAll:()=>[node],
   addEventListener:()=>{}
 };
 const window={};
 vm.runInNewContext(src,{
   window,document,MutationObserver:class{constructor(fn){notify=fn;}observe(){}},
   setTimeout:fn=>{const id=++clock;timers.set(id,fn);return id;},
   clearTimeout:id=>timers.delete(id)
 });
 assert.equal(timers.size,1);
 [...timers.values()][0]();
 assert.equal(classes.has("seppyl-toast-hidden"),true);
 span.textContent="Joueuse déjà dans l'effectif";
 notify();
 assert.equal(classes.has("seppyl-toast-hidden"),false);
 assert.equal(timers.size,1);
 const last=[...timers.values()].at(-1);last();
 assert.equal(classes.has("seppyl-toast-hidden"),true);
});
