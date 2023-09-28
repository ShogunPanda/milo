#include "output.h"
#include "milo.h"

isize_t append_output(const milo::Parser* parser, uchar_t* message, const uchar_t* data, usize_t size) {
  if (data == NULL) {
    printf("{ %s, \"data\": null }\n", message);
  } else {
    uchar_t* string_data = create_string();
    strncpy(reinterpret_cast<char*>(string_data), reinterpret_cast<const char*>(data), size);
    printf("{ %s, \"data\": \"%s\" }\n", message, string_data);
  }

  free(message);
  return 0;
}

isize_t event(const milo::Parser* parser, const char* name, const uchar_t* data, usize_t size) {
  usize_t position = parser->position;
  auto message = create_string();
  snprintf((char*) message, MAX_FORMAT, "\"pos\": %lu, \"event\": \"%s\"", position, name);
  return append_output(parser, message, data, size);
}

isize_t show_span(const milo::Parser* parser, const char* name, const uchar_t* data, usize_t size) {
  auto context = reinterpret_cast<context_t*>(parser->owner);
  uchar_t* string_data = create_string();
  strncpy(reinterpret_cast<char*>(string_data), reinterpret_cast<const char*>(data), size);

  if (strcmp(name, "method") == 0) {
    context->method = string_data;
  } else if (strcmp(name, "url") == 0) {
    context->url = string_data;
  } else if (strcmp(name, "protocol") == 0) {
    context->protocol = string_data;
  } else if (strcmp(name, "version") == 0) {
    context->version = string_data;
  } else {
    free(string_data);
  }

  return event(parser, name, data, size);
}
