#include "milo.h"
#include "stdio.h"
#include "string.h"

#include "output.h"
#include "utils.h"

#define EXTRACT_PAYLOAD(name, parser, from, size)                                                                      \
  auto name = size > 0 ? reinterpret_cast<context_t*>(parser->context)->input + from : NULL;

uchar_t* copy_string(const char* source, usize_t size) {
  if (size == 0) {
    size = strlen(source);
  }

  auto destination = reinterpret_cast<uchar_t*>(malloc(sizeof(uchar_t) * size));
  strncpy(reinterpret_cast<char*>(destination), source, size);
  return destination;
}

uchar_t* copy_string(uchar_t* source, usize_t size) {
  return copy_string(reinterpret_cast<const char*>(source), size);
}

void on_state_change(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  usize_t position = parser->position;
  auto state = milo::milo_state_string(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT, "\"pos\": %lu, \"event\": \"state\", \"state\": \"%s\"",
           position, state.ptr);
  milo::milo_free_string(state);

  append_output(parser, message, data, size);
}

void on_message_start(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  usize_t position = parser->position;

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
           "\"pos\": %lu, \"event\": \"begin\", \"configuration\": { \"debug\": %s }", position,
           milo::DEBUG ? "true" : "false");

  append_output(parser, message, data, size);
}

void on_message_complete(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  event(parser, "complete", parser->position, data, size);
}

void on_error(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  usize_t position = parser->position;
  usize_t error_code = static_cast<usize_t>(parser->error_code);
  auto error_code_string = milo::milo_error_code_string(parser);
  auto error_code_description = milo::milo_error_description_string(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
           "\"pos\": %lu, \"event\": \"error\", \"error_code\": %lu, \"error_code_string\": \"%s\", \"reason\": \"%s\"",
           position, error_code, error_code_string.ptr, error_code_description.ptr);
  milo::milo_free_string(error_code_string);
  milo::milo_free_string(error_code_description);

  append_output(parser, message, data, size);
}

void on_finish(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  event(parser, "finish", parser->position, data, size);
}

void on_request(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  event(parser, "request", parser->position, data, size);
}

void on_response(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  event(parser, "response", parser->position, data, size);
}

void on_method(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "method", data, size);
}

void on_url(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "url", data, size);
}

void on_protocol(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "protocol", data, size);
}

void on_version(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "version", data, size);
}

void on_status(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "status", data, size);
}

void on_reason(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "reason", data, size);
}

void on_header_name(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "header_name", data, size);
}

void on_header_value(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "header_value", data, size);
}

void on_headers(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  usize_t position = parser->position;
  usize_t content_length = parser->content_length;
  bool chunked = parser->has_chunked_transfer_encoding;

  context_t* context = reinterpret_cast<context_t*>(parser->context);
  uchar_t* method = context->method;
  uchar_t* url = context->url;
  uchar_t* protocol = context->protocol;
  uchar_t* version = context->version;

  auto message = create_string();

  if (parser->message_type == milo::MESSAGE_TYPE_RESPONSE) {
    if (chunked) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu, \"event\": \"headers\", \"type\": \"response\", \"status\": %lu, \"protocol\": \"%s\", "
               "\"version\": \"%s\", \"body\": \"chunked\"",
               position, parser->status, protocol, version);
    } else if (content_length > 0) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu, \"event\": \"headers\", \"type\": \"response\", \"status\": %lu, \"protocol\": \"%s\", "
               "\"version\": \"%s\", \"body\": %lu",
               position, parser->status, protocol, version, content_length);
    } else {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu, \"event\": \"headers\", \"type\": \"response\", \"status\": %lu, \"protocol\": \"%s\", "
               "\"version\": \"%s\", \"body\": null",
               position, parser->status, protocol, version);
    }
  } else {
    if (chunked) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu, \"event\": \"headers\", \"type\": \"request\", \"method\": \"%s\", \"url\": \"%s\", "
               "\"protocol\": \"%s\", \"version\": \"%s\", \"body\": \"chunked\"",
               position, method, url, protocol, version);
    } else if (content_length > 0) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu, \"event\": \"headers\", \"type\": \"request\", \"method\": \"%s\", \"url\": \"%s\", "
               "\"protocol\": \"%s\", \"version\": \"%s\", \"body\": %lu",
               position, method, url, protocol, version, content_length);
    } else {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "\"pos\": %lu, \"event\": \"headers\", \"type\": \"request\", \"method\": \"%s\", \"url\": \"%s\", "
               "\"protocol\": \"%s\", \"version\": \"%s\", \"body\": null",
               position, method, url, protocol, version);
    }
  }

  append_output(parser, message, data, size);
}

void on_upgrade(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  event(parser, "upgrade", parser->position, data, size);
}

void on_chunk_length(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "chunk_length", data, size);
}

void on_chunk_extension_name(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "chunk_extensions_name", data, size);
}

void on_chunk_extension_value(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "chunk_extension_value", data, size);
}

void on_chunk(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  event(parser, "chunk", parser->position, data, size);
}

void on_body(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  event(parser, "body", parser->position, data, size);
}

void on_data(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "data", data, size);
}

void on_trailer_name(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "trailer_name", data, size);
}

void on_trailer_value(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  show_span(parser, "trailer_value", data, size);
}

void on_trailers(milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  event(parser, "trailers", parser->position, data, size);
}

int main() {
  milo::Parser* parser = milo::milo_create();
  context_t context = {.input = NULL, .method = NULL, .url = NULL, .protocol = NULL, .version = NULL};
  parser->context = &context;

  const char* request1 = "GET / HTTP/1.1\r\n\r\n";
  const char* request2 = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: "
                         "x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n";

  parser->callbacks.on_state_change = on_state_change;
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
  parser->callbacks.on_upgrade = on_upgrade;
  parser->callbacks.on_chunk_length = on_chunk_length;
  parser->callbacks.on_chunk_extension_name = on_chunk_extension_name;
  parser->callbacks.on_chunk_extension_value = on_chunk_extension_value;
  parser->callbacks.on_chunk = on_chunk;
  parser->callbacks.on_body = on_body;
  parser->callbacks.on_data = on_data;
  parser->callbacks.on_trailer_name = on_trailer_name;
  parser->callbacks.on_trailer_value = on_trailer_value;
  parser->callbacks.on_trailers = on_trailers;

  context.input = copy_string(request1, 0);
  usize_t consumed = milo::milo_parse(parser, reinterpret_cast<const uchar_t*>(request1), strlen(request1));
  auto state = milo::milo_state_string(parser);

  printf("{ \"pos\": %lu, \"consumed\": %lu, \"state\": \"%s\" }\n", parser->position, consumed, state.ptr);
  milo::milo_free_string(state);
  clear_context(&context);

  printf("\n------------------------------------------------------------------------------------------\n\n");

  context.input = copy_string(request2, 0);
  consumed = milo::milo_parse(parser, reinterpret_cast<const uchar_t*>(request2), strlen(request2));
  state = milo::milo_state_string(parser);

  printf("{ \"pos\": %lu, \"consumed\": %lu, \"state\": \"%s\" }\n", parser->position, consumed, state.ptr);
  milo::milo_free_string(state);
  clear_context(&context);

  milo::milo_destroy(parser);
}
