export module m;
import other;
export {
    using other::f;
    int g();
    namespace ns2 { }
}
export using other::h;
export namespace ns3 { using other::i; }
export int local_fn();
export class K {};
export template <class T> void tf();
