#!/usr/bin/env node
// Release gate: every configured napi target must have produced its artifact
// before the root package may be published. napi-rs only warns (and
// continues) when an expected target file is missing, so this explicit check
// is what makes a partial release impossible to ship silently.
//
// Usage: node scripts/check-artifacts.mjs [npm-dir]
// Exits non-zero with a report when anything is missing.
import { readdirSync, existsSync } from 'node:fs'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'

const npmDir = process.argv[2] ?? './npm'
const pkg = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'))
const { binaryName, targets } = pkg.napi

// napi-rs platform suffix per target triple (must match `napi create-npm-dirs`).
const PLATFORM_OF = {
  'x86_64-apple-darwin': 'darwin-x64',
  'aarch64-apple-darwin': 'darwin-arm64',
  'x86_64-pc-windows-msvc': 'win32-x64-msvc',
  'i686-pc-windows-msvc': 'win32-ia32-msvc',
  'aarch64-pc-windows-msvc': 'win32-arm64-msvc',
  'x86_64-unknown-linux-gnu': 'linux-x64-gnu',
  'aarch64-unknown-linux-gnu': 'linux-arm64-gnu',
  'armv7-unknown-linux-gnueabihf': 'linux-arm-gnueabihf',
  'x86_64-unknown-linux-musl': 'linux-x64-musl',
  'aarch64-unknown-linux-musl': 'linux-arm64-musl',
  'aarch64-linux-android': 'android-arm64',
  'armv7-linux-androideabi': 'android-arm-eabi',
  'x86_64-linux-android': 'android-x64',
  'x86_64-unknown-freebsd': 'freebsd-x64',
  'wasm32-wasip1-threads': 'wasm32-wasi',
}

let failures = 0
for (const target of targets) {
  const platform = PLATFORM_OF[target]
  if (!platform) {
    console.error(`FAIL: no platform mapping for target ${target} (update scripts/check-artifacts.mjs)`)
    failures++
    continue
  }
  const dir = join(npmDir, platform)
  if (!existsSync(dir)) {
    console.error(`FAIL: missing package dir ${dir}`)
    failures++
    continue
  }
  // The directory carries the platform suffix; the npm package *name*
  // inside must be `<binaryName>-<platform>`.
  const inner = JSON.parse(await readFile(join(dir, 'package.json'), 'utf8'))
  const expectedName = `${binaryName}-${platform}`
  if (inner.name !== expectedName) {
    console.error(`FAIL: ${dir}/package.json name is ${inner.name}, expected ${expectedName}`)
    failures++
    continue
  }
  const files = readdirSync(dir)
  const hasNative = files.some((f) => f.endsWith('.node'))
  const hasWasm = files.some((f) => f.endsWith('.wasm'))
  if (!hasNative && !hasWasm) {
    console.error(`FAIL: ${dir} contains no .node or .wasm artifact (saw: ${files.join(', ') || '<empty>'})`)
    failures++
    continue
  }
  if (target === 'wasm32-wasip1-threads') {
    for (const required of ['.wasi-browser.js', 'wasi-worker']) {
      if (!files.some((f) => f.includes(required))) {
        console.error(`FAIL: ${dir} is missing WASI support file matching *${required}*`)
        failures++
      }
    }
  }
  if (failures === 0) console.log(`ok: ${dir} (${files.length} files)`)
}

if (failures > 0) {
  console.error(`\n${failures} artifact gate failure(s): refusing to publish`)
  process.exit(1)
}
console.log(`\nArtifact gate passed for all ${targets.length} targets`)
