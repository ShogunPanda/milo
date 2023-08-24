#include "milo.h"
#include "stdio.h"

typedef intptr_t isize;
typedef uintptr_t usize;
const usize MAX_FORMAT = 1000;

isize append_output(milo::Parser *parser, const char *message)
{
  printf("%s", message);
  return 0;
}

isize show_data(const char *name, milo::Parser *parser, const char *data, usize size)
{
  usize position = milo::get_position(parser);
  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu data[%s]=\"%s\" (len=%lu)\n", position, name, data, size);
  return append_output(parser, message);
}

isize show_span(milo::Parser *parser, const char *name, const char *value)
{
  usize position = milo::get_position(parser);
  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu span[%s]=\"%s\"\n", position, name, value);
  return append_output(parser, message);
}

isize status_complete(const char *name, milo::Parser *parser)
{
  usize position = milo::get_position(parser);
  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu %s complete\n", position, name);
  return append_output(parser, message);
}

isize on_data_method(milo::Parser *parser, const char *data, usize size)
{
  return show_data("method", parser, data, size);
}

isize on_data_url(milo::Parser *parser, const char *data, usize size)
{
  return show_data("url", parser, data, size);
}

isize on_data_protocol(milo::Parser *parser, const char *data, usize size)
{
  return show_data("protocol", parser, data, size);
}

isize on_data_version(milo::Parser *parser, const char *data, usize size)
{
  return show_data("version", parser, data, size);
}

isize on_data_header_field(milo::Parser *parser, const char *data, usize size)
{
  return show_data("header_field", parser, data, size);
}

isize on_data_header_value(milo::Parser *parser, const char *data, usize size)
{
  return show_data("header_value", parser, data, size);
}

isize on_data_chunk_length(milo::Parser *parser, const char *data, usize size)
{
  return show_data("chunk_length", parser, data, size);
}

isize on_data_chunk_extension_name(milo::Parser *parser, const char *data, usize size)
{
  return show_data("chunk_extension_name", parser, data, size);
}

isize on_data_chunk_extension_value(milo::Parser *parser, const char *data, usize size)
{
  return show_data("chunk_extension_value", parser, data, size);
}

isize on_data_chunk_data(milo::Parser *parser, const char *data, usize size)
{
  return show_data("chunk_data", parser, data, size);
}

isize on_data_body(milo::Parser *parser, const char *data, usize size)
{
  return show_data("body", parser, data, size);
}

isize on_data_trailer_field(milo::Parser *parser, const char *data, usize size)
{
  return show_data("trailer_field", parser, data, size);
}

isize on_data_trailer_value(milo::Parser *parser, const char *data, usize size)
{
  return show_data("trailer_value", parser, data, size);
}

isize on_error(milo::Parser *parser,
               const char *data, usize size)
{
  usize position = milo::get_position(parser);
  usize error_code = milo::get_error_code(parser);
  auto error_code_string = milo::get_error_code_string(parser);
  auto error_code_description = milo::get_error_code_description(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu error code=%lu (%s) description=\"%s\"\n", position, error_code, error_code_string, error_code_description);

  milo::free_string(error_code_string);
  milo::free_string(error_code_description);

  return append_output(parser, message);
}

isize on_finish(milo::Parser *parser,
                const char *data, usize size)
{
  usize position = milo::get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu finish\n", position);

  return 0;
}

isize on_request(milo::Parser *parser,
                 const char *data, usize size)
{
  usize position = milo::get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu request\n", position);

  return 0;
}

isize on_response(milo::Parser *parser,
                  const char *data, usize size)
{
  usize position = milo::get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu response\n", position);

  return 0;
}

isize on_message_start(milo::Parser *parser,
                       const char *data, usize size)
{
  usize position = milo::get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu message_start\n", position);

  return 0;
}

isize on_message_complete(milo::Parser *parser,
                          const char *data, usize size)
{
  usize position = milo::get_position(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);
  snprintf(message, MAX_FORMAT, "pos=%lu message_complete\n", position);

  return 0;
}

isize on_method(milo::Parser *parser,
                const char *data, usize size)
{
  return show_span(parser, "method", milo::get_method_string(parser));
}

isize on_method_complete(milo::Parser *parser,
                         const char *data, usize size)
{
  return status_complete("method", parser);
}

isize on_url(milo::Parser *parser,
             const char *data, usize size)
{
  return show_span(parser, "url", milo::get_url_string(parser));
}

isize on_url_complete(milo::Parser *parser,
                      const char *data, usize size)
{
  return status_complete("url", parser);
}

isize on_protocol(milo::Parser *parser,
                  const char *data, usize size)
{
  return show_span(parser, "protocol", milo::get_protocol_string(parser));
}

isize on_protocol_complete(milo::Parser *parser,
                           const char *data, usize size)
{
  return status_complete("protocol", parser);
}

isize on_version(milo::Parser *parser,
                 const char *data, usize size)
{
  return show_span(parser, "version", milo::get_version_string(parser));
}

isize on_version_complete(milo::Parser *parser,
                          const char *data, usize size)
{
  return status_complete("version", parser);
}

isize on_status(milo::Parser *parser,
                const char *data, usize size)
{
  return show_span(parser, "status", milo::get_status_string(parser));
}

isize on_status_complete(milo::Parser *parser,
                         const char *data, usize size)
{
  return status_complete("status", parser);
}

isize on_reason(milo::Parser *parser,
                const char *data, usize size)
{
  return show_span(parser, "reason", milo::get_reason_string(parser));
}

isize on_reason_complete(milo::Parser *parser,
                         const char *data, usize size)
{
  return status_complete("reason", parser);
}

isize on_header_field(milo::Parser *parser,
                      const char *data, usize size)
{
  return show_span(parser, "header_field", milo::get_header_field_string(parser));
}

isize on_header_field_complete(milo::Parser *parser,
                               const char *data, usize size)
{
  return status_complete("header_field", parser);
}

isize on_header_value(milo::Parser *parser,
                      const char *data, usize size)
{
  return show_span(parser, "header_value", milo::get_header_value_string(parser));
}

isize on_header_value_complete(milo::Parser *parser,
                               const char *data, usize size)
{
  return status_complete("header_value", parser);
}

isize on_headers_complete(milo::Parser *parser,
                          const char *data, usize size)
{
  usize position = milo::get_position(parser);
  auto version = milo::get_version_string(parser);
  usize content_length = milo::get_expected_content_length(parser);
  bool chunked = milo::get_has_chunked_transfer_encoding(parser) == 1;
  auto protocol = milo::get_protocol_string(parser);

  auto *message = (char *)malloc(sizeof(char) * MAX_FORMAT);

  if (milo::get_message_type(parser) == milo::RESPONSE)
  {
    if (chunked)
    {
      snprintf(
          message,
          MAX_FORMAT,
          "pos=%lu headers complete type=response status=%lu protocol=%s v=%s chunked\n",
          position, milo::get_status(parser), protocol, version);
    }
    else if (content_length > 0)
    {
      snprintf(
          message,
          MAX_FORMAT,
          "pos=%lu headers complete type=response status=%lu protocol=%s v=%s content_length=%lu\n",
          position, milo::get_status(parser), protocol, version, content_length);
    }
    else
    {
      snprintf(
          message,
          MAX_FORMAT,
          "pos=%lu headers complete type=response status=%lu protocol=%s v=%s no-body\n",
          position, milo::get_status(parser), protocol, version);
    }
  }
  else
  {
    auto method = milo::get_method_string(parser);
    auto url = milo::get_url_string(parser);

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

    milo::free_string(method);
    milo::free_string(url);
  }

  milo::free_string(version);
  milo::free_string(protocol);

  return append_output(parser, message);
}

isize on_upgrade(milo::Parser *parser,
                 const char *data, usize size)
{
  return status_complete("upgrade", parser);
}

isize on_chunk_length(milo::Parser *parser,
                      const char *data, usize size)
{
  return show_span(parser, "chunk_length", milo::get_chunk_length_string(parser));
}

isize on_chunk_extension_name(milo::Parser *parser,
                              const char *data, usize size)
{
  return show_span(parser, "chunk_extensions_name", milo::get_chunk_extension_name_string(parser));
}

isize on_chunk_extension_value(milo::Parser *parser,
                               const char *data, usize size)
{
  return show_span(parser, "chunk_extension_value", milo::get_chunk_extension_value_string(parser));
}

isize on_chunk_data(milo::Parser *parser,
                    const char *data, usize size)
{
  return show_span(parser, "chunk", milo::get_chunk_data_string(parser));
}

isize on_body(milo::Parser *parser,
              const char *data, usize size)
{
  return show_span(parser, "body", milo::get_body_string(parser));
}

isize on_trailer_field(milo::Parser *parser,
                       const char *data, usize size)
{
  return show_span(parser, "trailer_field", milo::get_trailer_field_string(parser));
}

isize on_trailer_value(milo::Parser *parser,
                       const char *data, usize size)
{
  return show_span(parser, "trailer_value", milo::get_trailer_value_string(parser));
}

isize on_trailers_complete(milo::Parser *parser,
                           const char *data, usize size)
{
  return status_complete("trailers", parser);
}

int main()
{
  auto parser = milo::create_parser();

  const char *request1 = "GET / HTTP/1.1\r\n\r\n";
  const char *request2 = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n";

#ifdef milo::TEST_DEBUG
  milo::set_on_data_method(parser, on_data_method);
  milo::set_on_data_url(parser, on_data_url);
  milo::set_on_data_protocol(parser, on_data_protocol);
  milo::set_on_data_version(parser, on_data_version);
  milo::set_on_data_header_field(parser, on_data_header_field);
  milo::set_on_data_header_value(parser, on_data_header_value);
  milo::set_on_data_chunk_length(parser, on_data_chunk_length);
  milo::set_on_data_chunk_extension_name(parser, on_data_chunk_extension_name);
  milo::set_on_data_chunk_extension_value(parser, on_data_chunk_extension_value);
  milo::set_on_data_chunk_data(parser, on_data_chunk_data);
  milo::set_on_data_body(parser, on_data_body);
  milo::set_on_data_trailer_field(parser, on_data_trailer_field);
  milo::set_on_data_trailer_value(parser, on_data_trailer_value);
#endif

  milo::set_on_error(parser, on_error);
  milo::set_on_finish(parser, on_finish);
  milo::set_on_request(parser, on_request);
  milo::set_on_response(parser, on_response);
  milo::set_on_message_start(parser, on_message_start);
  milo::set_on_message_complete(parser, on_message_complete);
  milo::set_on_method(parser, on_method);
  milo::set_on_method_complete(parser, on_method_complete);
  milo::set_on_url(parser, on_url);
  milo::set_on_url_complete(parser, on_url_complete);
  milo::set_on_protocol(parser, on_protocol);
  milo::set_on_protocol_complete(parser, on_protocol_complete);
  milo::set_on_version(parser, on_version);
  milo::set_on_version_complete(parser, on_version_complete);
  milo::set_on_status(parser, on_status);
  milo::set_on_status_complete(parser, on_status_complete);
  milo::set_on_reason(parser, on_reason);
  milo::set_on_reason_complete(parser, on_reason_complete);
  milo::set_on_header_field(parser, on_header_field);
  milo::set_on_header_field_complete(parser, on_header_field_complete);
  milo::set_on_header_value(parser, on_header_value);
  milo::set_on_header_value_complete(parser, on_header_value_complete);
  milo::set_on_headers_complete(parser, on_headers_complete);
  milo::set_on_body(parser, on_upgrade);
  milo::set_on_chunk_length(parser, on_chunk_length);
  milo::set_on_chunk_extension_name(parser, on_chunk_extension_name);
  milo::set_on_chunk_extension_value(parser, on_chunk_extension_value);
  milo::set_on_chunk_data(parser, on_chunk_data);
  milo::set_on_body(parser, on_body);
  milo::set_on_trailer_field(parser, on_trailer_field);
  milo::set_on_trailer_value(parser, on_trailer_value);
  milo::set_on_trailers_complete(parser, on_trailers_complete);

  usize consumed = milo::parse(parser, request1, 0, strlen(request1));
  usize pos = milo::get_position(parser);
  auto *state = milo::get_state_string(parser);

  printf("pos=%lu consumed=%lu state=%s\n", pos, consumed, state);
  milo::free_string(state);

  printf("--- --- --- ---\n");
  consumed = milo::parse(parser, request2, 0, strlen(request2));
  pos = milo::get_position(parser);
  state = milo::get_state_string(parser);

  printf("pos=%lu consumed=%lu state=%s\n", pos, consumed, state);

  milo::free_parser(parser);
}
