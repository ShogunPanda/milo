#include "milo.h"
#include "stdio.h"

typedef intptr_t isize;
typedef uintptr_t usize;
const usize MAX_FORMAT = 1000;

isize append_output(Parser *parser, const char *message)
{
  printf("%s", message);
  return 0;
}

isize show_data(const char *name, Parser *parser, const char *data, usize size)
{
  usize position = milo_get_position(parser);
  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu data[%s]=\"%s\" (len=%lu)\n", position, name, data, size);
  return append_output(parser, message);
}

isize show_span(Parser *parser, const char *name, const char *value)
{
  usize position = milo_get_position(parser);
  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu span[%s]=\"%s\"\n", position, name, value);
  return append_output(parser, message);
}

isize status_complete(const char *name, Parser *parser)
{
  usize position = milo_get_position(parser);
  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu %s complete\n", position, name);
  return append_output(parser, message);
}

isize on_data_method(Parser *parser, const char *data, usize size)
{
  return show_data("method", parser, data, size);
}

isize on_data_url(Parser *parser, const char *data, usize size)
{
  return show_data("url", parser, data, size);
}

isize on_data_protocol(Parser *parser, const char *data, usize size)
{
  return show_data("protocol", parser, data, size);
}

isize on_data_version(Parser *parser, const char *data, usize size)
{
  return show_data("version", parser, data, size);
}

isize on_data_header_field(Parser *parser, const char *data, usize size)
{
  return show_data("header_field", parser, data, size);
}

isize on_data_header_value(Parser *parser, const char *data, usize size)
{
  return show_data("header_value", parser, data, size);
}

isize on_data_chunk_length(Parser *parser, const char *data, usize size)
{
  return show_data("chunk_length", parser, data, size);
}

isize on_data_chunk_extension_name(Parser *parser, const char *data, usize size)
{
  return show_data("chunk_extension_name", parser, data, size);
}

isize on_data_chunk_extension_value(Parser *parser, const char *data, usize size)
{
  return show_data("chunk_extension_value", parser, data, size);
}

isize on_data_chunk_data(Parser *parser, const char *data, usize size)
{
  return show_data("chunk_data", parser, data, size);
}

isize on_data_body(Parser *parser, const char *data, usize size)
{
  return show_data("body", parser, data, size);
}

isize on_data_trailer_field(Parser *parser, const char *data, usize size)
{
  return show_data("trailer_field", parser, data, size);
}

isize on_data_trailer_value(Parser *parser, const char *data, usize size)
{
  return show_data("trailer_value", parser, data, size);
}

isize on_error(Parser *parser,
               const char *data, usize size)
{
  usize position = milo_get_position(parser);
  usize error_code = milo_get_error_code(parser);
  auto error_code_string = milo_get_error_code_string(parser);
  auto error_code_description = milo_get_error_code_description(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu error code=%lu (%s) description=\"%s\"\n", position, error_code, error_code_string, error_code_description);

  milo_free_string(error_code_string);
  milo_free_string(error_code_description);

  return append_output(parser, message);
}

isize on_finish(Parser *parser,
                const char *data, usize size)
{
  usize position = milo_get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu finish\n", position);

  return 0;
}

isize on_request(Parser *parser,
                 const char *data, usize size)
{
  usize position = milo_get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu request\n", position);

  return 0;
}

isize on_response(Parser *parser,
                  const char *data, usize size)
{
  usize position = milo_get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu response\n", position);

  return 0;
}

isize on_message_start(Parser *parser,
                       const char *data, usize size)
{
  usize position = milo_get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu message_start\n", position);

  return 0;
}

isize on_message_complete(Parser *parser,
                          const char *data, usize size)
{
  usize position = milo_get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu message_complete\n", position);

  return 0;
}

isize on_method(Parser *parser,
                const char *data, usize size)
{
  return show_span(parser, "method", milo_get_span_method(parser));
}

isize on_method_complete(Parser *parser,
                         const char *data, usize size)
{
  return status_complete("method", parser);
}

isize on_url(Parser *parser,
             const char *data, usize size)
{
  return show_span(parser, "url", milo_get_span_url(parser));
}

isize on_url_complete(Parser *parser,
                      const char *data, usize size)
{
  return status_complete("url", parser);
}

isize on_protocol(Parser *parser,
                  const char *data, usize size)
{
  return show_span(parser, "protocol", milo_get_span_protocol(parser));
}

isize on_protocol_complete(Parser *parser,
                           const char *data, usize size)
{
  return status_complete("protocol", parser);
}

isize on_version(Parser *parser,
                 const char *data, usize size)
{
  return show_span(parser, "version", milo_get_span_version(parser));
}

isize on_version_complete(Parser *parser,
                          const char *data, usize size)
{
  return status_complete("version", parser);
}

isize on_status(Parser *parser,
                const char *data, usize size)
{
  return show_span(parser, "status", milo_get_span_status(parser));
}

isize on_status_complete(Parser *parser,
                         const char *data, usize size)
{
  return status_complete("status", parser);
}

isize on_reason(Parser *parser,
                const char *data, usize size)
{
  return show_span(parser, "reason", milo_get_span_reason(parser));
}

isize on_reason_complete(Parser *parser,
                         const char *data, usize size)
{
  return status_complete("reason", parser);
}

isize on_header_field(Parser *parser,
                      const char *data, usize size)
{
  return show_span(parser, "header_field", milo_get_span_header_field(parser));
}

isize on_header_field_complete(Parser *parser,
                               const char *data, usize size)
{
  return status_complete("header_field", parser);
}

isize on_header_value(Parser *parser,
                      const char *data, usize size)
{
  return show_span(parser, "header_value", milo_get_span_header_value(parser));
}

isize on_header_value_complete(Parser *parser,
                               const char *data, usize size)
{
  return status_complete("header_value", parser);
}

isize on_headers_complete(Parser *parser,
                          const char *data, usize size)
{
  usize position = milo_get_position(parser);
  auto version = milo_get_span_version(parser);
  usize content_length = milo_get_value_expected_content_length(parser);
  bool chunked = milo_get_value_has_chunked_transfer_encoding(parser) == 1;
  auto protocol = milo_get_span_protocol(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);

  if (milo_get_value_message_type(parser) == RESPONSE)
  {
    if (chunked)
    {
      snprintf(
          message,
          MAX_FORMAT,
          "pos=%lu headers complete type=response status=%lu protocol=%s v=%s chunked\n",
          position, milo_get_value_response_status(parser), protocol, version);
    }
    else if (content_length > 0)
    {
      snprintf(
          message,
          MAX_FORMAT,
          "pos=%lu headers complete type=response status=%lu protocol=%s v=%s content_length=%lu\n",
          position, milo_get_value_response_status(parser), protocol, version, content_length);
    }
    else
    {
      snprintf(
          message,
          MAX_FORMAT,
          "pos=%lu headers complete type=response status=%lu protocol=%s v=%s no-body\n",
          position, milo_get_value_response_status(parser), protocol, version);
    }
  }
  else
  {
    auto method = milo_get_span_method(parser);
    auto url = milo_get_span_url(parser);

    if (chunked)
    {
      snprintf(
          message,
          MAX_FORMAT,
          "pos=%lu headers complete type=request method=%s url=%s protocol=%s v=%s chunked\n",
          position, method, url, protocol, version);
    }
    else if (content_length > 0)
    {
      snprintf(
          message,
          MAX_FORMAT,
          "pos=%lu headers complete type=request method=%s url=%s protocol=%s v=%s content_length=%lu\n",
          position, method, url, protocol, version, content_length);
    }
    else
    {
      snprintf(
          message,
          MAX_FORMAT,
          "pos=%lu headers complete type=request method=%s url=%s protocol=%s v=%s no-body\n",
          position, method, url, protocol, version);
    }

    milo_free_string(method);
    milo_free_string(url);
  }

  milo_free_string(version);
  milo_free_string(protocol);

  return append_output(parser, message);
}

isize on_upgrade(Parser *parser,
                 const char *data, usize size)
{
  return status_complete("upgrade", parser);
}

isize on_chunk_length(Parser *parser,
                      const char *data, usize size)
{
  return show_span(parser, "chunk_length", milo_get_span_chunk_length(parser));
}

isize on_chunk_extension_name(Parser *parser,
                              const char *data, usize size)
{
  return show_span(parser, "chunk_extensions_name", milo_get_span_chunk_extension_name(parser));
}

isize on_chunk_extension_value(Parser *parser,
                               const char *data, usize size)
{
  return show_span(parser, "chunk_extension_value", milo_get_span_chunk_extension_value(parser));
}

isize on_chunk_data(Parser *parser,
                    const char *data, usize size)
{
  return show_span(parser, "chunk", milo_get_span_chunk_data(parser));
}

isize on_body(Parser *parser,
              const char *data, usize size)
{
  return show_span(parser, "body", milo_get_span_body(parser));
}

isize on_trailer_field(Parser *parser,
                       const char *data, usize size)
{
  return show_span(parser, "trailer_field", milo_get_span_trailer_field(parser));
}

isize on_trailer_value(Parser *parser,
                       const char *data, usize size)
{
  return show_span(parser, "trailer_value", milo_get_span_trailer_value(parser));
}

isize on_trailers_complete(Parser *parser,
                           const char *data, usize size)
{
  return status_complete("trailers", parser);
}

int main()
{
  auto parser = milo_init();

  const char *request1 = "GET / HTTP/1.1\r\n\r\n";
  const char *request2 = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n";

#ifdef MILO_TEST_DEBUG
  milo_set_on_data_method(parser, on_data_method);
  milo_set_on_data_url(parser, on_data_url);
  milo_set_on_data_protocol(parser, on_data_protocol);
  milo_set_on_data_version(parser, on_data_version);
  milo_set_on_data_header_field(parser, on_data_header_field);
  milo_set_on_data_header_value(parser, on_data_header_value);
  milo_set_on_data_chunk_length(parser, on_data_chunk_length);
  milo_set_on_data_chunk_extension_name(parser, on_data_chunk_extension_name);
  milo_set_on_data_chunk_extension_value(parser, on_data_chunk_extension_value);
  milo_set_on_data_chunk_data(parser, on_data_chunk_data);
  milo_set_on_data_body(parser, on_data_body);
  milo_set_on_data_trailer_field(parser, on_data_trailer_field);
  milo_set_on_data_trailer_value(parser, on_data_trailer_value);
#endif

  milo_set_on_error(parser, on_error);
  milo_set_on_finish(parser, on_finish);
  milo_set_on_request(parser, on_request);
  milo_set_on_response(parser, on_response);
  milo_set_on_message_start(parser, on_message_start);
  milo_set_on_message_complete(parser, on_message_complete);
  milo_set_on_method(parser, on_method);
  milo_set_on_method_complete(parser, on_method_complete);
  milo_set_on_url(parser, on_url);
  milo_set_on_url_complete(parser, on_url_complete);
  milo_set_on_protocol(parser, on_protocol);
  milo_set_on_protocol_complete(parser, on_protocol_complete);
  milo_set_on_version(parser, on_version);
  milo_set_on_version_complete(parser, on_version_complete);
  milo_set_on_status(parser, on_status);
  milo_set_on_status_complete(parser, on_status_complete);
  milo_set_on_reason(parser, on_reason);
  milo_set_on_reason_complete(parser, on_reason_complete);
  milo_set_on_header_field(parser, on_header_field);
  milo_set_on_header_field_complete(parser, on_header_field_complete);
  milo_set_on_header_value(parser, on_header_value);
  milo_set_on_header_value_complete(parser, on_header_value_complete);
  milo_set_on_headers_complete(parser, on_headers_complete);
  milo_set_on_body(parser, on_upgrade);
  milo_set_on_chunk_length(parser, on_chunk_length);
  milo_set_on_chunk_extension_name(parser, on_chunk_extension_name);
  milo_set_on_chunk_extension_value(parser, on_chunk_extension_value);
  milo_set_on_chunk_data(parser, on_chunk_data);
  milo_set_on_body(parser, on_body);
  milo_set_on_trailer_field(parser, on_trailer_field);
  milo_set_on_trailer_value(parser, on_trailer_value);
  milo_set_on_trailers_complete(parser, on_trailers_complete);

  usize consumed = milo_parse(parser, request1);
  usize pos = milo_get_position(parser);
  auto *state = milo_get_state_string(parser);

  printf("pos=%lu consumed=%lu state=%s\n", pos, consumed, state);
  milo_free_string(state);

  printf("--- --- --- ---\n");
  consumed = milo_parse(parser, request2);
  pos = milo_get_position(parser);
  state = milo_get_state_string(parser);

  printf("pos=%lu consumed=%lu state=%s\n", pos, consumed, state);

  milo_free(parser);
}
