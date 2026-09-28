import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: '../tests',
  outputDir: '../test-results',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  use: { baseURL: 'http://127.0.0.1:1422', viewport: { width: 1060, height: 740 }, screenshot: 'only-on-failure' },
  webServer: {
    command: 'pnpm dev --port 1422',
    cwd: '..',
    url: 'http://127.0.0.1:1422',
    reuseExistingServer: !process.env.CI,
    timeout: 30000,
  },
});
