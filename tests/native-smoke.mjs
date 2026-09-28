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
  return { roots: snapshot.environment.gameRoots.length, profiles: snapshot.environment.profileRoots.length,
    groups: [...new Set(scan.targets.map(t => t.group))], errors: scan.errors.length,
    files: scan.totalFiles, bytes: scan.totalBytes, fontLoaded: document.fonts.check('32px Warface') };
});
assert.ok(result.roots > 0);
assert.ok(result.files > 0);
assert.ok(result.groups.every(g => ['logs', 'caches', 'crash', 'launcher', 'updates'].includes(g)));
assert.deepEqual(errors, []);
console.log(JSON.stringify(result));
await page.getByRole('button', { name: 'Закрыть', exact: true }).click();
await browser.close();
