const { test, expect } = require('@playwright/test');

const ciPassword = 'e2e-ci-only-secret!';

// Unlike a static check, these listeners catch uncaught Dioxus/WASM panics,
// including the historic <Link> rendered outside <Router> failure.
test.beforeEach(async ({ page }) => {
  const failures = [];
  page.__runtimeFailures = failures;
  page.on('pageerror', error => failures.push('pageerror: ' + error.message));
  page.on('console', message => {
    if (message.type() === 'error') failures.push('console.error: ' + message.text());
  });
  page.on('requestfailed', request => {
    if (request.url().startsWith('http://127.0.0.1:4173/') && !String(request.failure()?.errorText).includes('ERR_ABORTED')) {
      failures.push('requestfailed: ' + request.method() + ' ' + request.url() + ': ' + request.failure()?.errorText);
    }
  });
});
test.afterEach(async ({ page }) => {
  // Allow pending microtasks/resources to settle before checking runtime logs.
  await page.waitForTimeout(200);
  expect(page.__runtimeFailures, 'Browser runtime / JS / WASM / network errors').toEqual([]);
});

test('public pages hydrate, links navigate, and global footer never panics', async ({ page }) => {
  await page.goto('/login');
  await expect(page.getByRole('heading', { name: 'Connexion' })).toBeVisible();
  await expect(page.locator('.site-footer')).toBeVisible();
  const versionFromManifest = require('node:fs').readFileSync(require('node:path').resolve(__dirname, '../../client/Cargo.toml'), 'utf8').match(/^version\s*=\s*"(\d+\.\d+\.\d+)"/m)?.[1];
  expect(versionFromManifest, 'client Cargo version').toBeTruthy();
  await expect(page.locator('.site-footer-version')).toHaveText('v' + versionFromManifest);
  await page.locator('.site-footer a[href="/mentions-legales"]').click();
  await expect(page).toHaveURL(/\/mentions-legales$/);
  await expect(page.getByRole('heading', { name: 'Mentions légales' })).toBeVisible();
  await page.locator('.site-footer a[href="/confidentialite"]').click();
  await expect(page).toHaveURL(/\/confidentialite$/);
  await expect(page.getByRole('heading', { name: /confidentialité/i })).toBeVisible();
  await page.locator('.site-footer a[href="/cgu"]').click();
  await expect(page).toHaveURL(/\/cgu$/);
  await page.goto('/public');
  await expect(page.getByRole('heading', { name: /Annuaire public/ })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Connexion' })).toBeVisible();
});

test('real login, authenticated navigation and page reload work against the API', async ({ page }) => {
  await page.goto('/login');
  await page.locator('input[autocomplete="username"]').fill('fixture-coach');
  await page.locator('input[type="password"]').fill(ciPassword);
  await page.locator('button[type="submit"]').click();
  await expect(page).toHaveURL(/\/trainings$/);
  await expect(page.getByRole('heading', { name: 'Entraînements' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Administration' })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Entraînements' })).toBeVisible();
  for (const route of ['/players', '/activities', '/tactics', '/club', '/admin', '/settings']) {
    await page.goto(route);
    await expect(page.locator('main')).toBeVisible();
    await expect(page.locator('main')).not.toBeEmpty();
    await expect(page).not.toHaveURL(/\/login$/);
  }
});

test('mobile-sized public pages render without uncaught errors', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/public');
  await expect(page.getByRole('heading', { name: /Annuaire public/ })).toBeVisible();
  await page.goto('/login');
  await expect(page.getByRole('heading', { name: 'Connexion' })).toBeVisible();
  await expect(page.locator('.site-footer')).toBeVisible();
});

test('unauthenticated API is rejected, registration proxy stays closed', async ({ request }) => {
  const privateResponse = await request.get('/api/account');
  expect(privateResponse.status()).toBe(401);
  const register = await request.post('/api/register', {
    data: { username: 'never-created-e2e', password: 'never-created-e2e' },
  });
  expect(register.status()).toBe(403);
  const health = await request.get('/api/health');
  expect(health.ok()).toBeTruthy();
});


test('tickets and announcement pages render in Chromium without browser errors',async ({page})=>{
  await page.goto('/login');
  await page.locator('input[autocomplete="username"]').fill('fixture-coach');
  await page.locator('input[type="password"]').fill(ciPassword);
  await page.locator('button[type="submit"]').click();
  await expect(page).toHaveURL(/\/trainings$/);
  await page.goto('/tickets');
  await expect(page.getByRole('heading',{name:'Mes tickets'})).toBeVisible();
  // The release history is legitimately shown to first-time accounts and
  // intercepts clicks until acknowledged. Close it as a real user would.
  const releaseHistory=page.getByRole('dialog',{name:'Historique des versions'});
  if (await releaseHistory.isVisible()) {
    await releaseHistory.getByRole('button',{name:"Fermer l'historique"}).click();
  }
  await page.getByRole('button',{name:/Nouveau ticket/}).click();
  await page.getByRole('dialog').locator('input[maxlength="160"]').fill('Incident de test E2E');
  await page.getByRole('dialog').locator('textarea').fill('Rapport synthétique E2E');
  await page.getByRole('dialog').getByRole('button',{name:'Envoyer'}).click();
  await expect(page).toHaveURL(/\/tickets\/[\w-]+$/);
  await expect(page.getByText('Rapport synthétique E2E')).toBeVisible();
  await page.goto('/matches');
  await expect(page.getByRole('heading',{name:'Matchs'})).toBeVisible();
  await page.goto('/announcements/manage');
  await expect(page.getByRole('heading',{name:'Annonces'})).toBeVisible();
  await page.goto('/news');
  await expect(page.getByRole('heading',{name:'Nouveautés'})).toBeVisible();
});

test('ticket permission checks and non-public roster are enforced server-side',async ({request})=>{
  const privateTickets=await request.get('/api/tickets');
  expect(privateTickets.status()).toBe(401);
  const privateStats=await request.get('/api/players/00000000-0000-0000-0000-000000000000/time-totals');
  expect(privateStats.status()).toBe(401);
  const hiddenRoster=await request.get('/api/public/teams/00000000-0000-0000-0000-000000000000/players');
  expect(hiddenRoster.status()).toBe(404);
  const banners=await request.get('/api/announcements');
  expect(banners.status()).toBe(200);
  expect(Array.isArray(await banners.json())).toBeTruthy();
});

test('ticket owner isolation, staff escalation and published urgent banner',async ({request,page})=>{
  const userAuth=await request.post('/api/login',{
    data:{username:'fixture-user',password:ciPassword}});
  expect(userAuth.status()).toBe(200);
  const userToken=(await userAuth.json()).token;
  const created=await request.post('/api/tickets',{
    headers:{Authorization:'Bearer '+userToken},
    data:{subject:'Ticket privé E2E',message:'Message confidentiel'}});
  expect(created.status()).toBe(201);
  const ticket=await created.json();
  const maintainerAuth=await request.post('/api/login',{
    data:{username:'fixture-coach',password:ciPassword}});
  expect(maintainerAuth.status()).toBe(200);
  const staffToken=(await maintainerAuth.json()).token;
  const outsider=await request.get('/api/tickets/'+ticket.id);
  expect(outsider.status()).toBe(401);
  const cannotEscalate=await request.put('/api/tickets/'+ticket.id+'/status',{
    headers:{Authorization:'Bearer '+userToken},
    data:{status:'in_progress',maintainer_required:true}});
  expect(cannotEscalate.status()).toBe(403);
  const escalated=await request.put('/api/tickets/'+ticket.id+'/status',{
    headers:{Authorization:'Bearer '+staffToken},
    data:{status:'in_progress',maintainer_required:true}});
  expect(escalated.status()).toBe(204);
  const announcement=await request.post('/api/admin/announcements',{
    headers:{Authorization:'Bearer '+staffToken},
    data:{title:'Maintenance de test',body:'Annonce urgente E2E',
      urgent:true,published:true,starts_at:0,ends_at:null}});
  expect(announcement.status()).toBe(201);
  await page.goto('/news');
  await expect(page.getByText('Annonce urgente E2E')).toBeVisible();
  await expect(page.locator('.global-urgent-banner')).toContainText('Maintenance de test');
});

test('match event timeline and public-roster opt-in are enforced end to end',async ({request})=>{
  const response=await request.post('/api/login',{data:{username:'fixture-coach',password:ciPassword}});
  expect(response.status()).toBe(200);
  const token=(await response.json()).token;
  const headers={Authorization:'Bearer '+token};
  const newClub=await request.post('/api/clubs',{headers,data:{name:'Club sixième passe'}});
  expect(newClub.status()).toBe(201);
  const club=await newClub.json();
  const newTeam=await request.post('/api/clubs/'+club.id+'/teams',{
    headers,data:{name:'Équipe 1'}});
  expect(newTeam.status()).toBe(201);
  const team=await newTeam.json();
  const newPlayer=async (firstname,number)=>{
    const r=await request.post('/api/teams/'+team.id+'/players',{
      headers,data:{firstname,lastname:'Test E2E',number}});
    expect(r.status()).toBe(201);
    return r.json();
  };
  const alice=await newPlayer('Alice',101);
  const bob=await newPlayer('Bob',102);
  let hidden=await request.get('/api/public/clubs/'+club.id+'/teams');
  expect(hidden.status()).toBe(404);
  const published=await request.put('/api/clubs/'+club.id+'/visibility',{
    headers,data:{is_public:true,show_public_teams:true,show_public_roster:false}});
  expect(published.status()).toBe(204);
  const publicTeams=await request.get('/api/public/clubs/'+club.id+'/teams');
  expect(publicTeams.status()).toBe(200);
  expect((await publicTeams.json()).length).toBe(1);
  hidden=await request.get('/api/public/teams/'+team.id+'/players');
  expect(hidden.status()).toBe(404);
  const exposeRoster=await request.put('/api/clubs/'+club.id+'/visibility',{
    headers,data:{is_public:true,show_public_teams:true,show_public_roster:true}});
  expect(exposeRoster.status()).toBe(204);
  const roster=await request.get('/api/public/teams/'+team.id+'/players');
  expect(roster.status()).toBe(200);
  expect((await roster.json()).length).toBe(2);
  const match=await request.post('/api/matches',{
    headers,data:{team_id:team.id,opponent:'Adversaire E2E',
      date:'2026-10-31T00:00:00',half_minutes:45}});
  expect(match.status()).toBe(201);
  const game=await match.json();
  const lineup=await request.put('/api/matches/'+game.id+'/lineup',{
    headers,data:{players:[
      {player_id:alice.id,position:'Attaquante',starter:true},
      {player_id:bob.id,position:'Milieu',starter:false},
    ]}});
  expect(lineup.status()).toBe(204);
  const substitution=await request.post('/api/matches/'+game.id+'/events',{
    headers,data:{kind:'substitution',minute:30,player_id:alice.id,
      player_in_id:bob.id,event_type_id:null,notes:'Changement E2E'}});
  expect(substitution.status()).toBe(201);
  const detail=await request.get('/api/matches/'+game.id,{headers});
  expect(detail.status()).toBe(200);
  const gameInfo=await detail.json();
  expect(gameInfo.playing_times.find(p=>p.player_id===alice.id).minutes).toBe(30);
  expect(gameInfo.playing_times.find(p=>p.player_id===bob.id).minutes).toBe(60);
  const outsiderAuth=await request.post('/api/login',{
    data:{username:'fixture-user',password:ciPassword}});
  const outsiderToken=(await outsiderAuth.json()).token;
  const outsider=await request.get('/api/matches/'+game.id,{
    headers:{Authorization:'Bearer '+outsiderToken}});
  expect(outsider.status()).toBe(404);
  const total=await request.get('/api/players/'+alice.id+'/time-totals',{headers});
  expect(total.status()).toBe(200);
  expect((await total.json()).match_minutes).toBe(30);
});
