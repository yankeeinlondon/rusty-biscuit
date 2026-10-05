/* Symbol-kind, visibility, and storage-static fixture for C. */
#include <stddef.h>

#define FIXTURE_MACRO 1

struct NamedTag {
    int tag_field;
    char *tag_name_field;
};

typedef struct AliasedTag {
    int aliased_tag_field;
} TagAlias;

typedef struct {
    int anonymous_struct_field;
} AnonymousStructAlias;

typedef struct SameName {
    int same_name_field;
} SameName;

typedef unsigned long CountAlias;

enum NamedColor {
    COLOR_RED,
    COLOR_GREEN,
};

union NamedUnion {
    int union_int_field;
    float union_float_field;
};

int public_global_variable = 1;

static int file_static_variable = 2;

extern int extern_declared_variable;

int public_function(int parameter_value) {
    int local_value = parameter_value + 1;
    return local_value;
}

static int file_static_function(void) {
    return 0;
}

static int prototyped_helper(int);

int prototyped_helper(int helper_value) {
    return helper_value;
}

int function_with_local_static(void) {
    static int local_static_counter = 0;
    return ++local_static_counter;
}

/* ---- Declaration boundaries (keyword and signature text) ---- */

/* A comment naming declaration keywords: struct enum union typedef static */
const int boundary_table[] = {
    1, 2,
    3, 4,
};

int boundary_multi_line_header(
    int first_value,
    int second_value) {
    return first_value + second_value;
}

[[maybe_unused]] int boundary_annotated(void) {
    return 0;
}

/* ---- Flags and metrics ---- */

/** Documented, with two parameters. */
void documented_pair(int first, int second) {}

/* deprecated: an ordinary comment */
void undocumented_none(void) {}

void variadic(int first, ...) {}

void test_helper(void) {}
