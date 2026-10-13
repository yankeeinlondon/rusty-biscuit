#include <vector>
namespace n { int x; void f(); }
int main() { auto y = n::x; return y; }
