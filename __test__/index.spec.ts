import test from 'ava'
import { readFileSync } from 'fs'

import { nagaVersion, translate, translateDetailed, validateWgsl, validateWgslDetailed } from '../index.js'

const FRAGMENT_WGSL = `
@fragment
fn main_fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
`

const EXPECTED_GLSL = `#version 310 es

precision highp float;
precision highp int;

layout(location = 0) out vec4 _fs2p_location0;

void main() {
    _fs2p_location0 = vec4(1.0, 1.0, 1.0, 1.0);
    return;
}

`

test('nagaVersion reports the wrapped upstream recorded in package.json', (t) => {
  const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'))
  t.is(nagaVersion(), pkg.naga)
})

test('validateWgsl accepts a valid shader', (t) => {
  t.notThrows(() => validateWgsl(FRAGMENT_WGSL))
})

test('validateWgsl rejects an invalid shader with location info', (t) => {
  const err = t.throws(() => validateWgsl('fn broken( { nonsense'), { instanceOf: Error })
  t.truthy(err)
  t.true(err!.message.includes('WGSL parse error'))
})

test('translate WGSL to GLSL matches the naga reference output', (t) => {
  const out = translate({
    from: 'wgsl',
    to: 'glsl',
    source: FRAGMENT_WGSL,
    stage: 'fragment',
    entryPoint: 'main_fs',
  })
  t.is(out, EXPECTED_GLSL)
})

test('translate infers a sole entry point when stage is omitted', (t) => {
  const out = translate({ from: 'wgsl', to: 'glsl', source: FRAGMENT_WGSL })
  t.is(out, EXPECTED_GLSL)
})

test('translate WGSL to WGSL round-trips the entry point', (t) => {
  const out = translate({ from: 'wgsl', to: 'wgsl', source: FRAGMENT_WGSL })
  t.true(typeof out === 'string' && out.includes('main_fs'))
})

test('translate WGSL to HLSL produces a shader model 5.1 pixel shader', (t) => {
  const out = translate({ from: 'wgsl', to: 'hlsl', source: FRAGMENT_WGSL })
  t.true(typeof out === 'string' && out.includes('main_fs'))
})

test('translate WGSL to MSL produces metal source', (t) => {
  const out = translate({ from: 'wgsl', to: 'msl', source: FRAGMENT_WGSL })
  t.true(typeof out === 'string' && out.includes('main_fs'))
})

test('translate WGSL to SPIR-V returns words with the SPIR-V magic', (t) => {
  const out = translate({ from: 'wgsl', to: 'spv', source: FRAGMENT_WGSL })
  t.true(out instanceof Uint8Array)
  if (out instanceof Uint8Array) {
    const words = new Uint32Array(out.buffer, out.byteOffset, out.byteLength / 4)
    t.is(words[0], 0x07230203)
  }
})

test('translate SPIR-V back to WGSL round-trips', (t) => {
  const spv = translate({ from: 'wgsl', to: 'spv', source: FRAGMENT_WGSL })
  t.true(spv instanceof Uint8Array)
  if (!(spv instanceof Uint8Array)) return
  const out = translate({ from: 'spv', to: 'wgsl', source: spv })
  t.true(typeof out === 'string' && out.includes('main_fs'))
})

test('translate WGSL to DOT produces a graph', (t) => {
  const out = translate({ from: 'wgsl', to: 'dot', source: FRAGMENT_WGSL })
  t.true(typeof out === 'string' && out.includes('digraph'))
})

test('translate rejects an unknown entry point', (t) => {
  const err = t.throws(
    () =>
      translate({
        from: 'wgsl',
        to: 'glsl',
        source: FRAGMENT_WGSL,
        stage: 'fragment',
        entryPoint: 'missing',
      }),
    { instanceOf: Error },
  )
  t.true(err!.message.includes('not found'))
})

test('translate rejects a malformed glslVersion', (t) => {
  const err = t.throws(
    () =>
      translate({
        from: 'wgsl',
        to: 'glsl',
        source: FRAGMENT_WGSL,
        glslVersion: 'bogus',
      }),
    { instanceOf: Error },
  )
  t.true(err!.message.includes('glslVersion'))
})

const DUP_BINDINGS_WGSL = `@group(0) @binding(0) var<uniform> a: f32;
@group(0) @binding(0) var<uniform> b: f32;
@fragment
fn main() -> @location(0) vec4<f32> { return vec4<f32>(a + b, 0.0, 0.0, 1.0); }
`

const F16_WGSL = `enable f16;
@compute @workgroup_size(1)
fn main() {
    let x: f16 = 1.0h;
}
`

test('translateDetailed reports success with output and no error', (t) => {
  const result = translateDetailed({ from: 'wgsl', to: 'wgsl', source: FRAGMENT_WGSL })
  t.true(result.ok)
  t.true(typeof result.output === 'string' && result.output.includes('main_fs'))
  t.is(result.error, undefined)
})

test('translateDetailed reports parse errors with exact labels', (t) => {
  const result = translateDetailed({ from: 'wgsl', to: 'wgsl', source: 'fn broken( { nonsense' })
  t.false(result.ok)
  t.is(result.output, undefined)
  t.is(result.error?.kind, 'parse')
  t.true((result.error?.message.length ?? 0) > 0)
  t.deepEqual(result.error?.labels, [
    { line: 1, column: 12, length: 1, message: 'expected identifier' },
  ])
  t.deepEqual(result.error?.notes, [])
})

test('translateDetailed reports validation errors with spans', (t) => {
  const result = translateDetailed({ from: 'wgsl', to: 'wgsl', source: DUP_BINDINGS_WGSL })
  t.false(result.ok)
  t.is(result.error?.kind, 'validation')
  t.true(result.error?.message.includes('at line 2, column 23') ?? false)
  t.deepEqual(result.error?.labels, [
    { line: 2, column: 23, length: 20, message: 'naga::ir::GlobalVariable [1]' },
  ])
})

test('capabilities knob changes validation outcome', (t) => {
  t.true(translateDetailed({ from: 'wgsl', to: 'wgsl', source: F16_WGSL }).ok)
  const restricted = translateDetailed({
    from: 'wgsl',
    to: 'wgsl',
    source: F16_WGSL,
    validation: { capabilities: [] },
  })
  t.false(restricted.ok)
  t.is(restricted.error?.kind, 'validation')
})

test('flags knob changes validation outcome', (t) => {
  t.false(translateDetailed({ from: 'wgsl', to: 'wgsl', source: DUP_BINDINGS_WGSL }).ok)
  t.true(
    translateDetailed({ from: 'wgsl', to: 'wgsl', source: DUP_BINDINGS_WGSL, validation: { flags: [] } })
      .ok,
  )
})

test('validateWgsl accepts validator options as a second argument', (t) => {
  t.notThrows(() => validateWgsl(FRAGMENT_WGSL, {}))
  t.notThrows(() => validateWgsl(FRAGMENT_WGSL, { flags: ['all'], capabilities: ['all'] }))
  const err = t.throws(() => validateWgsl(F16_WGSL, { capabilities: [] }), { instanceOf: Error })
  t.true(err!.message.includes('validation error'))
})

test('validateWgslDetailed returns structured errors without throwing', (t) => {
  const ok = validateWgslDetailed(FRAGMENT_WGSL)
  t.true(ok.ok)
  t.is(ok.error, undefined)
  const bad = validateWgslDetailed('fn broken( { nonsense')
  t.false(bad.ok)
  t.is(bad.error?.kind, 'parse')
  t.is(bad.error?.labels[0]?.line, 1)
})

test('translateDetailed reports usage errors with no labels', (t) => {
  const result = translateDetailed({
    from: 'wgsl',
    to: 'glsl',
    source: FRAGMENT_WGSL,
    stage: 'fragment',
    entryPoint: 'missing',
  })
  t.false(result.ok)
  t.is(result.output, undefined)
  t.is(result.error?.kind, 'usage')
  t.true(result.error?.message.includes('not found') ?? false)
  t.deepEqual(result.error?.labels, [])
})

test('translate rejects unknown validation names with the valid list', (t) => {
  // TypeScript already rejects these at compile time; `as never` exercises
  // the runtime path that plain-JS and dynamic callers hit.
  const flagErr = t.throws(
    () =>
      translate({
        from: 'wgsl',
        to: 'glsl',
        source: FRAGMENT_WGSL,
        validation: { flags: ['bogus' as never] },
      }),
    { instanceOf: Error },
  )
  t.true(flagErr!.message.includes(`invalid flag 'bogus'`))
  t.true(flagErr!.message.includes('"bindings"'))
  const capErr = t.throws(
    () =>
      translate({
        from: 'wgsl',
        to: 'wgsl',
        source: FRAGMENT_WGSL,
        validation: { capabilities: ['nope' as never] },
      }),
    { instanceOf: Error },
  )
  t.true(capErr!.message.includes(`invalid capability 'nope'`))
})
