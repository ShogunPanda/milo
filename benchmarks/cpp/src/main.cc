#include "llhttp.h"
#include "milo.h"

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

static const uint64_t TARGET_BYTES = 8ULL << 30;

struct Fixture {
  std::string name;
  bool is_request;
  std::string payload;
};

struct Result {
  std::string parser;
  std::string iterations;
  std::string mb_per_second;
  std::string ops_per_second;
  double bytes_per_second;
};

static std::string read_file(const std::string &path) {
  std::ifstream stream(path.c_str(), std::ios::in | std::ios::binary);
  if (!stream) {
    std::cerr << "Cannot open fixture: " << path << std::endl;
    std::exit(1);
  }

  std::ostringstream contents;
  contents << stream.rdbuf();
  return contents.str();
}

static std::string decode_fixture(const std::string &raw) {
  std::string decoded;
  decoded.reserve(raw.size());

  for (size_t i = 0; i < raw.size();) {
    if (raw[i] == '\n') {
      i++;
    } else if (i + 3 < raw.size() && raw[i] == '\\' && raw[i + 1] == 'r' &&
               raw[i + 2] == '\\' && raw[i + 3] == 'n') {
      decoded.push_back('\r');
      decoded.push_back('\n');
      i += 4;
    } else {
      decoded.push_back(raw[i]);
      i++;
    }
  }

  return decoded;
}

static Fixture load_fixture(const std::string &name, bool is_request) {
  const std::string raw = read_file("../fixtures/" + name + ".txt");
  return {name, is_request, decode_fixture(raw)};
}

static std::string format_number(uint64_t value) {
  std::string formatted = std::to_string(value);
  for (int64_t i = static_cast<int64_t>(formatted.size()) - 3; i > 0; i -= 3) {
    formatted.insert(static_cast<size_t>(i), 1, '_');
  }

  return formatted;
}

static std::string format_number(double value) {
  std::ostringstream formatted;
  formatted << std::fixed << std::setprecision(2) << value;

  std::string result = formatted.str();
  size_t dot = result.find('.');
  if (dot == std::string::npos) {
    dot = result.size();
  }

  for (int64_t i = static_cast<int64_t>(dot) - 3; i > 0; i -= 3) {
    result.insert(static_cast<size_t>(i), 1, '_');
  }

  return result;
}

static std::string pad_right(const std::string &value, size_t width) {
  return value + std::string(width - value.size(), ' ');
}

static std::string pad_left(const std::string &value, size_t width) {
  return std::string(width - value.size(), ' ') + value;
}

static void validate_milo(const Fixture &fixture) {
  milo::Parser *parser = milo::milo_create();
  parser->autodetect = false;
  parser->is_request = fixture.is_request;

  const size_t consumed = milo::milo_parse(
      parser, reinterpret_cast<const unsigned char *>(fixture.payload.data()),
      fixture.payload.size());
  if (consumed != fixture.payload.size() ||
      parser->error_code != milo::ERROR_NONE) {
    std::cerr << "Milo failed to parse fixture " << fixture.name << std::endl;
    std::cerr << "Consumed " << consumed << " of " << fixture.payload.size()
              << " bytes" << std::endl;
    milo::milo_destroy(parser);
    std::exit(1);
  }

  milo::milo_destroy(parser);
}

static void validate_llhttp(const Fixture &fixture) {
  llhttp_settings_t settings;
  llhttp_settings_init(&settings);

  llhttp_t parser;
  llhttp_init(&parser, fixture.is_request ? HTTP_REQUEST : HTTP_RESPONSE,
              &settings);

  const llhttp_errno_t error =
      llhttp_execute(&parser, fixture.payload.data(), fixture.payload.size());
  if (error != HPE_OK) {
    std::cerr << "llhttp failed to parse fixture " << fixture.name << ": "
              << llhttp_errno_name(error) << std::endl;
    std::exit(1);
  }
}

static Result benchmark_milo(const Fixture &fixture) {
  const uint64_t iterations = TARGET_BYTES / fixture.payload.size();
  const uint64_t total = iterations * fixture.payload.size();
  milo::Parser *parser = milo::milo_create();
  parser->autodetect = false;
  parser->is_request = fixture.is_request;

  const auto start = std::chrono::steady_clock::now();
  size_t consumed = 0;
  for (uint64_t i = 0; i < iterations; i++) {
    consumed += milo::milo_parse(
        parser, reinterpret_cast<const unsigned char *>(fixture.payload.data()),
        fixture.payload.size());
  }
  const auto end = std::chrono::steady_clock::now();

  if (consumed != total || parser->error_code != milo::ERROR_NONE) {
    std::cerr << "Milo failed while benchmarking fixture " << fixture.name
              << std::endl;
    milo::milo_destroy(parser);
    std::exit(1);
  }

  milo::milo_destroy(parser);

  const std::chrono::duration<double> elapsed = end - start;
  const double seconds = elapsed.count();
  const double bytes_per_second = static_cast<double>(total) / seconds;
  return {"milo-cpp", format_number(iterations),
          format_number(bytes_per_second / (1024.0 * 1024.0)),
          format_number(static_cast<double>(iterations) / seconds),
          bytes_per_second};
}

static Result benchmark_llhttp(const Fixture &fixture) {
  const uint64_t iterations = TARGET_BYTES / fixture.payload.size();
  const uint64_t total = iterations * fixture.payload.size();
  llhttp_settings_t settings;
  llhttp_settings_init(&settings);

  llhttp_t parser;
  llhttp_init(&parser, fixture.is_request ? HTTP_REQUEST : HTTP_RESPONSE,
              &settings);

  const auto start = std::chrono::steady_clock::now();
  uint64_t errors = 0;
  for (uint64_t i = 0; i < iterations; i++) {
    errors += llhttp_execute(&parser, fixture.payload.data(), fixture.payload.size());
  }
  const auto end = std::chrono::steady_clock::now();

  if (errors != HPE_OK) {
    std::cerr << "llhttp failed while benchmarking fixture " << fixture.name
              << std::endl;
    std::exit(1);
  }

  const std::chrono::duration<double> elapsed = end - start;
  const double seconds = elapsed.count();
  const double bytes_per_second = static_cast<double>(total) / seconds;
  return {"llhttp-cpp", format_number(iterations),
          format_number(bytes_per_second / (1024.0 * 1024.0)),
          format_number(static_cast<double>(iterations) / seconds),
          bytes_per_second};
}

static void print_separator(size_t parser_width, size_t iterations_width,
                            size_t mb_width, size_t ops_width) {
  std::cout << "| " << std::string(parser_width, '-') << " | "
            << std::string(iterations_width, '-') << " | "
            << std::string(mb_width, '-') << " | "
            << std::string(ops_width, '-') << " |" << std::endl;
}

static void print_results(const Fixture &fixture,
                          const std::vector<Result> &results) {
  std::vector<Result> sorted = results;
  std::sort(sorted.begin(), sorted.end(), [](const Result &a, const Result &b) {
    return a.bytes_per_second < b.bytes_per_second;
  });

  size_t parser_width = std::string("Parser").size();
  size_t iterations_width = std::string("Iterations").size();
  size_t mb_width = std::string("MB/s").size();
  size_t ops_width = std::string("Ops/s").size();

  for (const Result &result : sorted) {
    parser_width = std::max(parser_width, result.parser.size());
    iterations_width = std::max(iterations_width, result.iterations.size());
    mb_width = std::max(mb_width, result.mb_per_second.size());
    ops_width = std::max(ops_width, result.ops_per_second.size());
  }

  std::cout << "### " << fixture.name << std::endl << std::endl;
  std::cout << "| " << pad_right("Parser", parser_width) << " | "
            << pad_left("Iterations", iterations_width) << " | "
            << pad_left("MB/s", mb_width) << " | "
            << pad_left("Ops/s", ops_width) << " |" << std::endl;
  print_separator(parser_width, iterations_width, mb_width, ops_width);

  for (const Result &result : sorted) {
    std::cout << "| " << pad_right(result.parser, parser_width) << " | "
              << pad_left(result.iterations, iterations_width) << " | "
              << pad_left(result.mb_per_second, mb_width) << " | "
              << pad_left(result.ops_per_second, ops_width) << " |"
              << std::endl;
  }

  std::cout << std::endl;
}

int main() {
  std::vector<Fixture> fixtures;
  fixtures.push_back(load_fixture("seanmonstar_httparse", true));
  fixtures.push_back(load_fixture("nodejs_http_parser", true));
  fixtures.push_back(load_fixture("undici", false));

  for (const Fixture &fixture : fixtures) {
    validate_milo(fixture);
    validate_llhttp(fixture);

    std::vector<Result> results;
    results.push_back(benchmark_milo(fixture));
    results.push_back(benchmark_llhttp(fixture));
    print_results(fixture, results);
  }

  return 0;
}
