namespace outer {
using ns::x;
using ns::T;
using ::global_fn;
using ns::a, ns::b;
using ns::detail::deep;
using enum ns::Color;
}
template <class B> struct W : B { using typename B::U; };
