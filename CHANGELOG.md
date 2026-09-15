# Changelog

Entries are newest-first. The `naga-js` version tracks the wrapped upstream
`naga` version (see README "Versioning"); each entry names the upstream it
wraps. Automated upstream bumps prepend their entry on release.

## 30.0.1

Initial release. JavaScript bindings for `naga` 30.0.1: WGSL validation
(`validateWgsl`, plus non-throwing `validateWgslDetailed` with structured
`labels[]`), full translation (`translate` / `translateDetailed`) across
WGSL, GLSL, SPIR-V inputs and WGSL, GLSL, HLSL, MSL, SPIR-V, DOT outputs,
validator strictness knobs (`ValidationOptions`), native addons for 13
platforms plus a WASI-threads fallback for Node.js and browsers.
