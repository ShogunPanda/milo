#include "milo.h"
#include "stdio.h"
#include "string.h"

int main() {
  // Create the parser.
  milo::Parser* parser = milo::milo_create();

  // Prepare a message to parse.
  const char* message = "HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc";

  parser->context = (char*) message;

  /*
    Milo works using callbacks.

    All callbacks have the same signature, which characterizes the payload:

      * p: The current parser.
      * at: The payload offset.
      * len: The payload length.

    The payload parameters above are relative to the last data sent to the milo_parse method.

    If the current callback has no payload, both values are set to 0.
  */
  parser->callbacks.on_data = [](milo::Parser* p, uintptr_t from, uintptr_t size) {
    char* payload = reinterpret_cast<char*>(malloc(sizeof(char) * size));
    strncpy(payload, reinterpret_cast<const char*>(p->context) + from, size);

    printf("Pos=%lu Body: %s\n", p->position, payload);
    free(payload);
  };

  // Now perform the main parsing using milo.parse. The method returns the number of consumed characters.
  milo::milo_parse(parser, reinterpret_cast<const unsigned char*>(message), strlen(message));

  // Cleanup used resources.
  milo::milo_destroy(parser);
}