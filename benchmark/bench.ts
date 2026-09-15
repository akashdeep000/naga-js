import { Bench } from 'tinybench'

import { translate, validateWgsl } from '../index.js'

const source = `
@fragment
fn main_fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
`

const bench = new Bench()

bench.add('validateWgsl', () => {
  validateWgsl(source)
})

bench.add('translate wgsl -> glsl', () => {
  translate({ from: 'wgsl', to: 'glsl', source })
})

bench.add('translate wgsl -> spv', () => {
  translate({ from: 'wgsl', to: 'spv', source })
})

await bench.run()

console.table(bench.table())
