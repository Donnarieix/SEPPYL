const { test, expect } = require('@playwright/test');

const password = 'e2e-ci-only-secret!';

test('Discord ticket link returns to the requested conversation after login', async ({ page, request }) => {
  const login = await request.post('/api/login', {
    data: { username: 'fixture-user', password },
  });
  expect(login.status()).toBe(200);
  const token = (await login.json()).token;
  const created = await request.post('/api/tickets', {
    headers: { Authorization: 'Bearer ' + token },
    data: { subject: 'Discord deep link validation', message: 'Private test message' },
  });
  expect(created.status()).toBe(201);
  const id = (await created.json()).id;
  await page.goto('/public');
  await expect(page.getByRole('heading', { name: /Annuaire public/ })).toBeVisible();
  await page.goto('/tickets/' + id);
  await expect.poll(() => new URL(page.url()).pathname).toBe('/login');
  expect(new URL(page.url()).searchParams.get('next')).toBe('/tickets/' + id);
  await page.locator('input[autocomplete="username"]').fill('fixture-user');
  await page.locator('input[type="password"]').fill(password);
  await page.locator('button[type="submit"]').click();
  await expect.poll(() => new URL(page.url()).pathname).toBe('/tickets/' + id);
  await expect(page.getByRole('heading', { name: 'Discord deep link validation' })).toBeVisible();
  await page.goBack();
  await expect.poll(() => new URL(page.url()).pathname).toBe('/public');
});

test('malicious next parameter cannot redirect to an external destination', async ({ page }) => {
  const candidates = [
    'https://evil.example/steal',
    '//evil.example/steal',
    'javascript:alert(1)',
    '/%2F%2Fevil.example',
    '/tickets/%2F%2Fevil.example',
    '/login?next=//evil.example',
    '/unknown-route',
  ];
  for (const next of candidates) {
    await page.goto('/login?next=' + encodeURIComponent(next));
    await expect(page.getByRole('heading', { name: 'Connexion' })).toBeVisible();
    await page.locator('input[autocomplete="username"]').fill('fixture-user');
    await page.locator('input[type="password"]').fill(password);
    await page.locator('button[type="submit"]').click();
    await expect.poll(() => new URL(page.url()).pathname).toBe('/trainings');
    expect(new URL(page.url()).hostname).toBe('127.0.0.1');
    await page.evaluate(() => localStorage.removeItem('seppyl.session.v1'));
  }
});

test('unknown routes show SEPPYL 404 without requesting authentication', async ({ page }) => {
  for (const path of ['/not-a-route', '/tickets/example/extra', '/some/unknown/path']) {
    await page.goto(path);
    await expect.poll(() => new URL(page.url()).pathname).toBe(path);
    await expect(page.getByRole('heading', { name: 'Page introuvable' })).toBeVisible();
    await expect(page.locator('.not-found-code')).toHaveText('404');
    await expect(page.getByRole('link', { name: "Retour à l'accueil" })).toBeVisible();
  }
});
