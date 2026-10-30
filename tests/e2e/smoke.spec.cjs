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
