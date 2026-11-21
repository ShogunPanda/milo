use std::{
  env,
  error::Error,
  fs,
  path::Path,
  process::{Command, Stdio},
};

use regex::{Captures, Regex};

fn main() -> Result<(), Box<dyn Error>> {
  let folders = ["macros", "parser", "references/rust", "parser/wasm/src"];
  let version = env::args().nth(1).ok_or("Usage: sync-versions <version>")?;
  let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");

  for folder in folders {
    let path = root.join(folder);
    env::set_current_dir(&path)?;

    if folder == "parser" {
      // Align the local dependency before cambi updates the parser lockfile.
      let manifest = path.join("Cargo.toml");
      let content = fs::read_to_string(&manifest)?;
      let matcher = Regex::new(r#"(?m)^(\s+milo-macros = \{ version = ")([^"]+)(")"#)?;
      let content = matcher.replace(&content, |captures: &Captures<'_>| {
        format!("{}{version}{}", &captures[1], &captures[3])
      });
      fs::write(manifest, content.as_bytes())?;
    }

    // Like execFileSync, wait for each update and discard its stdout.
    let output = Command::new("cambi")
      .args(["update", &version])
      .stderr(Stdio::inherit())
      .output()?;
    if !output.status.success() {
      return Err(format!("cambi update failed in {folder}: {}", output.status).into());
    }
  }

  Ok(())
}
