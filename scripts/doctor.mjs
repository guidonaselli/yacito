#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import os from 'node:os';

const checks = [
  { name: 'node', command: 'node', args: ['--version'], required: true },
  { name: 'npm', command: 'npm', args: ['--version'], required: true },
  { name: 'rustc', command: 'rustc', args: ['--version'], required: true },
  { name: 'cargo', command: 'cargo', args: ['--version'], required: true },
  { name: 'httpyac', command: 'httpyac', args: ['--version'], required: false },
];

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    shell: os.platform() === 'win32',
  });
  return {
    ok: result.status === 0,
    output: `${result.stdout ?? ''}${result.stderr ?? ''}`.trim(),
    error: result.error,
  };
}

let failedRequired = false;
console.log('Yacito doctor\n');

for (const check of checks) {
  const result = run(check.command, check.args);
  const icon = result.ok ? '✓' : check.required ? '✗' : '!';
  const label = check.required ? 'required' : 'optional runtime';
  console.log(`${icon} ${check.name.padEnd(8)} ${label.padEnd(16)} ${result.ok ? result.output : 'not found'}`);
  if (!result.ok && check.required) failedRequired = true;
}

console.log('\nNotes:');
console.log('- Yacito can start without httpyac, but Send requires it.');
console.log('- Install httpyac with: npm run setup:httpyac');
console.log('- On Linux, Tauri may require WebKit/GTK system packages.');
console.log('- On Windows, install Microsoft C++ Build Tools if Rust/Tauri asks for them.');

process.exit(failedRequired ? 1 : 0);
