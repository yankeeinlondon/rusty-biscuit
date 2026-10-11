struct D : B {
    using B::x;
    using B::B;
    using Base = B;
    typedef int I;
    using typename B::T;
private:
    using B::y;
};
