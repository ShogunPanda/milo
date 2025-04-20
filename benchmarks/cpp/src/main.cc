#include "milo.h"
#include "stdio.h"
#include "string.h"
#include <chrono>
#include <cmath>
#include <fstream>
#include <regex>
#include <sstream>

#define SAMPLES_NUM 3

std::string format_number(double num, bool drop_decimals) {
  char* raw = reinterpret_cast<char*>(malloc(sizeof(char*) * 100));

  if (drop_decimals) {
    snprintf(raw, 1000, "%d", (int) num);
  } else {
    snprintf(raw, 1000, "%'.2f", num);
  }

  std::string grouped = std::string(raw);
  free(raw);

  int length = grouped.length();
  for (int i = length - 1; i >= 0; i--) {
    int current = length - i;

    if ((drop_decimals || current > 3) && current % 3 == 0) {
      grouped.insert(i, 1, '_');
    }
  }

  if (grouped[0] == '_') {
    grouped.erase(0, 1);
  }

  return grouped;
}

std::string load_message(std::string path) {
  // Build the file path
  std::stringstream file_path;
  file_path << "../fixtures/" << path << ".txt";
  std::ifstream file_stream(file_path.str(), std::ifstream::in);

  // Read the file to a string
  std::stringstream file_contents;
  file_contents << file_stream.rdbuf();
  size_t file_length = file_contents.str().length();
  std::string payload = file_contents.str();

  // Perform replacements
  //  Trim
  size_t first_non_space = payload.find_first_not_of(" \t\n");
  payload.erase(0, first_non_space);
  size_t last_non_space = payload.find_last_not_of(" \t\n");
  payload.erase(last_non_space + 1);

  //  \r\n manipulation
  payload = std::regex_replace(payload, std::regex("\\n"), "");
  return std::regex_replace(payload, std::regex("\\\\r\\\\n"), "\r\n");
}

int main() {
  std::string samples[SAMPLES_NUM] = {"seanmonstar_httparse", "nodejs_http_parser", "undici"};

  for (size_t i = 0; i < SAMPLES_NUM; i++) {
    milo::Parser* parser = milo::milo_create();
    std::string payload = load_message(samples[i]);
    double len = payload.length();
    double iterations = pow(2, 33) / len;
    double total = iterations * len;

    const std::chrono::steady_clock::time_point start = std::chrono::steady_clock::now();
    for (double j = 0; j < iterations; j++) {
      milo::milo_parse(parser, reinterpret_cast<const unsigned char*>(payload.c_str()), (int) len);
    }
    const std::chrono::steady_clock::time_point end = std::chrono::steady_clock::now();

    milo::milo_destroy(parser);

    std::chrono::duration<double> diff = end - start;
    double time = diff.count();
    double bw = total / time;

    std::string total_samples = format_number(iterations, true);
    std::string size = format_number(total / (1024.0 * 1024.0), false);
    std::string speed = format_number(bw / (1024 * 1024), false);
    std::string throughtput = format_number((iterations) / time, false);
    std::string duration = format_number(time, false);

    printf("%21s | %12s samples | %8s MB | %10s MB/s | %10s ops/sec | %6s s\n", samples[i].c_str(),
           total_samples.c_str(), size.c_str(), speed.c_str(), throughtput.c_str(), duration.c_str());
  }

  return 0;
}