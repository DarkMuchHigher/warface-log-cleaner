import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';

function fontNames(path: string) {
  const data = readFileSync(path);
  let offset = 0;
  for (let i = 0; i < data.readUInt16BE(4); i++) {
    const position = 12 + i * 16;
    if (data.toString('ascii', position, position + 4) === 'name') offset = data.readUInt32BE(position + 8);
  }
  expect(offset).toBeGreaterThan(0);
  const count = data.readUInt16BE(offset + 2);
  const strings = offset + data.readUInt16BE(offset + 4);
  const names: Record<number, string> = {};
  for (let i = 0; i < count; i++) {
    const position = offset + 6 + i * 12;
    if (data.readUInt16BE(position) !== 3) continue;
    const id = data.readUInt16BE(position + 6);
    const length = data.readUInt16BE(position + 8);
    const start = strings + data.readUInt16BE(position + 10);
    const text = Buffer.from(data.subarray(start, start + length)).swap16().toString('utf16le');
    names[id] = text;
  }
  return names;
}

test('supplied fonts are valid SFNT files', () => {
  for (const file of ['public/warface-ru.ttf', 'public/warface-en.ttf']) {
    const names = fontNames(file);
    expect(names[1]).toBeTruthy();
    console.log(JSON.stringify({ file, family: names[1], copyright: names[0], license: names[13], licenseUrl: names[14] }));
  }
});

test('package versions and workflow artifact names agree', () => {
  const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
  const tauri = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
  const cargo = readFileSync('src-tauri/Cargo.toml', 'utf8');
  const workflow = readFileSync('.github/workflows/windows.yml', 'utf8');
  expect(tauri.version).toBe(pkg.version);
  expect(cargo).toContain(`version = "${pkg.version}"`);
  expect(workflow).toContain('SHA256SUMS.txt');
  expect(workflow).toContain('contents: read');
  expect(workflow).not.toContain('pull_request_target');
  expect(workflow).not.toContain('--clobber');
});
