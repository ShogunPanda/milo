mod helpers;

use crate::helpers::llhttp::run_tests;

#[test]
fn llhttp_requests() { run_tests("requests", &None); }

#[test]
fn llhttp_responses() { run_tests("responses", &None); }
