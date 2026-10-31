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

test('public registration succeeds via signup, remains least-privileged, and legacy URL stays closed', async ({ request }) => {
  const privateResponse = await request.get('/api/account');
  expect(privateResponse.status()).toBe(401);
  // The installed Nginx location = /api/register is intentionally not
  // rewritten by app deployments: reproduce its behavior in this test proxy.
  const blockedLegacy = await request.post('/api/register', {
    data: { username: 'never-created-e2e', password: 'never-created-e2e' },
  });
  expect(blockedLegacy.status()).toBe(403);

  const username = 'hotfix-signup-e2e';
  const password = 'test-signup-e2e-very-strong';
  const malformed = await request.post('/api/signup', {
    data: { username: 'xx', password: 'short' },
  });
  expect(malformed.status()).toBe(400);
  const created = await request.post('/api/signup', {
    data: { username, password },
  });
  expect(created.status()).toBe(200);
  const duplicate = await request.post('/api/signup', {
    data: { username, password },
  });
  expect(duplicate.status()).toBe(409);

  const login = await request.post('/api/login', { data: { username, password } });
  expect(login.status()).toBe(200);
  const { token } = await login.json();
  expect(token).toBeTruthy();
  const self = await request.get('/api/account', { headers: { Authorization: 'Bearer ' + token } });
  expect(self.status()).toBe(200);
  expect((await self.json()).role).toBe('user');
  const deniedWrite = await request.post('/api/player/create', {
    headers: { Authorization: 'Bearer ' + token },
    data: { firstname: 'Denied', lastname: 'Signup', number: 77 },
  });
  expect(deniedWrite.status()).toBe(403);
  const health = await request.get('/api/health');
  expect(health.ok()).toBeTruthy();
});

test('registration form signs up and logs in through production-style proxy', async ({ page }) => {
  await page.goto('/login');
  await page.getByRole('button', { name: 'Créer un nouveau compte' }).click();
  await expect(page.getByRole('heading', { name: 'Créer un compte' })).toBeVisible();
  await page.locator('input[autocomplete="username"]').fill('e2e-new-account-form');
  await page.locator('input[type="password"]').fill('e2e-signup-form-password!');
  await page.getByRole('button', { name: 'Créer le compte' }).click();
  await expect(page).toHaveURL(/\/trainings$/);
  await expect(page.getByRole('heading', { name: 'Entraînements' })).toBeVisible();
});
