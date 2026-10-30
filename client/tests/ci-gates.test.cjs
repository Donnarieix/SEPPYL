const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '../..');
function read(rel) { return fs.readFileSync(path.join(root, rel), 'utf8'); }

test('production never builds the emergency maintainer provisioner again', () => {
  assert.ok(!fs.existsSync(path.join(root, 'api/src/bootstrap.rs')));
  const main = read('api/src/main.rs');
  const state = read('api/src/state.rs');
  const workflow = read('.github/workflows/deploy.yml');
  for (const source of [main, state, workflow]) {
    assert.ok(!source.includes('SEPPYL_BOOTSTRAP_'), 'bootstrap variable still present');
    assert.ok(!source.includes('initialize_if_configured'), 'bootstrap initializer still invoked');
  }
  assert.ok(!state.includes('password: Set("$argon2id'), 'no embedded test admin account');
});

test('a push can only deploy on main after tests, artifacts and browser E2E succeed', () => {
  const workflow = read('.github/workflows/deploy.yml');
  assert.match(workflow, /needs: \[verify, build-api, build-web, e2e\]/);
  assert.match(workflow, /github\.ref == 'refs\/heads\/main'/);
  assert.match(workflow, /uses: \.\/\.github\/workflows\/verify\.yml/);
  assert.match(workflow, /Browser E2E \(Chromium \+ real API\)/);
  assert.match(workflow, /Upload browser traces and screenshots on failures/);
});

test('the browser suite catches the historic router Link runtime panic', () => {
  const spec = read('tests/e2e/smoke.spec.cjs');
  assert.match(spec, /page\.on\('pageerror'/);
  assert.match(spec, /page\.on\('console'/);
  assert.match(spec, /page\.on\('requestfailed'/);
  assert.match(spec, /site-footer a\[href/);
});
