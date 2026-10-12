#ifdef USE_A
#include "a.h"
#else
#include "b.h"
#endif
#if defined(X) && X > 1
#define EXPORTED a_fn
#endif
#if __has_include(<x.h>)
#include <x.h>
#endif
