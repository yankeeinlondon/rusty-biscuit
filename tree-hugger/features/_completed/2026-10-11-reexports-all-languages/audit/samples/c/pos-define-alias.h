#define PUBLIC_FN other_fn
#define ALIAS_CONST OTHER_CONST
#define CALL(x) other_call(x)
typedef other_t my_t;
typedef struct other_s my_s;
extern int shared;
extern int other_fn(void);
extern "C" int not_valid_in_c;
