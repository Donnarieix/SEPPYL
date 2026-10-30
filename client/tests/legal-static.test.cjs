const test=require("node:test");
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const root=path.join(__dirname,"..");
function file(name){return fs.readFileSync(path.join(root,name),"utf8");}
test("all four indexable URLs are public and login is not indexed",()=>{
 const map=file("public/sitemap.xml");
 const robots=file("public/robots.txt");
 for(const route of ["/public","/mentions-legales","/confidentialite","/cgu"]){
   assert.match(map,new RegExp("https://seppyl\\.donnarieix\\.fr"+route.replace(/\//g,"\\/")));
 }
 for(const route of ["/admin","/settings","/players","/login"]){
   assert.ok(!map.includes("fr"+route),"private route in sitemap: "+route);
 }
 assert.ok(robots.includes("Disallow: /login"));
 assert.ok(robots.includes("Sitemap: https://seppyl.donnarieix.fr/sitemap.xml"));
});
test("legal links, contact and release are in the shared footer",()=>{
 const source=file("src/routes/legal.rs");
 for(const entry of ["Route::LegalNotice","Route::PrivacyPolicy","Route::TermsOfUse",
   "mailto:seppyl@donnarieix.fr","https://github.com/LuDoSniper"]){
    assert.ok(source.includes(entry),entry);
 }
 assert.ok(source.includes('env!("CARGO_PKG_VERSION")'));
});
test("privacy notice reflects actual localStorage authentication",()=>{
 const privacy=file("src/routes/legal.rs");
 const session=file("src/session.rs");
 assert.ok(session.includes("LocalStorage"));
 assert.ok(privacy.includes("stockage local du navigateur"));
});
