#!/usr/bin/env node
// Version policy (chromedriver convention, adapted to 3-segment semver):
//
//   `naga-js@X.Y.Z` wraps the upstream recorded in its `naga` field.
//   Normally X.Y.Z equals that upstream version (lockstep). A binding-only
//   fix may bump the package *patch* ahead of upstream so it ships on
//   `latest`; the `naga` field (and `nagaVersion()` at runtime) always tell
//   the truth about the wrapped upstream.
//
// Invariants enforced by `--check` (CI lint):
//   1. package.json `naga` === resolved naga version in Cargo.lock.
//   2. package major.minor === naga major.minor.
//   3. package patch >= naga patch.
//
// Sync behavior (no `--check`):
//   - align the `naga` field with Cargo.lock;
//   - if the version then violates invariant 2/3, reset it to the naga
//     version (normal upstream-bump path);
//   - else, if the `naga` field just changed while the version already
//     equals the new naga version numerically (a previous binding hotfix
//     colliding with an upstream release of the same number), bump the
//     package patch by one so bits are never republished under one version.
//
// Usage:
//   node scripts/sync-version.mjs          # sync package.json from Cargo.lock
//   node scripts/sync-version.mjs --check  # fail (exit 1) when out of sync
import { readFileSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

export function parseVersion(v) {
  const m = String(v)
    .trim()
    .match(/^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$/)
  if (!m) throw new Error(`not a semver X.Y.Z version: ${v}`)
  return { major: Number(m[1]), minor: Number(m[2]), patch: Number(m[3]) }
}

/**
 * Pure sync step. Returns `{ version, naga, changed, reason }`.
 * Throws when the current state is unrecoverable (major/minor drift that a
 * sync must not paper over — that needs a human decision).
 */
export function computeSync(pkgVersion, pkgNaga, lockNaga) {
  const lock = parseVersion(lockNaga)
  const current = parseVersion(pkgVersion)
  const nagaChanged = pkgNaga !== lockNaga

  if (current.major !== lock.major || current.minor !== lock.minor) {
    throw new Error(
      `major/minor drift: package is ${pkgVersion} but naga is ${lockNaga}. ` +
        `Align them explicitly (breaking change), this script will not guess.`,
    )
  }

  let version = pkgVersion
  let reason = 'in sync'
  if (current.patch < lock.patch) {
    // Normal upstream-bump path: adopt the naga version.
    version = lockNaga
    reason = `adopted naga version ${lockNaga}`
  } else if (nagaChanged && version === lockNaga) {
    // Collision: a binding hotfix already occupies this number for different
    // bits (it wrapped an older naga). Step one patch ahead.
    version = `${lock.major}.${lock.minor}.${lock.patch + 1}`
    reason = `binding revision ${pkgVersion} collided with naga ${lockNaga}; stepped ahead`
  } else if (nagaChanged) {
    reason = `upstream moved to ${lockNaga}; binding revision ${pkgVersion} already ahead`
  } else if (current.patch > lock.patch) {
    reason = `binding revision ${pkgVersion} ahead of naga ${lockNaga}`
  }
  return { version, naga: lockNaga, changed: version !== pkgVersion || nagaChanged, reason }
}

export function nagaVersionFromLock(lockText) {
  const lines = lockText.split('\n')
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim() === 'name = "naga"') {
      for (let j = i + 1; j < Math.min(i + 5, lines.length); j++) {
        const m = lines[j].trim().match(/^version = "([^"]+)"$/)
        if (m) return m[1]
      }
    }
  }
  throw new Error('could not find resolved naga version in Cargo.lock')
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href
if (isMain) {
  const root = new URL('..', import.meta.url)
  const check = process.argv.includes('--check')
  const lockNaga = nagaVersionFromLock(readFileSync(new URL('Cargo.lock', root), 'utf8'))
  const pkgUrl = new URL('package.json', root)
  const pkg = JSON.parse(readFileSync(pkgUrl, 'utf8'))

  const result = computeSync(pkg.version, pkg.naga, lockNaga)
  if (check) {
    if (result.changed) {
      console.error(
        `out of sync (${result.reason}). Run \`node scripts/sync-version.mjs\` and commit the result.`,
      )
      process.exit(1)
    }
    console.log(`in sync: naga-js ${pkg.version} wraps naga ${pkg.naga}`)
  } else {
    pkg.version = result.version
    pkg.naga = result.naga
    writeFileSync(pkgUrl, JSON.stringify(pkg, null, 2) + '\n')
    console.log(`${result.reason}; package is now ${result.version} (naga ${result.naga})`)
  }
}
