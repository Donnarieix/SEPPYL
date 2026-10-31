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
