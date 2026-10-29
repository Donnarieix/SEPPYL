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
