// A 2D point structure.
struct Point {
    int x;
    int y;
};

// A greeter class.
class Greeter {
public:
    void greet();
private:
    std::string prefix;
};

// Status codes for responses.
enum Status {
    SUCCESS,
    ERROR,
    PENDING
};

// Scoped enum (C++11).
enum class Color {
    Red,
    Green,
    Blue
};
