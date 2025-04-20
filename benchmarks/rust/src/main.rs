use std::fs::read_to_string;
use std::path::Path;
use std::time::Instant;

use milo::Parser;
use regex::Regex;

fn format_number(num: f64, drop_decimals: bool) -> String {
  let formatted = if drop_decimals {
    format!("{:.0}", num)
  } else {
    format!("{:.2}", num)
  };

  let thousands = Regex::new("([0-9]{3})").unwrap();
  let last = Regex::new("^_").unwrap();

  let reversed = formatted.chars().rev().collect::<String>().to_string();
  let grouped = thousands.replace_all(&reversed, "${1}_").to_string();
  last
    .replace(grouped.chars().rev().collect::<String>().as_str(), "")
    .to_string()
}

fn load_message(name: &str) -> String {
  let mut absolute_path = Path::new(file!())
    .canonicalize()
    .unwrap()
    .parent()
    .unwrap()
    .to_path_buf();

  absolute_path.push(format!("../../fixtures/{}.txt", name));

  read_to_string(absolute_path)
    .unwrap()
    .replace('\n', "")
    .replace("\\r\\n", "\r\n")
}

fn main() {
  let samples = vec!["seanmonstar_httparse", "nodejs_http_parser", "undici"];

  for name in samples {
    let payload = load_message(name);
    let len = payload.len();
    let iterations = (8 << 30) / len;
    let total = iterations * len;

    let mut parser = Parser::new();
    let start = Instant::now();

    for _i in 0..iterations {
      parser.parse(payload.as_ptr(), payload.len());
    }

    let time = Instant::now().duration_since(start).as_secs_f64();
    let bw = (total as f64) / time;

    println!(
      "{:>21} | {:>12} samples | {:>8} MB | {:>10} MB/s | {:>10} ops/sec | {:>6} s",
      name,
      format_number(iterations as f64, true),
      format_number(total as f64 / (1024.0 * 1024.0), false),
      format_number(bw / (1024 * 1024) as f64, false),
      format_number((iterations as f64) / time, false),
      format_number(time, false)
    );
  }
}
