import { chromium } from '@playwright/test';
import assert from 'node:assert/strict';

const browser = await chromium.connectOverCDP('http://127.0.0.1:9223');
const page = browser.contexts().flatMap(c => c.pages()).find(p => p.url().includes('tauri.localhost'));
assert.ok(page, 'Native Tauri window not found');
const errors = [];
page.on('pageerror', e => errors.push(e.message));
await page.getByRole('button', { name: 'RU', exact: true }).click();
await page.getByRole('button', { name: /^(Сканировать|Пересканировать)$/ }).click();
await page.getByRole('button', { name: 'Очистить', exact: true }).waitFor({ timeout: 30000 });
await page.waitForFunction(() => ![...document.querySelectorAll('button')].find(b => b.textContent === 'Очистить')?.disabled);
await page.screenshot({ path: 'test-results/native-cleanup.png' });
const result = await page.evaluate(async () => {
  const snapshot = await window.__TAURI_INTERNALS__.invoke('snapshot', { gamePath: null });
  const scan = await window.__TAURI_INTERNALS__.invoke('scan_targets');
  const groups = {};
  for (const target of scan.targets) groups[target.group] = (groups[target.group] ?? 0) + target.size;
  return { roots: snapshot.environment.gameRoots.length, profiles: snapshot.environment.profileRoots.length,
    groups, errors: scan.errors.length, files: scan.totalFiles, bytes: scan.totalBytes,
    accountOptIn: scan.targets.filter(t => t.group === 'account').every(t => !t.defaultOn),
    fontLoaded: document.fonts.check('32px Warface') };
});
assert.ok(result.roots > 0);
assert.ok(result.files > 0);
const known = ['logs', 'caches', 'crash', 'launcher', 'updates', 'account'];
assert.ok(Object.keys(result.groups).every(g => known.includes(g)));
assert.ok(result.accountOptIn);
assert.ok(result.groups.logs > 0 && result.groups.caches > 0);
assert.deepEqual(errors, []);
console.log(JSON.stringify(result));
await page.getByRole('button', { name: 'Закрыть', exact: true }).click();
await browser.close();
