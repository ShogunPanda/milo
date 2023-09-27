#include "milo.h"
#include "stdio.h"
#include "string.h"

typedef intptr_t isize_t;
typedef uintptr_t usize_t;
typedef unsigned char uchar_t;

const usize_t MAX_FORMAT = 1000;
std::unordered_map<const char*, const uchar_t*> spans;

uchar_t* create_string() {
  return reinterpret_cast<uchar_t*>(malloc(sizeof(uchar_t) * MAX_FORMAT));
}

isize_t append_output(milo::Parser* parser, uchar_t* message, const uchar_t* data, usize_t size) {
  if (data == NULL) {
    printf("{ %s, \"data\": null}\n", message);
  } else {
    uchar_t* string_data = create_string();
    strncpy(reinterpret_cast<char*>(string_data), reinterpret_cast<const char*>(data), size);
  }

  free(message);
  return 0;
}

isize_t event(milo::Parser* parser, const char* name, const uchar_t* data, usize_t size) {
  usize_t position = parser->position;
  auto message = create_string();
  snprintf((char*) message, MAX_FORMAT, "\"pos\": %lu, \"event\": \"%s\"", position, name);
  return append_output(parser, message, data, size);
}

isize_t show_span(milo::Parser* parser, const char* name, const uchar_t* data, usize_t size) {
  if (strcmp(name, "version") == 0 || strcmp(name, "protocol") == 0 || strcmp(name, "method") == 0 ||
      strcmp(name, "url") == 0) {
    uchar_t* string_data = create_string();
    strncpy(reinterpret_cast<char*>(string_data), reinterpret_cast<const char*>(data), size);
    spans[name] = string_data;
  }

  return event(parser, name, data, size);
}

isize_t before_state_change(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = parser->position;
  auto state = milo::milo_state_string(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
           "\"pos\": %lu, \"event\": \"before_state_change\", \"current_state\": \"%s\"", position, state);
  milo::milo_free_string(state);

  return append_output(parser, message, data, size);
}

isize_t after_state_change(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = parser->position;
  auto state = milo::milo_state_string(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
           "\"pos\": %lu, \"event\": \"after_state_change\", \"current_state\": \"%s\"", position, state);
  milo::milo_free_string(state);

  return append_output(parser, message, data, size);
}

isize_t on_message_start(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return event(parser, "begin", data, size);
}

isize_t on_message_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return event(parser, "complete", data, size);
}

isize_t on_error(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = parser->position;
  usize_t error_code = static_cast<usize_t>(parser->error_code);
  auto error_code_string = milo::milo_error_code_string(parser);
  auto error_code_description = milo::milo_error_description_string(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
           "\"pos\": %lu, \"event\": \"error\", \"error_code\": %lu, \"error_code_string\": \"%s\", \"reason\": \"%s\"",
           position, error_code, error_code_string, error_code_description);
  milo::milo_free_string(error_code_string);
  milo::milo_free_string(error_code_description);

  return append_output(parser, message, data, size);
}

isize_t on_finish(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return event(parser, "finish", data, size);
}

isize_t on_request(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return event(parser, "request", data, size);
}

isize_t on_response(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return event(parser, "response", data, size);
}

isize_t on_method(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "method", data, size);
}

isize_t on_url(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "url", data, size);
}

isize_t on_protocol(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "protocol", data, size);
}

isize_t on_version(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "version", data, size);
}

isize_t on_status(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "status", data, size);
}

isize_t on_reason(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "reason", data, size);
}

isize_t on_header_name(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "header_name", data, size);
}

isize_t on_header_value(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "header_value", data, size);
}

isize_t on_headers(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = parser->position;
  auto version = spans["version"];
  usize_t content_length = parser->content_length;
  bool chunked = parser->has_chunked_transfer_encoding;
  auto protocol = spans["protocol"];

  auto message = create_string();

  if (parser->message_type == milo::RESPONSE) {
    if (chunked) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu \"event\": \"headers\", \"type\": \"response\", \"status\": %lu, \"protocol\": \"%s\", "
               "\"version\": \"%s\", \"body\": \"chunked\"",
               position, parser->status, protocol, version);
    } else if (content_length > 0) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu \"event\": \"headers\", \"type\": \"response\", \"status\": %lu, \"protocol\": \"%s\", "
               "\"version\": \"%s\", \"body\": %lu",
               position, parser->status, protocol, version, content_length);
    } else {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu \"event\": \"headers\", \"type\": \"response\", \"status\": %lu, \"protocol\": \"%s\", "
               "\"version\": \"%s\", \"body\": null",
               position, parser->status, protocol, version);
    }
  } else {
    auto method = spans["method"];
    auto url = spans["url"];

    if (chunked) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu \"event\": \"headers\", \"type\": \"request\", \"method\": \"%s\", \"url\": \"%s\", "
               "\"protocol\": \"%s\", \"version\": \"%s\", \"body\": \"chunked\"",
               position, method, url, protocol, version);
    } else if (content_length > 0) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu \"event\": \"headers\", \"type\": \"request\", \"method\": \"%s\", \"url\": \"%s\", "
               "\"protocol\": \"%s\", \"version\": \"%s\", \"body\": %lu",
               position, method, url, protocol, version, content_length);
    } else {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu \"event\": \"headers\", \"type\": \"request\", \"method\": \"%s\", \"url\": \"%s\", "
               "\"protocol\": \"%s\", \"version\": \"%s\", \"body\": null",
               position, method, url, protocol, version);
    }
  }

  return append_output(parser, message, data, size);
}

isize_t on_upgrade(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return event(parser, "upgrade", data, size);
}

isize_t on_chunk_length(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "chunk_length", data, size);
}

isize_t on_chunk_extension_name(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "chunk_extensions_name", data, size);
}

isize_t on_chunk_extension_value(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "chunk_extension_value", data, size);
}

isize_t on_body(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return event(parser, "body", data, size);
}

isize_t on_data(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "data", data, size);
}

isize_t on_trailer_name(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "trailer_name", data, size);
}

isize_t on_trailer_value(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "trailer_value", data, size);
}

isize_t on_trailers(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return event(parser, "trailers", data, size);
}

int main() {
  milo::Parser* parser = milo::milo_create();

  const char* request1 = "GET / HTTP/1.1\r\n\r\n";
  const char* request2 = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: "
                         "x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n";

  parser->callbacks.before_state_change = before_state_change;
  parser->callbacks.after_state_change = after_state_change;
  parser->callbacks.on_error = on_error;
  parser->callbacks.on_finish = on_finish;
  parser->callbacks.on_request = on_request;
  parser->callbacks.on_response = on_response;
  parser->callbacks.on_message_start = on_message_start;
  parser->callbacks.on_message_complete = on_message_complete;
  parser->callbacks.on_method = on_method;
  parser->callbacks.on_url = on_url;
  parser->callbacks.on_protocol = on_protocol;
  parser->callbacks.on_version = on_version;
  parser->callbacks.on_status = on_status;
  parser->callbacks.on_reason = on_reason;
  parser->callbacks.on_header_name = on_header_name;
  parser->callbacks.on_header_value = on_header_value;
  parser->callbacks.on_headers = on_headers;
  parser->callbacks.on_body = on_upgrade;
  parser->callbacks.on_chunk_length = on_chunk_length;
  parser->callbacks.on_chunk_extension_name = on_chunk_extension_name;
  parser->callbacks.on_chunk_extension_value = on_chunk_extension_value;
  parser->callbacks.on_body = on_body;
  parser->callbacks.on_data = on_data;
  parser->callbacks.on_trailer_name = on_trailer_name;
  parser->callbacks.on_trailer_value = on_trailer_value;
  parser->callbacks.on_trailers = on_trailers;

  usize_t consumed = milo::milo_parse(parser, reinterpret_cast<const uchar_t*>(request1), strlen(request1));
  usize_t pos = parser->position;
  auto* state = milo::milo_state_string(parser);

  printf("{ \"pos\": %lu, \"consumed\": %lu, \"state\": \"%s\" }\n", pos, consumed, state);

  printf("------------------------------------------------------------------------------------------\n");

  consumed = milo::milo_parse(parser, reinterpret_cast<const uchar_t*>(request2), strlen(request2));
  pos = parser->position;
  state = milo::milo_state_string(parser);

  printf("{ \"pos\": %lu, \"consumed\": %lu, \"state\": \"%s\" }\n", pos, consumed, state);
  milo::milo_free_string(state);

  milo::milo_destroy(parser);
}
