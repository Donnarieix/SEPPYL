const test=require("node:test");
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const vm=require("node:vm");
const src=fs.readFileSync(path.join(__dirname,"../assets/select-overlay.js"),"utf8");
function fixture(){
 const listeners={};
 class Element {
   constructor(tag="div"){
     this.tag=tag;this.style={};this.children=[];
     this.className="";this.textContent="";this.attributes={};
     this.disabled=false;this.hidden=false;this.isConnected=true;
     this.scrollHeight=120;this.tabIndex=-1;
     this.listeners={};
     const classNames=new Set();
     this.classList={
       add:name=>classNames.add(name),remove:name=>classNames.delete(name),
       toggle:(name,on)=>on?classNames.add(name):classNames.delete(name),
       contains:name=>classNames.has(name)
     };
   }
   addEventListener(name,handler){this.listeners[name]=handler;}
   emit(name,event){return this.listeners[name]?.(event);}
   setAttribute(k,v){this.attributes[k]=v;}
   removeAttribute(k){delete this.attributes[k];}
   getAttribute(k){return this.attributes[k]??null;}
   append(child){this.children.push(child);child.parent=this;}
   replaceChildren(...children){this.children=[];children.forEach(child=>this.append(child));}
   remove(){this.parent.children=this.parent.children.filter(c=>c!==this);this.isConnected=false;}
   contains(child){return this.children.includes(child);}
   closest(selector){
     if(selector===".site-root")return host;
     if(selector===".field")return null;
     return null;
   }
   matches(selector){return this.tag==="select"&&selector.startsWith("select:");}
   focus(){document.activeElement=this;}
   getBoundingClientRect(){return {left:20,top:30,right:200,bottom:74,width:180};}
   scrollIntoView(){}
 }
 const host=new Element();
 const select=new Element("select");select.value="a";
 select.options=[
   {value:"a",textContent:"Premier",hidden:false,disabled:false},
   {value:"b",textContent:"Deuxième",hidden:false,disabled:false}
 ];
 select.attributes["aria-label"]="Choix";
 select.dispatchEvent=e=>events.push({type:e.type,value:select.value});
 let events=[];
 const document={
   fullscreenElement:null,
   addEventListener(n,f){listeners[n]=f;},
   createElement(tag){return new Element(tag);},
 };
 const window={
   matchMedia(){return {matches:false};},
   addEventListener(){}
 };
 vm.runInNewContext(src,{window,document,innerHeight:600,innerWidth:800,Event:class{constructor(type){this.type=type;}},console});
 const pointerdown={target:select,button:0,preventDefault(){this.prevented=true;}};
 return {listeners,document,select,host,events,pointerdown};
}
test("dropdown retains native select value, emits framework input/change, supports choice",()=>{
 const f=fixture();
 f.listeners.pointerdown(f.pointerdown);
 assert.equal(f.pointerdown.prevented,true);
 assert.equal(f.host.children.length,1);
 const popover=f.host.children[0];
 assert.equal(popover.attributes.role,"listbox");
 popover.children[1].emit("click",{});
 assert.equal(f.select.value,"b");
 assert.deepEqual(f.events.map(e=>e.type),["input","change"]);
 assert.equal(f.host.children.length,0);
});
test("Escape closes dropdown without changing selection",()=>{
 const f=fixture();
 f.listeners.pointerdown(f.pointerdown);
 f.listeners.keydown({key:"Escape",preventDefault(){}});
 assert.equal(f.select.value,"a");
 assert.equal(f.host.children.length,0);
});

test("large account selector searches inside the popup without listing its placeholder",()=>{
 const f=fixture();
 f.select.options=[
   {value:"",textContent:"Choisir un utilisateur",hidden:false,disabled:false},
   {value:"1",textContent:"Émilie Laurent",hidden:false,disabled:false},
   {value:"2",textContent:"Camille Martin",hidden:false,disabled:false},
   {value:"3",textContent:"Émilie Petit",hidden:false,disabled:false}
 ];
 f.select.value="";
 f.select.setAttribute("data-seppyl-searchable","true");
 f.listeners.pointerdown(f.pointerdown);
 const panel=f.host.children[0];
 assert.equal(panel.attributes.role,"dialog");
 const search=panel.children[0];
 const list=panel.children[1];
 assert.equal(search.type,"search");
 assert.equal(list.children.length,3); // Placeholder is not an option.
 search.value="emilie";
 search.emit("input",{});
 assert.equal(list.children.length,2);
 assert.equal(list.children[0].textContent,"Émilie Laurent");
 list.children[1].emit("click",{});
 assert.equal(f.select.value,"3");
 assert.deepEqual(f.events.map(e=>e.type),["input","change"]);
});
test("short role selector keeps the existing menu without search",()=>{
 const f=fixture();
 f.listeners.pointerdown(f.pointerdown);
 const panel=f.host.children[0];
 assert.equal(panel.attributes.role,"listbox");
 assert.equal(panel.children.length,2);
 assert.equal(panel.children.some(child=>child.tag==="input"),false);
});
