# naga-js

JavaScript bindings for [Naga](https://github.com/gfx-rs/wgpu/tree/trunk/naga)
(`naga` v30) — validate and translate GPU shaders from Node.js and browsers.
Built with [napi-rs](https://napi.rs): one `#[napi]` API compiles to native
`.node` addons plus a `wasm32-wasip1-threads` fallback for browsers and
platforms without a prebuilt binary.

Supported frontends: WGSL, GLSL (440+, Vulkan semantics), SPIR-V.
Supported backends: WGSL, GLSL, HLSL, MSL, SPIR-V, DOT.

## Install

```sh
pnpm add naga-js
```

## Usage

```ts
import { validateWgsl, translate } from 'naga-js'

const source = `
@fragment
fn main_fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
`

validateWgsl(source) // throws on error, with line info in the message

const glsl = translate({
  from: 'wgsl',
  to: 'glsl',
  source,
  stage: 'fragment',
  entryPoint: 'main_fs',
})

const spv = translate({ from: 'wgsl', to: 'spv', source }) // Uint8Array (LE bytes)
const back = translate({ from: 'spv', to: 'wgsl', source: spv })

nagaVersion() // e.g. '30.0.1' — always equals the package version
```

For programmatic error handling, the non-throwing variants return structured
diagnostics instead of raising — same pipeline, `{ ok, output?, error? }`
shape with `error.labels[]` carrying 1-based `{ line, column, length,
message }` spans for editors and tooling:

```ts
import { translateDetailed } from 'naga-js'

const result = translateDetailed({ from: 'wgsl', to: 'wgsl', source: 'fn broken( { nonsense' })
// { ok: false, error: { kind: 'parse', message: '…', labels: [{ line: 1, column: 12, length: 1, message: 'expected identifier' }], notes: [] } }
```

### Options

`translate(options)` where `options` is:

| Field             | Type                                                    | Notes                                                                                                                                                             |
| ----------------- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `from`            | `'wgsl' \| 'glsl' \| 'spv'`                             | Input language.                                                                                                                                                   |
| `to`              | `'wgsl' \| 'glsl' \| 'hlsl' \| 'msl' \| 'spv' \| 'dot'` | Output language. `spv` returns a `Uint8Array`, everything else returns `string`.                                                                                  |
| `source`          | `string \| Uint8Array`                                  | Text for `wgsl`/`glsl`, raw SPIR-V bytes for `spv`.                                                                                                               |
| `stage`           | `'vertex' \| 'fragment' \| 'compute'`                   | Required for GLSL _input_; required for `glsl`/`spv` output when the module has more than one entry point. A sole entry point is used automatically when omitted. |
| `entryPoint`      | `string`                                                | Entry-point name. Same resolution rules as `stage`.                                                                                                               |
| `glslVersion`     | `string`                                                | GLSL _output_ version, e.g. `"310 es"` (default) or `"450"`.                                                                                                      |
| `hlslShaderModel` | `string`                                                | e.g. `"5_1"` (default), `"6_0"` … `"6_9"`.                                                                                                                        |
| `defines`         | `Record<string, string>`                                | Preprocessor definitions for GLSL _input_.                                                                                                                        |
| `validation`      | `ValidationOptions`                                     | Validator strictness (below). Defaults to everything enabled.                                                                                                     |

### Validation

`validateWgsl(source, validation?)` and `translate()`/`translateDetailed()`
(via `options.validation`) accept the same knobs. Every field is optional and
defaults to `"all"`; pass `"all"` explicitly for the same effect, or `[]` for
none. Names mirror naga exactly (lowercased) — TypeScript gets literal-union
types, and unknown names fail at runtime listing the valid ones:

```ts
validateWgsl(source, {
  capabilities: ['shader_float16'], // only allow f16 instead of everything
  flags: ['all'],
  subgroupStages: ['compute'],
  subgroupOperations: ['basic', 'vote'],
})
```

Use this to validate against what a target platform actually accepts rather
than everything naga can express — e.g. a shader using `f16` fails
validation with `capabilities: []` while passing by default. The full
capability list (44 entries: `subgroup`, `mesh_shader`, `ray_query`,
`cooperative_matrix`, …) is in the generated `index.d.ts`
`ValidationOptions` type.

Errors are thrown as `Error`s whose message contains the full naga
diagnostic (codespan-formatted with line numbers for WGSL/GLSL parse errors,
`line/column` suffix for validation errors).

### Browser

The package's `browser` entry re-exports the prebuilt WASI package, so
bundlers (Vite, webpack) pick it up automatically. The threads build needs
cross-origin isolation on the serving page:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

## Support

One binary per listed platform, built against Node-API 4. CI tests Node.js 22
on macOS arm64/x64, Windows x64, Linux x64 glibc/musl, plus the
WASI-threads fallback (`NAPI_RS_FORCE_WASI=true`). Other
Node-API-compatible releases are expected to work but are not in the blocking
matrix.

| Runtime               | Status                                                                                                                        |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Node.js native addon  | Primary, blocking CI.                                                                                                         |
| Node.js WASI fallback | Tested in CI (`NAPI_RS_FORCE_WASI=true` over a workspace with no native artifact present, so only WASM can satisfy the load). |
| Browser WASI          | Available via the `browser` entry; needs COOP/COEP (see above).                                                               |
| Bun native            | Tested in CI (blocking, pinned Bun 1.3.14 — the full suite passes; bump deliberately).                                        |
| Deno native           | Smoke-tested only, not claimed.                                                                                               |
| Bun/Deno WASI         | Not supported (known upstream gaps, napi-rs#2965).                                                                            |

| Platform target             | CI                                                       |
| --------------------------- | -------------------------------------------------------- |
| macOS x64/arm64             | Native runners.                                          |
| Windows x64/x86/arm64       | Native runner.                                           |
| Linux glibc x64/arm64/armv7 | `--use-napi-cross` (glibc 2.17 floor).                   |
| Linux musl x64/arm64        | `-x` + zig.                                              |
| Android arm64/armv7         | Cross-built on Ubuntu (NDK); build-verified (see below). |
| FreeBSD x64                 | Tested in a FreeBSD VM job.                              |
| WASI threads                | Ubuntu, no cross flag.                                   |

## Versioning

`naga-js` versions track upstream `naga` the way driver wrappers like
`chromedriver` track their upstream: normally `naga-js@X.Y.Z` wraps exactly
`naga@X.Y.Z`, so the number tells you which upstream you get. The wrapped
version is recorded in the package.json `naga` field and reported at runtime
by `nagaVersion()` — those two always agree, and CI enforces it
(`node scripts/sync-version.mjs --check`).

Because npm semver has only three segments (a `+build` suffix is silently
stripped by `npm publish` — verified — so it can't carry wrapper revisions),
a binding-only fix ships as a **patch bump ahead of upstream**
(e.g. `naga-js@30.0.2` wrapping `naga@30.0.1`) so it flows through `latest`
instead of hiding on a prerelease tag. The sync script owns the edge cases:

- upstream bump (`naga` 30.0.1 → 30.0.2): package adopts `30.0.2`;
- upstream leapfrogs a hotfix (`naga` → 30.0.3 while package is at `30.0.2`):
  package adopts `30.0.3`;
- collision (upstream releases the number a hotfix already occupies):
  package steps one patch ahead (`30.0.3`), so bits are never republished
  under one version;
- major/minor drift is never auto-fixed — that needs a human decision.

Upstream updates arrive via Renovate: `naga` patch bumps automerge once CI
(including the golden snapshot tests) is green; minor/major bumps need human
review because backend output text can change. `sync-version.yml` runs the
sync on those PRs automatically.

## Develop

Requirements: Rust 1.88+, Node.js 22.13+ (or 24+) for the CLI, pnpm.

```sh
pnpm install
pnpm build   # native release addon for the host
pnpm test    # ava suite (22 tests: golden outputs, round-trips, error cases)
node simple-test.js
```

Useful checks: `cargo clippy --all-targets`, `cargo fmt --check`,
`pnpm exec tsc --noEmit -p __test__/tsconfig.json` (strict types over the
generated `index.d.ts`), `pnpm exec oxlint`,
`node scripts/check-artifacts.mjs ./npm` (release gate, CI-only data).

Git hooks (lefthook) install automatically on `pnpm install`: fast
lint/format/version-sync on pre-commit, conventional-commit linting on
commit-msg, clippy + tsc on pre-push. Commits follow
`@commitlint/config-conventional` (`feat:`, `fix:`, `chore:`, …); bare
version commits (`30.0.1`) are exempt because the publish workflow gates on
them. Run any group manually with `lefthook run <hook>`.

## Release

Releases go through `.github/workflows/CI.yml`. One-time setup before the
first release:

1. Create the GitHub repo and point `package.json#repository` at it (npm
   provenance requires the match).
2. Reserve **all** platform package names (`naga-js-darwin-x64`, …,
   `naga-js-wasm32-wasi`) — unscoped sibling names can otherwise be squatted.
3. Create a short-lived granular access token (All packages, publish+stage,
   ~7 days) as the `NPM_TOKEN` Actions secret. This bootstrap token exists
   only because trusted publishers attach to packages that don't exist yet —
   the first publish creates them. Migrate to OIDC immediately after (next
   section) and revoke it.

### Trusted publishing (OIDC, no long-lived tokens)

Direct-publish tokens are deprecated by npm (removed January 2027), so CI
publishes via OpenID Connect once configured. The workflow is already
prepared: `id-token: write` is set, the publish job installs npm ≥ 11.5.1
(OIDC minimum), and `NPM_TOKEN` is used only when the secret exists — npm
prefers OIDC and falls back to the token otherwise.

1. Publish once with the bootstrap token so all 15 packages exist
   (`naga-js` + the 14 `naga-js-<platform>` packages).
2. On npmjs.com, for **each** of the 15 packages: Settings → Trusted
   publishing → Add GitHub Actions publisher with organization/user
   `akashdeep000`, repository `naga-js`, workflow filename `CI.yml`
   (exactly — extension included), no environment, direct `npm publish`
   allowed.
3. Cut a `*-next.0` prerelease and confirm it publishes with a provenance
   badge and no token involved.
4. Per package: Settings → Publishing access → require 2FA and disallow
   tokens. Delete the `NPM_TOKEN` secret and revoke the bootstrap token on
   npmjs.com. From here nothing secret remains to rotate.

To cut a release once `main` is green at the desired (already synced) version:

```sh
npm version "$(node -p "require('./package.json').version")" --allow-same-version -m "%s"
git push --follow-tags
```

(Binding-only hotfix: `npm version patch -m "%s"` — the version runs one
patch ahead of upstream, which the sync check accepts; the `naga` field keeps
telling the truth.) CI builds every target, runs all test jobs, collects
artifacts, runs the `scripts/check-artifacts.mjs` gate (refuses partial
releases), publishes platform packages, then the root. Verify afterwards with
`npm view naga-js@<version> --json` plus clean installs on
glibc/musl/macOS/Windows. See napi-rs
[Release native packages](https://napi.rs/docs/deep-dive/release) for the
recovery procedure — never rebuild binaries under a published version.

## Roadmap

- Promote Android to runtime-tested once either becomes true: GitHub-hosted
  ARM runners that can boot the emulator (there is no Linux-ARM64 emulator
  host build today), or napi-rs loader support for an x86_64 Android target
  (probed: the CLI builds the binary but the generated `index.js` only
  resolves `arm64`/`arm` on Android, so an x86_64 artifact can never load).
  Manual verification path in the meantime: Termux `nodejs` + `adb push` the
  matching `.node` next to `index.js`, then `node simple-test.js` on-device.
