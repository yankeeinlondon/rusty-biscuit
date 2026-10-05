// Symbol-kind, visibility, static, and linkage fixture for C++.
#include <string>

namespace fixture_space {

class AccessWidget {
    int implicitPrivateField;
    void implicitPrivateMethod();

public:
    AccessWidget();
    int firstPublicField;
    int secondPublicField;
    void firstPublicMethod();
    void secondPublicMethod() {}
    static int staticMemberField;
    static AccessWidget staticFactory();
    int instanceMethod() const { return firstPublicField; }
    int countCalls() {
        static int memberLocalStatic = 0;
        return ++memberLocalStatic;
    }

protected:
    int protectedField;
    void protectedMethod() {}

private:
    int explicitPrivateField;
    void explicitPrivateMethod() {}
};

struct PlainStruct {
    int structPublicField;
    void structPublicMethod() {}

private:
    int structPrivateField;
};

union PlainUnion {
    int unionIntField;
    float unionFloatField;
};

enum class ScopedColor { Red, Green };

enum PlainColor { PLAIN_RED, PLAIN_GREEN };

using SizeAlias = unsigned long;

typedef int IntTypedef;

int namespaceFunction(int parameterValue) {
    int localValue = parameterValue + 1;
    return localValue;
}

}  // namespace fixture_space

static int helper(int);

int helper(int helperValue) { return helperValue; }

int helper(long helperValue);

namespace {
int hiddenVariable;

void hiddenFunction() {}
}  // namespace

const int LIMIT = 3;

extern const int SHARED = 4;

inline const int INLINE_LIMIT = 5;

constexpr int CONSTEXPR_LIMIT = 8;

const char* constCharPointer = "public";

char* const constPointerToChar = nullptr;

int publicGlobal = 6;

static int fileStaticVariable = 7;

static void fileStaticFunction() {}

extern "C" void api(void);

int functionWithLocalStatic() {
    static int localStaticCounter = 0;
    return ++localStaticCounter;
}

struct DeclaratorFieldForms {
    int values[3];
    int **pointer;
    int (*callback)(int);
    template<class T> static T cache[3];
};

// ---- Declaration boundaries (keyword and signature text) ----

// A comment naming declaration keywords: class struct namespace enum template
const int boundaryTable[] = {
    1, 2,
    3, 4,
};

int boundaryMultiLineHeader(
    int firstValue,
    int secondValue) {
    return firstValue + secondValue;
}

[[nodiscard]] int boundaryAnnotated() {
    return 0;
}

// ---- Scope and qualified names ----

namespace outer_space {
namespace inner_space {
struct FirstShape {
    int area();
};
}  // namespace inner_space
}  // namespace outer_space

struct SecondShape {
    int area();
};

// ---- Flags and metrics ----

/// Documented, with two parameters.
void documented_pair(int first, int second) {}

// deprecated: an ordinary comment
void undocumented_none() {}

[[deprecated("use documented_pair")]] void deprecated_api() {}

[[gnu::deprecated]] void namespaced_attribute() {}

void test_helper() {}

void variadic(int first, ...) {}

class [[deprecated]] DeprecatedClass {};
