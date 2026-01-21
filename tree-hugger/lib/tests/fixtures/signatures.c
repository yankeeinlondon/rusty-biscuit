/* A simple greeting function. */
char* greet(char* name) {
    return "Hello";
}

/* A function with multiple parameters. */
int add(int a, int b) {
    return a + b;
}

/* A void function with pointer parameter. */
void print_greeting(const char* message) {
    /* print message */
}

/* A function with array parameter. */
int sum_array(int* numbers, int length) {
    int total = 0;
    for (int i = 0; i < length; i++) {
        total += numbers[i];
    }
    return total;
}

/* A function pointer parameter. */
void apply(int (*func)(int), int value) {
    func(value);
}
