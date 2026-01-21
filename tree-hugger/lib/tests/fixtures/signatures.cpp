#include <string>
#include <vector>

// A simple greeting function.
std::string greet(std::string name) {
    return "Hello, " + name + "!";
}

// A function with multiple parameters.
int add(int a, int b) {
    return a + b;
}

// A function with default parameter.
std::string greetWithPrefix(std::string name, std::string prefix = "Hello") {
    return prefix + ", " + name + "!";
}

// A class with methods.
class Greeter {
public:
    Greeter(std::string prefix);
    std::string greet(std::string name);
    void greetMany(std::vector<std::string> names);

private:
    std::string prefix_;
};

Greeter::Greeter(std::string prefix) : prefix_(prefix) {}

std::string Greeter::greet(std::string name) {
    return prefix_ + ", " + name + "!";
}

void Greeter::greetMany(std::vector<std::string> names) {
    for (const auto& name : names) {
        greet(name);
    }
}
