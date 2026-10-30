const test=require("node:test");
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const root=path.join(__dirname,"..");
const read=p=>fs.readFileSync(path.join(root,p),"utf8");

test("root route redirects while private routes share an auth boundary",()=>{
  const routes=read("src/routes/mod.rs");
  assert.match(routes,/\#\[route\("\/"\)\]\s*RootRedirect/);
  assert.match(routes,/navigator\.replace\(destination\)/);
  assert.match(routes,/\#\[layout\(RequireAuth\)\]/);
  assert.match(routes,/if !authenticated\{return rsx!\{\};\}/);
  assert.match(routes,/Outlet::<Route>/);
  assert.match(routes,/\#\[route\("\/trainings"\)\]/);
});

test("login consumes available viewport and print eliminates application chrome",()=>{
  const css=read("assets/main.css");
  assert.ok(css.includes(".site-root > .login-screen"));
  assert.ok(css.includes("min-height:0;"));
  assert.ok(css.includes(".site-root > .print-sheet"));
  assert.ok(css.includes("display:block!important"));
  assert.ok(css.includes(".site-root > .site-footer"));
  assert.ok(css.includes(".print-guidance{display:none!important}"));
});

test("global footer does not render router links outside Router",()=>{
 const app=read("src/main.rs");
 const legal=read("src/routes/legal.rs");
 assert.ok(app.includes("Router::<routes::Route> {}"));
 assert.ok(app.includes("routes::legal::SiteFooter {}"));
 const footer=legal.split("pub fn SiteFooter()")[1].split("fn LegalHeading(")[0];
 assert.ok(!footer.includes("Link{"),"Router Link outside router would crash WebAssembly");
 for(const url of ["/mentions-legales","/confidentialite","/cgu"]){
   assert.ok(footer.includes('href:"'+url+'"'));
 }
});

test("footer layout uses one viewport for shell and only natural heights for routes",()=>{
 const css=read("assets/main.css");
 assert.ok(css.includes(".app { min-height: 0; }"),"Routes must not reserve their own 100vh");
 assert.ok(css.includes(".site-root > .app{"),"Authenticated shell must flex inside root");
 assert.ok(css.includes(".site-root > .app > main{"),"Header and content share available height");
 assert.ok(css.includes(".site-root > main{"),"Public and print pages share the same shell");
 assert.match(css,/\.site-root\s*>\s*\.app\{[^}]*flex:1 0 auto/);
 assert.match(css,/\.site-root\s*>\s*main\{[^}]*flex:1 0 auto/);
 assert.match(css,/\.site-footer\{[^}]*margin-top:auto/);
 assert.ok(css.includes(".site-root:has(.mobile-nav) > .site-footer"));
 assert.ok(css.includes(".site-root{padding-bottom:0!important}"));
 assert.ok(!css.includes("min-height: 100vh; }"),"Legacy route-level viewport is forbidden");
});
