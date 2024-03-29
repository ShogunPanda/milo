pub struct Context {
  pub input: String,
  pub output: String,
  pub method: String,
  pub url: String,
  pub protocol: String,
  pub version: String,
}

impl Default for Context {
  fn default() -> Self { Self::new() }
}

impl Context {
  #[allow(dead_code)]
  pub fn new() -> Context {
    Context {
      input: String::new(),
      output: String::new(),
      method: String::new(),
      url: String::new(),
      protocol: String::new(),
      version: String::new(),
    }
  }
}
