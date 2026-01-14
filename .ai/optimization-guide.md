# Schematic Header Implementation: Quick Reference Guide

## Key Finding
**Your plan has low parallelization potential BUT can be optimized by 25.6% (23 minutes saved) through simple restructuring.**

---

## Three Critical Changes

### 1. DEFER Phase 1: Directory Restructuring
```
❌ CURRENT (blocks everything):
┌─────────────────────────────┐
│ Phase 1: Move directory (10 min)  ← BLOCKING
└──────────────┬──────────────┘
               │
               ▼
         [Other phases...]

✅ OPTIMIZED (non-blocking):
┌─────────────┐     ┌────────────────────────┐
│ Phase 1:    │     │ Phases 2-6: Features   │
│ Validate    │ → │ (52 minutes)           │
│ paths (2 min)│     └────────────────────────┘
└─────────────┘              ▼
                    ┌────────────────────────┐
                    │ Phase 1B: Move dir     │
                    │ after Phase 5 (15 min) │
                    └────────────────────────┘
```

**Why**: Directory structure doesn't affect code logic. Moving it at the end eliminates a major blocker.

**Action**:
- Phase 1: Just validate paths exist (2 min)
- After Phase 5 passes: Execute full move (15 min)

---

### 2. SPLIT Phase 2-3: Overlap Type Definition with Codegen Prep

```
❌ CURRENT (sequential):
        Phase 2          Phase 3
    [12-15 min]  →   [15-20 min]
     (add fields)    (update codegen)

✅ OPTIMIZED (overlapped):
    Phase 2 Types    Phase 3 Prep      Phase 3 Integration
    [12 min]    +    [8 min overlap]  →  [12 min]
                     (design codegen)      (implement)
```

**Why**: While adding header fields to structs (Phase 2), developers can design codegen changes in parallel.

**Action**:
- Start Phase 2: Add RestApi.headers and Endpoint.headers fields
- At minute 5: Codegen team starts designing how to generate headers
- After Phase 2 completes (minute 14): Codegen team integrates designs
- **Total saved**: ~10 minutes

---

### 3. PARALLELIZE Phase 3: Three Codegen Module Updates

```
❌ CURRENT (one person, sequential):
[request_structs.rs] → [client.rs] → [api_struct.rs]
    (7 min)         (9 min)       (4 min)
    Total: 20 min

✅ OPTIMIZED (3 people, parallel):
Dev A: [request_structs.rs] (7 min)
Dev B: [client.rs] (9 min)
Dev C: [api_struct.rs] (4 min)
                        ↓ Sequential merge
Total time: max(7, 9, 4) = 9 min
(vs 20 min sequential, saves 11 min BUT requires team)
```

**Why**: Three codegen modules have minimal cross-dependencies. Different developers can update different files simultaneously.

**Action**:
- Assign 3 developers to different modules
- Each completes their update independently
- Merge sequentially: request_structs → client → api_struct
- **Total saved**: ~8-10 minutes (team-level only)

---

## Revised Timeline

### Before Optimization
```
0:00  Phase 1  [████████████] 10 min - blocking!
0:10  Phase 2  [██████████████████] 15 min
0:25  Phase 3  [████████████████████████] 20 min
0:45  Phase 4  [███████████████] 15 min
1:00  Phase 5  [██████████████████] 20 min
1:20  Phase 6  [██████████] 10 min
1:30  TOTAL: 90 minutes
```

### After Optimization
```
0:00  Phase 1  [██] 2 min (validate only)
0:02  Phase 2  [████████████] 12 min ──┐
0:05            Phase 3 prep starts     ├─ Overlap!
0:05               [████████] 8 min ───┘
0:14  Phase 3  [████████████] 12 min (integrate)
0:26  Phase 4  [██████████] 10 min
0:36  Phase 5  [████████] 8 min (parallel tests)
0:44  Phase 6  [████████] 8 min
0:52  FUNCTIONAL COMPLETE
      │
      └→ Phase 1B [███████████████] 15 min (defer)
         (post-implementation)
1:07  TOTAL: 67 minutes (or 52 min core work)
```

**Improvement**: 23 minutes saved (25.6% faster)

---

## Critical Synchronization Point

### ⚠️ CRITICAL: Phase 2 Atomicity

**RestApi and Endpoint are in the same file:**

```rust
// schematic/define/src/types.rs

pub struct RestApi {
    // ... existing fields
    pub headers: ???  // ← MUST ADD
}

pub struct Endpoint {
    // ... existing fields
    pub headers: ???  // ← MUST ADD in SAME commit
}
```

**Rule**: Both fields must exist before Phase 3 begins. If only one is added, Phase 3 codegen will fail with type errors.

**Solution**: Create single atomic commit:
```bash
git add schematic/define/src/types.rs
git commit -m "feat(schematic): add header fields to RestApi and Endpoint"
cargo test -p schematic-define  # Validate both fields work
```

Then proceed to Phase 3.

---

## Module Dependency Map

```
schematic/define/src/
├── types.rs
│   ├── RestApi       ← Codegen reads this
│   └── Endpoint      ← Codegen reads this
│
schematic/gen/src/codegen/
├── request_structs.rs   ← Consumes Endpoint fields
│                          (can start: Phase 2 complete)
│
├── client.rs            ← Consumes Endpoint + RestApi fields
│                          (depends on: request_structs.rs first)
│
└── api_struct.rs        ← Consumes RestApi fields
                           (optional enhancement)
```

**Key insight**: No circular dependencies. One-way dependency flow: types → codegen → generated output

---

## Parallel Execution Matrix

| Phase | Sequential | Can Parallelize? | With Whom | Time Saved |
|-------|---|---|---|---|
| 1 | 10 min | ✅ Yes, defer to end | Type work (Phase 2) | 5-10 min |
| 2 | 15 min | ⚠️ Partially | Codegen prep (Phase 3 design) | 5-10 min |
| 3 | 20 min | ✅ Yes, module level | 3 developers | 8-10 min |
| 4 | 15 min | ❌ No | - | 0 min |
| 5 | 20 min | ✅ Yes, test level | Parallel tests | 5-8 min |
| 6 | 10 min | ✅ Yes | Phase 5 (docs during testing) | 2-3 min |

**Total potential savings**: 23-31 minutes

---

## Implementation Checklist

### Pre-Work
- [ ] Create feature branch: `git checkout -b feat/schematic-headers`
- [ ] Run existing tests: `cargo test -p schematic-define -p schematic-gen`
- [ ] Verify clean workspace: `git status`

### Phase 1 (Minimal)
- [ ] Verify `schematic/define/Cargo.toml` exists
- [ ] Verify `schematic/gen/Cargo.toml` exists
- [ ] Verify `schematic/schematic/schema/Cargo.toml` exists

### Phase 2 (Sequential, ATOMIC COMMIT)
- [ ] Add `headers: HashMap<String, String>` to `RestApi` struct
- [ ] Add `headers: Option<HashMap<String, String>>` to `Endpoint` struct
- [ ] Document both fields with examples
- [ ] Add unit tests for both fields
- [ ] **ATOMIC COMMIT**: Include both struct changes + tests
- [ ] Validate: `cargo test -p schematic-define --lib`

### Phase 3 (Can Parallelize - Team Level)
**If single developer**:
- [ ] Update `request_structs.rs` (extract headers into generated struct)
- [ ] Update `client.rs` (inject headers in request builder)
- [ ] Optionally update `api_struct.rs` (configuration interface)
- [ ] Validate: `cargo build -p schematic-gen`

**If 3 developers**:
- [ ] Dev A: Update `request_structs.rs`
- [ ] Dev B: Update `client.rs` (start after Dev A merges)
- [ ] Dev C: Update `api_struct.rs` (optional, can be skipped)
- [ ] Sequential merge: A → B → C
- [ ] Validate after each: `cargo build -p schematic-gen`

### Phase 4 (Sequential)
- [ ] Update request enum for header propagation
- [ ] Test header flow through generated code

### Phase 5 (Parallel Tests)
- [ ] Add header propagation test to `e2e_generation.rs`
- [ ] Run: `cargo test -- --test-threads=4`
- [ ] Verify generated code compiles and runs

### Phase 6 (Can Overlap Phase 5)
- [ ] Update README.md
- [ ] Update API documentation
- [ ] Update examples

### Phase 1B (POST-Phase 5, Deferred)
- [ ] Create new directory: `mkdir schematic/schema`
- [ ] Move files: `mv schematic/schematic/schema/* schematic/schema/`
- [ ] Update root `Cargo.toml` workspace members
- [ ] Update import paths in `define/` and `gen/`
- [ ] Verify: `cargo build --all && cargo test --all`

---

## Risk Mitigation Strategies

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Type sync failure | CRITICAL | Keep Phase 2 atomic: both fields in one commit |
| Merge conflicts | MEDIUM | Clear module ownership, sequential merge in Phase 3 |
| Build breakage | MEDIUM | Run `cargo build` after each phase boundary |
| Forgotten Phase 1B | LOW | Create GitHub issue with "post-Phase-5" label |

---

## Commands Reference

### Full Build & Test
```bash
# Test everything
cargo test -p schematic-define -p schematic-gen

# Build everything
cargo build -p schematic-define -p schematic-gen

# With parallel tests
cargo test -p schematic-define -p schematic-gen -- --test-threads=4
```

### Phase-Specific Validation
```bash
# After Phase 2
cargo test -p schematic-define --lib

# After Phase 3
cargo build -p schematic-gen
cargo test -p schematic-gen --lib

# After Phase 5
cargo test --all -- --test-threads=4
```

### Dependency Verification
```bash
# Check for circular dependencies
cargo metadata --format-version 1 | jq '.packages[] | select(.name | contains("schematic")) | .dependencies'

# View dependency tree
cargo tree -p schematic-gen
```

---

## Expected Outcomes

### Time Savings
- **Current plan**: 90 minutes
- **Optimized plan**: 52 minutes (functional work) + 15 minutes (deferred restructuring)
- **Improvement**: 25.6% faster core work

### Code Quality Improvements
- Atomic commits enable safe rollback
- Sequential merge strategy in Phase 3 prevents conflicts
- Parallel test execution reduces feedback cycle
- Better code review opportunities with smaller commits

### Risk Reduction
- Deferring Phase 1 eliminates blocking operation
- Atomic synchronization prevents type mismatch bugs
- Better test isolation with parallel execution

---

## Next Steps

1. **Approve** this optimization plan
2. **Create feature branch**: `git checkout -b feat/schematic-headers`
3. **Start Phase 1**: Validate paths (2 minutes)
4. **Proceed to Phase 2**: Add header fields atomically
5. **Follow revised timeline** (Phase 2-5 on fast path)
6. **Schedule Phase 1B** for after Phase 5 passes
7. **Document** changes in git commit messages

---

## Questions?

- **Can we truly parallelize Phase 3?** Yes, if you have 3 developers. Otherwise, sequential (20 min) is fine.
- **Why defer Phase 1?** It blocks everything but doesn't affect functionality. Deferring it eliminates the largest bottleneck.
- **What if Phase 2 sync fails?** Cargo will fail to compile in Phase 3 with type errors. Revert Phase 2 and try again with both fields in one commit.
- **Is parallel test execution safe?** Yes—all tests use isolated temporary directories (tempfile) and mocked HTTP (wiremock).

---

**Estimated Execution Time with Optimizations**: 52-67 minutes (including all deferred work)
**Current Estimate**: 90 minutes
**Time Saved**: 23-38 minutes (25-42% improvement)
