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

int publicGlobal = 6;

static int fileStaticVariable = 7;

static void fileStaticFunction() {}

extern "C" void api(void);

int functionWithLocalStatic() {
    static int localStaticCounter = 0;
    return ++localStaticCounter;
}
