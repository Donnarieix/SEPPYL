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
 assert.ok(robots.includes("Disallow: /")); // Default deny includes the login and private routes.
 assert.ok(robots.includes("Allow: /public"));
 assert.ok(robots.includes("Sitemap: https://seppyl.donnarieix.fr/sitemap.xml"));
});
test("legal links, contact and release are in the shared footer",()=>{
 const source=file("src/routes/legal.rs");
 const footer=source.split("pub fn SiteFooter()")[1].split("fn LegalHeading(")[0];
 for(const entry of ['href:"/mentions-legales"','href:"/confidentialite"',
   'href:"/cgu"',"mailto:seppyl@donnarieix.fr","https://github.com/Donnarieix"]){
    assert.ok(footer.includes(entry),entry);
 }
 assert.ok(!footer.includes("Link{"),"Footer must not render router Links outside Router");
 assert.ok(source.includes('env!("CARGO_PKG_VERSION")'));
});
test("privacy notice reflects actual localStorage authentication",()=>{
 const privacy=file("src/routes/legal.rs");
 const session=file("src/session.rs");
 assert.ok(session.includes("LocalStorage"));
 assert.ok(privacy.includes("stockage local"));
});

test("legal notice distinguishes home hosting from DNS/email provider",()=>{
 const source=file("src/routes/legal.rs");
 assert.ok(source.includes("auto-héberg")||source.includes("hébergées sur un serveur privé"));
 assert.ok(source.includes("o2switch"));
 assert.ok(source.includes("DNS"));
 assert.ok(source.includes("mineures"));
 assert.ok(source.includes("suppression"));
 assert.ok(source.includes("Donnarieix"));
 assert.ok(!source.includes("À renseigner avant la mise en ligne"));
});
