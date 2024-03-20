#include "milo.h"
#include "stdio.h"
#include "string.h"

#include "output.h"
#include "utils.h"

#define EXTRACT_PAYLOAD(name, parser, from, size)                                                                      \
  auto name = size > 0 ? reinterpret_cast<context_t*>(parser->owner) -> input + from : NULL;

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

void process_offsets(const milo::Parser* parser) {
  usize_t position = parser->position;
  usize_t content_length = parser->content_length;
  bool chunked = parser->has_chunked_transfer_encoding;

  context_t* context = reinterpret_cast<context_t*>(parser->owner);

  usize_t* offsets = parser->offsets;
  usize_t total = offsets[2];
  offsets[2] = 0;

  for (uintptr_t i = 1; i <= total; i++) {
    usize_t offset_from = offsets[i * 3 + 1];
    usize_t offset_size = offsets[i * 3 + 2];
    EXTRACT_PAYLOAD(value, parser, offset_from, offset_size);

    switch (offsets[i * 3]) {
    case milo::OFFSET_MESSAGE_START:
      event(parser, "offset.message_start", offset_from, value, offset_size);
      break;
    case milo::OFFSET_MESSAGE_COMPLETE:
      event(parser, "offset.message_complete", offset_from, value, offset_size);
      break;
    case milo::OFFSET_METHOD:
      event(parser, "offset.method", offset_from, value, offset_size);
      context->method = copy_string(value, offset_size);
      break;
    case milo::OFFSET_URL:
      event(parser, "offset.url", offset_from, value, offset_size);
      context->url = copy_string(value, offset_size);
      break;
    case milo::OFFSET_PROTOCOL:
      event(parser, "offset.protocol", offset_from, value, offset_size);
      context->protocol = copy_string(value, offset_size);
      break;
    case milo::OFFSET_VERSION:
      event(parser, "offset.version", offset_from, value, offset_size);
      context->version = copy_string(value, offset_size);
      break;
    case milo::OFFSET_STATUS:
      event(parser, "offset.status", offset_from, value, offset_size);
      break;
    case milo::OFFSET_REASON:
      event(parser, "offset.reason", offset_from, value, offset_size);
      break;
    case milo::OFFSET_HEADER_NAME:
      event(parser, "offset.header_name", offset_from, value, offset_size);
      break;
    case milo::OFFSET_HEADER_VALUE:
      event(parser, "offset.header_value", offset_from, value, offset_size);
      break;
    case milo::OFFSET_HEADERS:
      event(parser, "offset.headers", offset_from, value, offset_size);
      break;
    case milo::OFFSET_CHUNK_LENGTH:
      event(parser, "offset.chunk_length", offset_from, value, offset_size);
      break;
    case milo::OFFSET_CHUNK_EXTENSION_NAME:
      event(parser, "offset.chunk_extensions_name", offset_from, value, offset_size);
      break;
    case milo::OFFSET_CHUNK_EXTENSION_VALUE:
      event(parser, "offset.chunk_extension_value", offset_from, value, offset_size);
      break;
    case milo::OFFSET_CHUNK:
      event(parser, "offset.chunk", offset_from, value, offset_size);
      break;
    case milo::OFFSET_DATA:
      event(parser, "offset.data", offset_from, value, offset_size);
      break;
    case milo::OFFSET_BODY:
      event(parser, "offset.body", offset_from, value, offset_size);
      break;
    case milo::OFFSET_TRAILER_NAME:
      event(parser, "offset.trailer_name", offset_from, value, offset_size);
      break;
    case milo::OFFSET_TRAILER_VALUE:
      event(parser, "offset.trailer_value", offset_from, value, offset_size);
      break;
    case milo::OFFSET_TRAILERS:
      event(parser, "offset.trailers", offset_from, value, offset_size);
      break;
    default:
      printf("Unexpected offset with type %lu", offsets[i * 3]);
      exit(1);
    }
  }
}

isize_t before_state_change(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);

  usize_t position = parser->position;
  auto state = milo::milo_state_string(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
           "\"pos\": %lu, \"event\": \"before_state_change\", \"current_state\": \"%s\"", position, state.ptr);
  milo::milo_free_string(state);

  return append_output(parser, message, data, size);
}

isize_t after_state_change(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  usize_t position = parser->position;
  auto state = milo::milo_state_string(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
           "\"pos\": %lu, \"event\": \"after_state_change\", \"current_state\": \"%s\"", position, state.ptr);
  milo::milo_free_string(state);

  return append_output(parser, message, data, size);
}

isize_t on_message_start(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  usize_t position = parser->position;

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
           "\"pos\": %lu, \"event\": \"begin\", \"configuration\": { \"debug\": %s }", position,
           milo::DEBUG ? "true" : "false");

  return append_output(parser, message, data, size);
}

isize_t on_message_complete(const milo::Parser* parser, usize_t from, usize_t size) {
  process_offsets(parser);

  EXTRACT_PAYLOAD(data, parser, from, size);
  return event(parser, "complete", parser->position, data, size);
}

isize_t on_error(const milo::Parser* parser, usize_t from, usize_t size) {
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

  return append_output(parser, message, data, size);
}

isize_t on_finish(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return event(parser, "finish", parser->position, data, size);
}

isize_t on_request(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return event(parser, "request", parser->position, data, size);
}

isize_t on_response(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return event(parser, "response", parser->position, data, size);
}

isize_t on_method(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "method", data, size);
}

isize_t on_url(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "url", data, size);
}

isize_t on_protocol(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "protocol", data, size);
}

isize_t on_version(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "version", data, size);
}

isize_t on_status(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "status", data, size);
}

isize_t on_reason(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "reason", data, size);
}

isize_t on_header_name(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "header_name", data, size);
}

isize_t on_header_value(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "header_value", data, size);
}

isize_t on_headers(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  usize_t position = parser->position;
  usize_t content_length = parser->content_length;
  bool chunked = parser->has_chunked_transfer_encoding;

  context_t* context = reinterpret_cast<context_t*>(parser->owner);
  uchar_t* method = context->method;
  uchar_t* url = context->url;
  uchar_t* protocol = context->protocol;
  uchar_t* version = context->version;

  auto message = create_string();
  process_offsets(parser);

  if (parser->message_type == milo::RESPONSE) {
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

  return append_output(parser, message, data, size);
}

isize_t on_upgrade(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return event(parser, "upgrade", parser->position, data, size);
}

isize_t on_chunk_length(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "chunk_length", data, size);
}

isize_t on_chunk_extension_name(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "chunk_extensions_name", data, size);
}

isize_t on_chunk_extension_value(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "chunk_extension_value", data, size);
}

isize_t on_chunk(const milo::Parser* parser, usize_t from, usize_t size) {
  process_offsets(parser);

  usize_t* offsets = parser->offsets;
  usize_t total = offsets[2];

  for (uintptr_t i = 1; i <= total; i++) {
    usize_t offset_from = offsets[i * 3 + 1];
    usize_t offset_size = offsets[i * 3 + 2];
    EXTRACT_PAYLOAD(value, parser, offset_from, offset_size);

    switch (offsets[i * 3]) {
    case milo::OFFSET_CHUNK_LENGTH:
      event(parser, "offset.chunk_length", offset_from, value, offset_size);
      break;
    case milo::OFFSET_CHUNK_EXTENSION_NAME:
      event(parser, "offset.chunk_extensions_name", offset_from, value, offset_size);
      break;
    case milo::OFFSET_CHUNK_EXTENSION_VALUE:
      event(parser, "offset.chunk_extension_value", offset_from, value, offset_size);
      break;
    }
  }

  milo::milo_clear_offsets(parser);

  EXTRACT_PAYLOAD(data, parser, from, size);
  return event(parser, "chunk", parser->position, data, size);
}

isize_t on_body(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return event(parser, "body", parser->position, data, size);
}

isize_t on_data(const milo::Parser* parser, usize_t from, usize_t size) {
  process_offsets(parser);

  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "data", data, size);
}

isize_t on_trailer_name(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "trailer_name", data, size);
}

isize_t on_trailer_value(const milo::Parser* parser, usize_t from, usize_t size) {
  EXTRACT_PAYLOAD(data, parser, from, size);
  return show_span(parser, "trailer_value", data, size);
}

isize_t on_trailers(const milo::Parser* parser, usize_t from, usize_t size) {
  process_offsets(parser);

  EXTRACT_PAYLOAD(data, parser, from, size);
  return event(parser, "trailers", parser->position, data, size);
}

int main() {
  milo::Parser* parser = milo::milo_create();

  context_t context = {.input = NULL, .method = NULL, .url = NULL, .protocol = NULL, .version = NULL};
  parser->owner = &context;

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
