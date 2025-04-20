#include "utils.h"

void clear_context(context_t* context) {
  if (context->input != NULL) {
    free(context->input);
    context->input = NULL;
  }

  if (context->method != NULL) {
    free(context->method);
    context->method = NULL;
  }

  if (context->url != NULL) {
    free(context->url);
    context->url = NULL;
  }

  if (context->protocol != NULL) {
    free(context->protocol);
    context->protocol = NULL;
  }

  if (context->version != NULL) {
    free(context->version);
    context->version = NULL;
  }
}

uchar_t* create_string() {
  return reinterpret_cast<uchar_t*>(calloc(MAX_FORMAT, sizeof(uchar_t)));
}
