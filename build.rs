extern crate napi_build;

use std::fs;

fn main() {
  napi_build::setup();

  // Expose the resolved naga version (single source of truth: Cargo.lock) to
  // the crate, so `naga_version()` always reports the wrapped upstream.
  println!("cargo:rerun-if-changed=Cargo.lock");
  let lock =
    fs::read_to_string("Cargo.lock").expect("Cargo.lock must exist to resolve naga version");
  let mut lines = lock.lines();
  let mut version = None;
  while let Some(line) = lines.next() {
    if line.trim() == "name = \"naga\"" {
      for candidate in lines.by_ref().take(5) {
        let candidate = candidate.trim();
        if let Some(v) = candidate
          .strip_prefix("version = \"")
          .and_then(|v| v.strip_suffix('"'))
        {
          version = Some(v.to_owned());
          break;
        }
      }
    }
    if version.is_some() {
      break;
    }
  }
  let version = version.expect("could not find resolved naga version in Cargo.lock");
  println!("cargo:rustc-env=NAGA_VERSION={version}");
}
