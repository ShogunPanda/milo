#include "milo.h"
#include "stdio.h"
#include "string.h"

typedef intptr_t isize_t;
typedef uintptr_t usize_t;
typedef unsigned char uchar_t;
const usize_t MAX_FORMAT = 1000;

uchar_t* create_string() {
  return reinterpret_cast<uchar_t*>(malloc(sizeof(uchar_t) * MAX_FORMAT));
}

isize_t append_output(milo::Parser* parser, uchar_t* message, const uchar_t* data, usize_t size) {
  if (data == NULL) {
    printf("%-50s | cb_len=%lu cb_data=NULL\n", message, size);
  } else {
    uchar_t* read_data = create_string();
    strncpy(reinterpret_cast<char*>(read_data), reinterpret_cast<const char*>(data), size);

    printf("%-50s | cb_len=%lu cb_data=\"%s\"\n", message, size, read_data);
  }

  free(message);
  return 0;
}

isize_t show_span(milo::Parser* parser, const char* name, const uchar_t* value, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);
  auto message = create_string();
  snprintf((char*) message, MAX_FORMAT, "pos=%lu span[%s]=\"%s\"", position, name, value);
  return append_output(parser, message, data, size);
}

isize_t status_complete(const char* name, milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);
  auto message = create_string();
  snprintf((char*) message, MAX_FORMAT, "pos=%lu %s complete", position, name);
  return append_output(parser, message, data, size);
}

isize_t on_error(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);
  usize_t error_code = milo::get_error_code(parser);
  auto error_code_string = milo::get_error_code_string(parser);
  auto error_code_description = milo::get_error_description_string(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT, "pos=%lu error code=%lu (%s) description=\"%s\"", position,
           error_code, error_code_string, error_code_description);

  return append_output(parser, message, data, size);
}

isize_t on_finish(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT, "pos=%lu finish", position);

  return append_output(parser, message, data, size);
}

isize_t on_request(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT, "pos=%lu request", position);

  return append_output(parser, message, data, size);
}

isize_t on_response(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT, "pos=%lu response", position);

  return append_output(parser, message, data, size);
}

isize_t on_message_start(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT, "pos=%lu message_start", position);

  return append_output(parser, message, data, size);
}

isize_t on_message_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);

  auto message = create_string();
  snprintf(reinterpret_cast<char*>(message), MAX_FORMAT, "pos=%lu message_complete", position);

  return append_output(parser, message, data, size);
}

isize_t on_method(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "method", milo::get_method_string(parser), data, size);
}

isize_t on_method_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("method", parser, data, size);
}

isize_t on_url(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "url", milo::get_url_string(parser), data, size);
}

isize_t on_url_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("url", parser, data, size);
}

isize_t on_protocol(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "protocol", milo::get_protocol_string(parser), data, size);
}

isize_t on_protocol_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("protocol", parser, data, size);
}

isize_t on_version(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "version", milo::get_version_string(parser), data, size);
}

isize_t on_version_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("version", parser, data, size);
}

isize_t on_status(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "status", milo::get_status_string(parser), data, size);
}

isize_t on_status_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("status", parser, data, size);
}

isize_t on_reason(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "reason", milo::get_reason_string(parser), data, size);
}

isize_t on_reason_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("reason", parser, data, size);
}

isize_t on_header_name(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "header_name", milo::get_header_name_string(parser), data, size);
}

isize_t on_header_name_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("header_name", parser, data, size);
}

isize_t on_header_value(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "header_value", milo::get_header_value_string(parser), data, size);
}

isize_t on_header_value_complete(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("header_value", parser, data, size);
}

isize_t on_headers(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);
  auto version = milo::get_version_string(parser);
  usize_t content_length = milo::get_expected_content_length(parser);
  bool chunked = milo::get_has_chunked_transfer_encoding(parser) == 1;
  auto protocol = milo::get_protocol_string(parser);

  auto message = create_string();

  if (milo::get_message_type(parser) == milo::RESPONSE) {
    if (chunked) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "pos=%lu headers complete type=response status=%lu protocol=%s v=%s chunked", position,
               milo::get_status(parser), protocol, version);
    } else if (content_length > 0) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "pos=%lu headers complete type=response status=%lu protocol=%s v=%s content_length=%lu", position,
               milo::get_status(parser), protocol, version, content_length);
    } else {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "pos=%lu headers complete type=response status=%lu protocol=%s v=%s no-body", position,
               milo::get_status(parser), protocol, version);
    }
  } else {
    auto method = milo::get_method_string(parser);
    auto url = milo::get_url_string(parser);

    if (chunked) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "pos=%lu headers complete type=request method=%s url=%s protocol=%s v=%s chunked", position, method, url,
               protocol, version);
    } else if (content_length > 0) {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "pos=%lu headers complete type=request method=%s url=%s protocol=%s v=%s content_length=%lu", position,
               method, url, protocol, version, content_length);
    } else {
      snprintf(reinterpret_cast<char*>(message), MAX_FORMAT,
               "pos=%lu headers complete type=request method=%s url=%s protocol=%s v=%s no-body", position, method, url,
               protocol, version);
    }
  }

  return append_output(parser, message, data, size);
}

isize_t on_upgrade(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("upgrade", parser, data, size);
}

isize_t on_chunk_length(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "chunk_length", milo::get_chunk_length_string(parser), data, size);
}

isize_t on_chunk_extension_name(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "chunk_extensions_name", milo::get_chunk_extension_name_string(parser), data, size);
}

isize_t on_chunk_extension_value(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "chunk_extension_value", milo::get_chunk_extension_value_string(parser), data, size);
}

isize_t on_chunk_data(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "chunk", milo::get_chunk_data_string(parser), data, size);
}

isize_t on_body(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "body", milo::get_body_string(parser), data, size);
}

isize_t on_data(milo::Parser* parser, const uchar_t* data, usize_t size) {
  usize_t position = milo::get_position(parser);
  auto message = create_string();
  uchar_t* read_data = create_string();
  strncpy(reinterpret_cast<char*>(read_data), reinterpret_cast<const char*>(data), size);
  snprintf((char*) message, MAX_FORMAT, "pos=%lu data=\"%s\" (len=%lu)", position, read_data, size);
  return append_output(parser, message, data, size);
}

isize_t on_trailer_name(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "trailer_name", milo::get_trailer_name_string(parser), data, size);
}

isize_t on_trailer_value(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return show_span(parser, "trailer_value", milo::get_trailer_value_string(parser), data, size);
}

isize_t on_trailers(milo::Parser* parser, const uchar_t* data, usize_t size) {
  return status_complete("trailers", parser, data, size);
}

int main() {
  auto parser = milo::create_parser();

  const char* request1 = "GET / HTTP/1.1\r\n\r\n";
  const char* request2 = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: "
                         "x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n";

  milo::set_on_error(parser, on_error);
  milo::set_on_finish(parser, on_finish);
  milo::set_on_request(parser, on_request);
  milo::set_on_response(parser, on_response);
  milo::set_on_message_start(parser, on_message_start);
  milo::set_on_message_complete(parser, on_message_complete);
  milo::set_on_method(parser, on_method);
  milo::set_on_url(parser, on_url);
  milo::set_on_protocol(parser, on_protocol);
  milo::set_on_version(parser, on_version);
  milo::set_on_status(parser, on_status);
  milo::set_on_reason(parser, on_reason);
  milo::set_on_header_name(parser, on_header_name);
  milo::set_on_header_value(parser, on_header_value);
  milo::set_on_headers(parser, on_headers);
  milo::set_on_body(parser, on_upgrade);
  milo::set_on_chunk_length(parser, on_chunk_length);
  milo::set_on_chunk_extension_name(parser, on_chunk_extension_name);
  milo::set_on_chunk_extension_value(parser, on_chunk_extension_value);
  milo::set_on_chunk_data(parser, on_chunk_data);
  milo::set_on_body(parser, on_body);
  milo::set_on_data(parser, on_data);
  milo::set_on_trailer_name(parser, on_trailer_name);
  milo::set_on_trailer_value(parser, on_trailer_value);
  milo::set_on_trailers(parser, on_trailers);

  usize_t consumed = milo::execute_parser(parser, reinterpret_cast<const uchar_t*>(request1), strlen(request1));
  usize_t pos = milo::get_position(parser);
  auto* state = milo::get_state_string(parser);

  printf("pos=%lu consumed=%lu state=%s\n", pos, consumed, state);

  printf("------------------------------------------------------------------------------------------\n");

  consumed = milo::execute_parser(parser, reinterpret_cast<const uchar_t*>(request2), strlen(request2));
  pos = milo::get_position(parser);
  state = milo::get_state_string(parser);

  printf("pos=%lu consumed=%lu state=%s\n", pos, consumed, state);

  milo::free_parser(parser);
}
