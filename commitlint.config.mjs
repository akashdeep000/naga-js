export default {
  extends: ['@commitlint/config-conventional'],
  // Release and automation commits are not conventional by design:
  // - `npm version` writes the bare version (`30.0.1`, optionally `v`-prefixed)
  //   because the publish workflow gates on that exact commit message;
  // - `sync-version.yml` writes `chore: sync package version with naga`
  //   (already conventional, listed here only as documentation).
  ignores: [(message) => /^v?\d+\.\d+\.\d+(-\S+)?(\s|$)/m.test(message)],
}
