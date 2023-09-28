#ifndef MILO_UTILS_H
#define MILO_UTILS_H

#include "stdio.h"
#include <cstdlib>
#define MAX_FORMAT 1000

typedef intptr_t isize_t;
typedef uintptr_t usize_t;
typedef unsigned char uchar_t;

struct context_t {
  uchar_t* input;
  uchar_t* method;
  uchar_t* url;
  uchar_t* protocol;
  uchar_t* version;
};

void clear_context(context_t* context);
uchar_t* create_string();
#endif // MILO_UTILS_H