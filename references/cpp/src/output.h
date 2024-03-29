#ifndef MILO_OUTPUT_H
#define MILO_OUTPUT_H

#include "milo.h"
#include "stdio.h"
#include "string.h"

#include "utils.h"

void append_output(const milo::Parser* parser, uchar_t* message, const uchar_t* data, usize_t size);
void event(const milo::Parser* parser, const char* name, usize_t position, const uchar_t* data, usize_t size);
void show_span(const milo::Parser* parser, const char* name, const uchar_t* data, usize_t size);
#endif // MILO_OUTPUT_H