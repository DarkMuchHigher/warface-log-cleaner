import { test, expect, type Page } from '@playwright/test';

async function mockDesktop(page: Page) {
  await page.addInitScript(() => {
    const w = window as any;
    w.isTauri = true;
    w.calls = [];
    w.listeners = new Set();
    w.failure = '';
    let callback = 0;
    localStorage.setItem('wlc.lang', 'ru');
    const target = (id: string, group: string, path: string, size: number, files: number) => ({
      id, group, path, size, files, pattern: null, kind: 'file', risk: group === 'logs' ? 'safe' : 'caution',
      admin: false, defaultOn: group === 'logs', exists: true, scanned: true, note: null,
    });
    const targets = [
      target('log.game@demo', 'logs', 'D:\\Games\\Warface\\Game.log', 350000, 1),
      target('log.backups@demo', 'logs', 'D:\\Games\\Warface\\LogBackups', 128000000, 42),
      target('cache.models@demo', 'caches', 'C:\\Users\\Player\\Saved Games\\My Games\\Warface\\modelscache', 1400000000, 1200),
      target('cache.shaders@demo', 'caches', 'C:\\Users\\Player\\Saved Games\\My Games\\Warface\\Shaders', 220000000, 150),
      target('crash.dumps@demo', 'crash', 'D:\\Games\\Warface', 18000000, 2),
      target('gc.main@demo', 'launcher', 'C:\\Users\\Player\\AppData\\Local\\GameCenter\\main.log', 12000000, 1),
      target('upd.warface@demo', 'updates', 'D:\\VK Play\\Distrib\\packages\\warface', 450000000, 12),
      target('account.launcher_ini@demo', 'account', 'C:\\Users\\Player\\AppData\\Local\\GameCenter\\GameCenter.ini', 32000, 1),
    ];
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
      transformCallback: () => ++callback,
      invoke: async (command: string, args: any) => {
        w.calls.push({ command, args });
        if (command === 'plugin:event|listen') { w.listeners.add(args.handler); return args.handler; }
        if (command === 'plugin:event|unlisten') { w.listeners.delete(args.eventId); return; }
        if (command === 'snapshot') return { environment: { user: 'Player', isAdmin: false,
          gameRoots: ['D:\\Games\\Warface'], profileRoots: ['C:\\Users\\Player\\Saved Games\\My Games\\Warface'],
          freeSpace: 96000000000, running: [], notes: [] }, targets };
        if (command === 'scan_targets') return { targets, totalBytes: 1766350000, totalFiles: 1395, durationMs: 250, errors: [] };
        if (command === 'preview_targets') return { planId: 7, bytes: 350000, files: [{ path: 'D:\\Games\\Warface\\Game.log', size: 350000, targetId: targets[0].id }], errors: [] };
        if (command === 'clean_targets') {
          if (w.failure) throw w.failure;
          return { bytes: 350000, files: 1, skipped: 0, errors: [], targets, durationMs: 10 };
        }
      },
    };
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: (_event: string, id: number) => w.listeners.delete(id) };
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Сканировать', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Очистить', exact: true })).toBeEnabled();
}

test('Warface-only selection, preview and explicit confirmation', async ({ page }) => {
  await mockDesktop(page);
  await expect(page.getByRole('checkbox', { name: 'Логи игры', exact: true })).toBeChecked();
  await page.getByRole('button', { name: 'Выбрать всё', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'Кэш игры', exact: true })).toBeChecked();
  await page.getByRole('button', { name: 'Только логи', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'Кэш игры', exact: true })).not.toBeChecked();
  await page.getByRole('button', { name: 'Очистить', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Отмена', exact: true })).toBeFocused();
  expect(await page.evaluate(() => (window as any).calls.filter((c: any) => c.command === 'clean_targets').length)).toBe(0);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Очистить', exact: true })).toBeFocused();
  await page.getByRole('button', { name: 'Очистить', exact: true }).click();
  await page.getByRole('button', { name: 'Удалить эти файлы', exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: 'Удалено:' })).toBeVisible();
  const calls = await page.evaluate(() => (window as any).calls.filter((c: any) => c.command === 'clean_targets'));
  expect(calls).toEqual([{ command: 'clean_targets', args: { planId: 7, confirmed: true } }]);
  expect(await page.evaluate(() => (window as any).listeners.size)).toBe(1);
});

test('close-game failure stays visible in confirmation dialog', async ({ page }) => {
  await mockDesktop(page);
  await page.evaluate(() => { (window as any).failure = 'close_game_and_launcher'; });
  await page.getByRole('button', { name: 'Очистить', exact: true }).click();
  await page.getByRole('button', { name: 'Удалить эти файлы', exact: true }).click();
  await expect(page.getByRole('dialog').getByRole('alert')).toContainText('Закройте Warface');
  await expect(page.getByRole('dialog')).toBeVisible();
});

for (const locale of ['RU', 'EN']) {
  for (const [width, height] of [[720, 600], [600, 460], [1060, 740]]) {
    test(`layout and fonts ${locale} ${width}x${height}`, async ({ page }) => {
      const errors: string[] = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.setViewportSize({ width, height });
      await mockDesktop(page);
      await page.getByRole('button', { name: locale, exact: true }).click();
      await page.evaluate(() => document.fonts.ready);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      const main = page.locator('main');
      expect(await main.evaluate(e => e.scrollWidth <= e.clientWidth)).toBe(true);
      const font = await page.locator('.group-expand strong').first().evaluate(e => getComputedStyle(e).fontFamily);
      expect(font).toContain('Warface');
      expect(errors).toEqual([]);
      await page.screenshot({ path: `test-results/cleanup-${locale}-${width}.png` });
      if (width === 1060) {
        await page.getByRole('button', { name: locale === 'RU' ? 'Очистить' : 'Clean', exact: true }).click();
        await page.screenshot({ path: `test-results/preview-${locale}.png` });
      }
    });
  }
}

test('browser preview cannot perform filesystem operations', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'EN', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Scan', exact: true })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Clean', exact: true })).toBeDisabled();
});
