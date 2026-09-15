const { validateWgsl, translate, nagaVersion } = require('./index')
const pkg = require('./package.json')

// Framework-free smoke test for the built package through the CJS loader
// path (ava covers ESM + behavior). Fails without devDependencies installed,
// so it doubles as the clean-install check: load, identify, translate.
console.assert(
  typeof nagaVersion() === 'string' && nagaVersion() === pkg.naga,
  `nagaVersion() should report the wrapped upstream (${pkg.naga})`,
)

const source = `
@fragment
fn main_fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
`

validateWgsl(source)
const glsl = translate({ from: 'wgsl', to: 'glsl', source })
console.assert(typeof glsl === 'string' && glsl.includes('_fs2p_location0'), 'GLSL output mismatch')

const spv = translate({ from: 'wgsl', to: 'spv', source })
console.assert(spv instanceof Uint8Array, 'SPIR-V output should be a Uint8Array')

console.info('Simple test passed')
