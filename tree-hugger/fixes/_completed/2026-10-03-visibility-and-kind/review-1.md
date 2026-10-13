---
$schema: feature-review.yaml
ready: false
findings:
  - title: Earlier static declarations do not reliably reach later C and C++ functions
    priority: high
  - title: C++ template wrappers break member kinds and static flags
    priority: high
  - title: C# local functions are incorrectly listed as class methods
    priority: high
  - title: Python and Scala infer static membership from unrelated syntax
    priority: high
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-03T20:44:50-07:00"
spec: 2026-10-03-visibility-and-kind/spec.md
implemented: true
next: 2026-10-03-visibility-and-kind/review-2.md
implemented_by: claude/opus
log: tree-hugger/fixes/2026-10-03-visibility-and-kind/log.md
description: "A **fix** review of `2026-10-03-visibility-and-kind/spec.md`"
fix: 2026-10-03-visibility-and-kind/review-1.md
---

The fix is **not production ready**. The primary fixtures pass, but variations of valid declarations still produce incorrect access, kinds, and static membership. These errors reach the shipped CLI, including missing methods and invented instance methods in `hug classes`. No human decision is needed to resolve them.

Verification used the tree-hugger and rust-testing skills. `just test` passed all **658 L1 tests**, `just lint` passed, and the root `just check-tier-coverage tree-hugger` reported no stranded tests. The consolidated library target declares the new test modules; Cargo discovers the CLI `symbols` test binary. The implementation changes and current documentation were reviewed alongside the specification; the plan's restoration showed no additional actionable defect.

For the sweeps below, copies of the primary files in `lib/tests/fixtures/symbols/` were placed in a scratch Git repository, with the shown declarations appended. Results were checked through the current `target/debug/hug --json symbols --no-cache <file>` binary; affected member lists were also checked with `hug --plain classes --no-cache <file>`. Existing fixture controls passed before the variations were introduced. False static flags are omitted in JSON; tables render an omitted flag as `false`.

## Findings

### High — Earlier static declarations do not reliably reach later C and C++ functions

**Defect class:** same-file linkage inference incompletely enumerates declarations and includes non-type syntax in the function identity, causing later declarations of internally linked functions to be reported as Public.

In the tree-hugger library, [has_earlier_static_declaration](../../lib/src/file/extract/visibility/c.rs:135) implements the rule used by both C and C++ visibility readers. Its helper takes only the first `declarator` field of a declaration. The declaration collector does not enter `template_declaration`, and parameter comparison retains default argument expressions. These are three sibling failures of the same linkage rule, rather than three separate findings.

```c
static int multiFirst(int), multiSecond(int);
int multiSecond(int x) { return x; }
```

```cpp
static int defaultProbe(int x = 0);
int defaultProbe(int x) { return x; }

template<class T> static T templateLinkProbe(T x);
template<class T> T templateLinkProbe(T x) { return x; }
```

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C reader, shared linkage scan | Second prototype in one `static` declaration, followed by definition | Prototype ModulePrivate; definition Public | Both ModulePrivate |
| C++ reader, shared linkage scan | Same multiple-prototype shape | Prototype ModulePrivate; definition Public | Both ModulePrivate |
| C++ parameter identity | Prototype has a default argument; definition does not | Definition Public | ModulePrivate; the default expression is not a parameter type |
| C++ declaration collector | Earlier static function template; later definition without `static` | Definition Public | ModulePrivate |
| C and C++ controls | Single earlier static prototype with unnamed parameter, followed by named parameter | Both ModulePrivate | Both ModulePrivate; clean |
| C++ pointer-return control | `static char* pointerProbe(int);` followed by definition | Both ModulePrivate | Both ModulePrivate; clean |
| C++ overload control | Same name, `long` instead of `int` | Public | Public; clean |
| C++ namespace control | Same name and parameter type, different namespace | Public | Public; clean |

These are the only readers that implement this linkage decision; the other 14 languages do not use this scan. The overload and namespace controls must remain clean when fixing it.

Enumerate every function declarator of each earlier declaration, traverse template wrappers, and compare parameter types without parameter names or default expressions. Add L1 regressions for all failing rows through both `TreeFile::symbols()` and `symbol_index_v2()`. The existing earlier-prototype test covers only a single declarator and a simple parameter.

### High — C++ template wrappers break member kinds and static flags

**Defect class:** member detection treats a transparent declaration wrapper as a scope boundary, so wrapped members lose their kind and receive storage-static flags instead of member-static flags.

In the tree-hugger library, [is_direct_member](../../lib/src/file/extract/classify.rs:196) does not traverse `template_declaration`, while [is_class_member](../../lib/src/file/extract/static_flags/cpp.rs:22) requires the immediate parent to be the class body. The C++ visibility reader already handles the wrapper correctly, so a declaration can have Public class-member access while being classified as a free function with static storage.

```cpp
class TemplateSweep {
public:
    template<class T> static void staticProbe(T x) {}
    template<class T> void instanceProbe(T x) {}
    template<class T> static T fieldProbe;
};
```

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C++ kind classifier | Static member function template | Function | Method |
| C++ kind classifier | Instance member function template | Function | Method |
| C++ field classifier | Static member variable template | Variable | Field |
| C++ member/storage static readers | Static method and variable templates | `is_static=false`, `is_static_storage=true` | `is_static=true`, `is_static_storage=false` |
| C++ visibility reader | Same three public template members | Public | Public; clean |
| CLI class projection | Same method templates | Both omitted from method lists | Static and instance method sections respectively |
| C++ control | Ordinary in-class static method | Method, member-static only, listed | Same; clean |
| Rust, TS, JS, Python, Java, C#, Swift, Scala, PHP controls | Ordinary members, including existing decorated Python methods and Rust impl members | Method | Method; clean |

C has no C++ template wrapper. Go, Lua, Perl, Bash, and Zsh have no applicable class-template member form. The language-specific static-reader sweep is also recorded in the fourth finding.

Normalize the path from the declaration to its nearest member owner consistently for kind, member-static, and storage-static decisions. Add public-result L1 tests for the three wrapped declarations and an isolated CLI check proving that the methods appear in their correct class sections. Do not fix only the CLI: the records themselves are wrong.

### High — C# local functions are incorrectly listed as class methods

**Defect class:** a method query capture is trusted without validating that the declaration belongs directly to a type, causing function-local declarations to become class members.

The tree-hugger library's [C# locals query](../../lib/queries/vendor/c_sharp/locals.scm:26) captures `local_function_statement` as a method. [refine_symbol_kind](../../lib/src/file/extract/classify.rs:68) promotes Function captures for direct members but never corrects Method captures for nested functions.

```csharp
class Sweep {
    public void Outer() {
        void nestedProbe() {}
        static void nestedStaticProbe() {}
    }
}
```

Both local functions are Method records with container `Sweep`. `hug classes` lists both under Instance Methods. Their Unknown visibility and false member-static flag are correct for local declarations, but their kind is not: the specification says a function nested inside another function remains Function.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| C# query and classifier | Ordinary and static local functions inside a method | Method; both listed as instance methods | Function; neither listed as a method |
| Rust | Named function nested in an impl method | Function, Unknown access | Same; clean |
| TypeScript and JavaScript | Named function nested in a class method | Function, Unknown access | Same; clean |
| Python | Named function nested in a class method | Function, Unknown access | Same; clean |
| Swift | Named function nested in a type method | Function, Unknown access | Same; clean |
| Scala | Named function nested in a class method | Function, Unknown access | Same; clean |
| PHP | Named function nested in a class method | Function, Unknown access | Same; clean |
| Lua and Perl | Named function nested in another function/sub | Function, Unknown access | Same; clean |
| Bash and Zsh | Function declaration inside another function | Function, Unknown access | Same; clean |
| Go | Local binding to a function literal | Variable, Unknown access | Same; clean; no named nested function syntax |
| Java | Local binding inside a lambda | Variable, Unknown access | Same; clean; no named nested function syntax |
| C and C++ | Function-local variable controls in primary fixtures | Variable, Unknown access | Same; clean; standard C/C++ do not allow nested function definitions |

Capture C# local functions as Function, or make the classifier validate Method captures against the direct-member rule. Add L1 assertions for both local-function forms and a CLI assertion that neither appears in `hug classes`. This also prevents future query changes from silently inventing methods.

### High — Python and Scala infer static membership from unrelated syntax

**Defect class:** static-member readers use broad text or ancestor searches instead of the declaration's actual decorator or nearest owning type, creating false static members.

In the tree-hugger library, [extract_python_is_static](../../lib/src/file/extract/static_flags/python.rs:6) uses substring matching, and [extract_scala_is_static](../../lib/src/file/extract/static_flags/scala.rs:4) stops at a class but passes through a trait to an enclosing object.

```python
class Sweep:
    @notstaticmethod
    def decoratorProbe(self): pass
    @notclassmethod
    def decoratorClassProbe(self): pass
```

The decorators may be ordinary identity decorators defined elsewhere; their names do not make these methods static. Both currently report `is_static=true` and appear under Static Methods.

```scala
object SweepObject {
    trait InnerTrait { def traitProbe(): Unit }
    class InnerClass { def instanceProbe(): Unit = () }
}
```

`traitProbe` belongs to instances implementing `InnerTrait`, but reports `is_static=true` and appears under that trait's Static Methods. The nested class method correctly stays non-static.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Python static reader | `@notstaticmethod`, `@notclassmethod` | Both static | Both instance |
| Python controls | Actual `@staticmethod`, `@classmethod`, and undecorated method | True, true, false | Same; clean |
| Scala static reader | Trait method inside an object | Static | Instance |
| Scala controls | Direct object method; method in class nested in object | True; false | Same; clean |
| C++ static reader | Wrapped member templates | Storage-static instead of member-static | See second finding; same decision swept |
| C++ controls | Ordinary static method; function-local static variable | Member-static; storage-static | Same; clean |
| Java and C# shared reader | Static and instance methods; existing Java interface constant | Correct split; constant static | Same; clean |
| TS and JS shared reader | Static and instance class methods | True; false | Same; clean |
| PHP reader | Static and instance methods | True; false | Same; clean |
| Swift reader | Static and instance methods | True; false | Same; clean |
| Rust reader | Associated function and self method | True; false | Same; clean |

These rows enumerate every member-static reader. C, Go, Lua, Perl, Bash, and Zsh return false for member-static; their applicable local/function controls remained false. Rust and C/C++ storage-static cases are separately covered by the passing static matrix.

Match Python decorator expressions by the recognized decorator name rather than arbitrary substring occurrence. Stop Scala's search at every type owner, including traits, so an enclosing object's static grouping cannot leak into another type. Add negative controls alongside the current positive L1 matrix rows and verify the Python and Scala class sections through an isolated CLI run. These reader heuristics predate parts of this fix, but their results are now explicitly part of its static-membership contract.

## Verification levels and input robustness

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Kind, visibility, container, declaration text, and Rust trait relations | L1 public library matrices and exact-text tests | Appropriate level; boundary cases in the findings are missing |
| Adapter agreement, signatures, and schema version | L1 explicit conversions and serialization tests | Appropriate level |
| Constructor/function and alias/type inclusion; visibility labels and class grouping | L1 isolated CLI subprocess checks | Appropriate for this fix's textual and data contract; add the failing sweep cases |
| Pretty/plain label agreement | L1 forced-style subprocess output, with styling stripped | Proves labels; does not claim real-terminal rendering proof |
| Warm-cache invalidation and reuse | L1 isolated cache mutation, old-key seed, and hit control | Appropriate level |

The fix defines no keyboard, mouse, paste, scrolling, or specific emulator-rendering behavior. There is no new Level-2 or Level-3 mismatch; the existing god-files terminal tests concern other functionality. Cross-OS evidence is left to CI and did not affect readiness.

The persisted JSON symbol reader is relevant because the fix adds container kinds and static/visibility meanings. An additional review-time matrix mutated one field per cell in a snapshot produced by the shipped CLI, then checked the subsequent CLI result. A renamed record served as a cache-hit sentinel. The unedited control was served for each field; malformed snapshots were recomputed from source rather than accepted with permissive values.

| Shape | `identity.container_kind` | `visibility.level` | `modifiers` |
|---|---|---|---|
| Unedited control | Class, served | Private, served | Static, served |
| Absent | No container kind, served | Recomputed | Empty modifiers, served |
| Explicit null | No container kind, served | Recomputed | Recomputed |
| Wrong whole-field type (`123`) | Recomputed | Recomputed | Recomputed |
| One wrong array element | Recomputed; scalar field given an array | Recomputed; scalar field given an array | Recomputed for `["Static", 123]` |
| Every array element wrong | Recomputed | Recomputed | Recomputed for `[123]` |
| Empty | Empty string rejected; recomputed | Empty string rejected; recomputed | `[]` served as empty modifiers |
| Duplicate field key | Recomputed | Recomputed | Recomputed |
| Trailing garbage | Recomputed | Recomputed | Recomputed |

Absent container kinds explicitly mean unknown, and absent modifiers explicitly mean an empty list in this schema; those are allowed defaults rather than parser errors. A null container kind has the same optional meaning. JSON is the persisted format; this fix adds no manifest, lockfile, or configuration parser for other formats. The matrix above is review evidence, not a claim that these full corruption cases already have permanent regression tests.
