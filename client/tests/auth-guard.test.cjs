const assert = require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const vm=require("node:vm");
const test=require("node:test");
const source=fs.readFileSync(path.join(__dirname,"../assets/auth-guard.js"),"utf8");

function browser(status){
  const calls={redirects:[],removed:[],fetches:[]};
  const storage={removeItem(key){calls.removed.push(key);}};
  const window={
    fetch:async(input,init)=>{calls.fetches.push({input,init});return {status};},
    location:{replace(url){calls.redirects.push(url);}},
  };
  vm.runInNewContext(source,{window,Headers,Request,localStorage:storage});
  return {window,calls};
}
test("an authenticated 401 logs out and goes to /login",async()=>{
  const b=browser(401);
  await b.window.fetch("http://127.0.0.1:3000/player/create",
    {headers:{Authorization:"Bearer expired"},method:"POST"});
  assert.deepEqual(b.calls.removed,["seppyl.session.v1"]);
  assert.deepEqual(b.calls.redirects,["/login"]);
});
test("invalid credentials on /login do not trigger a redirect",async()=>{
  const b=browser(401);
  await b.window.fetch("http://127.0.0.1:3000/login",{method:"POST"});
  assert.deepEqual(b.calls.removed,[]);
  assert.deepEqual(b.calls.redirects,[]);
});
test("403 authorization is distinct from 401 authentication",async()=>{
  const b=browser(403);
  await b.window.fetch("/catalog",{headers:{Authorization:"Bearer valid"}});
  assert.deepEqual(b.calls.redirects,[]);
});

test("refresh of a protected route with no stored session redirects before rendering",()=>{
  const calls={redirects:[],removed:[]};
  const storage={
    getItem:()=>null,
    removeItem:key=>calls.removed.push(key)
  };
  const window={
    fetch:async()=>({status:200}),
    location:{pathname:"/trainings",replace:url=>calls.redirects.push(url)},
  };
  vm.runInNewContext(source,{window,Headers,Request,localStorage:storage});
  assert.deepEqual(calls.redirects,["/login"]);
});
test("legal and public pages stay available without a session",()=>{
  for(const pathname of ["/login","/public","/mentions-legales","/confidentialite","/cgu"]){
    const redirects=[];
    const window={
      fetch:async()=>({status:200}),
      location:{pathname,replace:url=>redirects.push(url)},
    };
    const storage={getItem:()=>null,removeItem(){}};
    vm.runInNewContext(source,{window,Headers,Request,localStorage:storage});
    assert.equal(redirects.length,0,pathname);
  }
});
test("expired JWT is redirected on load without waiting for an API response",()=>{
  const payload=Buffer.from(JSON.stringify({exp:1000})).toString("base64url");
  const token="header."+payload+".sig";
  const calls=[];
  const window={
    fetch:async()=>({status:200}),
    location:{pathname:"/club",replace:url=>calls.push(url)}
  };
  const storage={getItem:()=>token,removeItem(){}};
  vm.runInNewContext(source,{
    window,Headers,Request,localStorage:storage,
    atob:value=>Buffer.from(value,"base64").toString("utf8")
  });
  assert.deepEqual(calls,["/login"]);
});
test("a late 401 from an older session must not revoke a newer session",async()=>{
  const redirects=[],removed=[];
  const window={
    fetch:async()=>({status:401}),
    location:{pathname:"/players",replace:url=>redirects.push(url)}
  };
  const storage={getItem:()=>"new-session",removeItem:key=>removed.push(key)};
  vm.runInNewContext(source,{window,Headers,Request,localStorage:storage});
  await window.fetch("/players",{headers:{Authorization:"Bearer old-session"}});
  assert.deepEqual(redirects,[]);
  assert.deepEqual(removed,[]);
});
