// Multi-file fixture: C++.

namespace geometry {

class Vector {
public:
    Vector(int x, int y) : x_(x), y_(y) {}

    int length() const { return x_ + y_; }

private:
    int x_;
    int y_;
};

int dot(const Vector &a, const Vector &b) {
    return a.length() * b.length();
}

}  // namespace geometry
