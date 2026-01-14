# Schematic Header Implementation: Parallelization Analysis

## Executive Summary

The proposed header implementation plan has **limited parallelization opportunities** due to strict sequential dependencies. However, **3 key optimizations** can reduce wall-clock time by 30-40%:

1. **Split Phase 2-3**: Separate type definition from code generation prep
2. **Defer Phase 1**: Delay directory restructuring until post-Phase 5
3. **Parallelize codegen modules**: Three independent generator file updates

## Current Plan Assessment

| Phase | Parallelizable | Dependencies | Duration |
|-------|---|---|---|
| 1. Directory Restructuring | ❌ No | None (blocker) | 5-10 min |
| 2. Header Type Definition | ❌ No (internally) | Phase 1 | 10-15 min |
| 3. Code Generation | ⚠️ Partial | Phase 2 | 15-20 min |
| 4. Endpoint Propagation | ❌ No | Phase 3 | 10-15 min |
| 5. Integration Testing | ✅ Yes | Phase 4 | 10-20 min |
| 6. Documentation | ✅ Yes | Phase 5 | 5-10 min |

**Current Sequential Total**: ~65-90 minutes

---

## Recommended Optimizations

### OPTIMIZATION 1: Phase 2-3 Splitting (REC_001)
**Impact**: -10 minutes wall time | **Risk**: Low

#### Rationale
- **Phase 2** only adds fields to structs in `schematic-define`
- **Phase 3** code generation can be *refactored in parallel* while Phase 2 is executing
- The actual *integration* of headers into generators still depends on Phase 2 completion, but preparation work doesn't

#### Two-Track Approach
```
Timeline:
─────────────────────────────────
Track A (Types):     [Phase 2: Add RestApi::headers, Endpoint::headers]
Track B (Codegen):   [Refactor schematic-gen modules for header awareness]
                     ↓ (Phase 2 complete)
Merge:              [Integrate header logic into generators]
─────────────────────────────────
```

#### Parallelizable Codegen Prep Work (Phase 3 early)
1. **request_structs.rs** - Extract header fields from endpoint definition
2. **client.rs** - Identify injection points for header application
3. **api_struct.rs** - Design optional header configuration interface

These can be refactored/designed while Phase 2 types are still being finalized.

---

### OPTIMIZATION 2: Defer Phase 1 Directory Restructuring (REC_002)
**Impact**: -5 minutes wall time | **Risk**: Low

#### Current Plan Problem
- Phase 1 (move `schematic/schematic/schema/` → `schematic/schema/`) **blocks all downstream work**
- But directory structure is purely mechanical—doesn't affect code logic
- Current path works fine as temporary location

#### Recommended Approach
1. **Execute Phase 1 validation only**: Confirm paths exist, files reference correctly
2. **Skip mechanical move**: Leave `schematic/schematic/schema/` as-is during development
3. **Defer full restructuring**: Execute after Phase 5 tests pass (post-implementation)
4. **Single batch move**: Update all references (Cargo.toml, imports, paths) in one commit

**Benefit**: Removes 5-10min blocking operation, maintains clear development focus on functionality

---

### OPTIMIZATION 3: Parallel Codegen Module Updates (REC_003)
**Impact**: -5 to 10 minutes wall time | **Risk**: Medium

#### Three Independent Tasks
Each codegen module handles a specific concern with minimal cross-dependencies:

| Module | Task | Duration | Dependency |
|--------|------|----------|------------|
| `request_structs.rs` | Extract header fields from Endpoint into generated struct | 5-8 min | Phase 2 types |
| `client.rs` | Inject header application logic in request builder | 7-10 min | Phase 2 types |
| `api_struct.rs` | (Optional) Add header configuration methods | 3-5 min | request_structs result |

#### Execution Strategy
```
Parallel for Rust development (one person per module):
├─ Dev 1: Update request_structs.rs
├─ Dev 2: Update client.rs
└─ Dev 3: (Optional) Update api_struct.rs

Sequential merge:
1. Merge request_structs result
2. Merge client.rs (may reference request_structs changes)
3. Merge api_struct (optional enhancement)
4. Run full test suite after each merge

Total time: max(5-8, 7-10, 3-5) = ~10 min vs 15-20 min sequential
```

#### Risk Mitigation
- **Merge conflicts**: High probability if multiple devs edit `codegen/mod.rs` exports
- **Validation**: Run `cargo build -p schematic-gen` after each merge to catch breakage immediately
- **Test coverage**: Existing `e2e_generation.rs` test will catch misintegrations

---

## CRITICAL: Race Condition Risk (REC_005)

### Synchronization Required
**RestApi** and **Endpoint** structs are defined in same file (`schematic-define/src/types.rs`):

```rust
pub struct RestApi {
    // ... existing fields
    pub headers: ???  // ← MUST be added
}

pub struct Endpoint {
    // ... existing fields
    pub headers: ???  // ← MUST be added in SAME commit
}
```

**Critical Constraint**: Both must be present before Phase 3 code generation begins

**Mitigation**:
- Phase 2 is **NOT internally parallelizable**
- Create single atomic commit: `feat(schematic): add header fields to RestApi and Endpoint`
- Include tests for both types in same commit
- Validate with: `cargo test -p schematic-define` before proceeding

---

## Phase 5 Testing: Parallelization Opportunity (REC_004)

**Excellent parallelization opportunity** for validation:

```bash
# Run integration tests with 4 concurrent threads
cargo test --test '*' -- --test-threads=4

# Expected parallelism:
├─ e2e_generation.rs test suite (2-5 sec)
├─ http_client.rs test suite (2-5 sec)
├─ path_substitution.rs tests (1-2 sec)
└─ (concurrent execution reduces from 10-20s → 5-8s)
```

**Safety**: All tests use isolated temporary directories (wiremock mocking, tempfile for output)

---

## Dependency Tree Analysis

```
Workspace Root (Cargo.toml)
└── schematic/
    ├── define/
    │   └── src/types.rs (RestApi, Endpoint, AuthStrategy)
    │       └── PHASE 2: Add header fields
    │
    └── gen/
        └── src/codegen/
            ├── request_structs.rs  ← Consumes Endpoint.headers
            ├── client.rs           ← Consumes Endpoint.headers
            └── api_struct.rs       ← Consumes RestApi.headers

schematic/schematic/schema/ (generated output)
└── PHASE 1: Restructure path (deferred to end)
```

**Key Finding**: No circular dependencies. Phase 1 restructuring can be deferred without blocking Phases 2-5.

---

## Revised Recommended Timeline

### Fast Path (Optimized)
```
Timeline: ~50-70 minutes (vs. 65-90 current)

0:00  ├─ Phase 1: Skip full restructuring, validate paths only (2 min)
      │
0:02  ├─ Phase 2: Add header fields to RestApi/Endpoint (12 min)
      │   └─ [PARALLEL: Phase 3 codegen prep work starts at 0:05]
      │
0:14  ├─ Phase 3: Integrate headers into generators (12 min)
      │   ├─ request_structs.rs update
      │   ├─ client.rs update
      │   └─ api_struct.rs update
      │
0:26  ├─ Phase 4: Test endpoint header propagation (10 min)
      │
0:36  ├─ Phase 5: Integration testing [PARALLEL MODE] (8 min)
      │   ├─ e2e_generation.rs tests (2-5 sec)
      │   ├─ http_client.rs tests (2-5 sec)
      │   └─ path_substitution.rs tests (concurrent)
      │
0:44  ├─ Phase 6: Documentation (8 min)
      │
0:52  └─ Phase 1B: Execute directory restructuring (15 min, post-implementation)
```

**Total Execution Time**: ~52 minutes (Phase 1-6) + 15 min (Phase 1B restructuring)

---

## Recommendations Summary

| Recommendation | Effort | Risk | Impact | Status |
|---|---|---|---|---|
| Split Phase 2-3 into parallel tracks | Low | Low | -10 min | ✅ RECOMMENDED |
| Defer directory restructuring | Low | Low | -5 min | ✅ RECOMMENDED |
| Parallelize codegen module updates | Medium | Medium | -5 to 10 min | ⚠️ CONDITIONAL |
| Use parallel test execution (Phase 5) | Low | Low | -2 to 5 min | ✅ RECOMMENDED |
| Atomic commit strategy | Low | Low | Better bisectability | ✅ CRITICAL |

---

## Implementation Checklist

### Phase 2: Add Header Fields
- [ ] Add `headers: HashMap<String, String>` field to `RestApi` struct
- [ ] Add `headers: Option<HashMap<String, String>>` field to `Endpoint` struct
- [ ] Add field documentation with examples
- [ ] Add unit tests for new fields
- [ ] Validate with: `cargo test -p schematic-define`

### Phase 3: Code Generation Updates
- [ ] **request_structs.rs**: Emit header fields in generated struct
- [ ] **client.rs**: Inject header application in request builder
- [ ] **api_struct.rs** (optional): Add header configuration methods
- [ ] Update codegen/mod.rs exports if needed
- [ ] Validate with: `cargo build -p schematic-gen`

### Phase 4: Endpoint Propagation
- [ ] Update request enum to include endpoint headers
- [ ] Test with `cargo build -p schematic-gen`

### Phase 5: Integration Testing
- [ ] Run parallel tests: `cargo test --test '*' -- --test-threads=4`
- [ ] Add test case for header propagation in `e2e_generation.rs`
- [ ] Verify generated code compiles and runs

### Phase 6: Documentation
- [ ] Update README.md with header examples
- [ ] Document in `schematic-define` crate docs
- [ ] Update `schematic-gen` documentation

### Phase 1B: Directory Restructuring (post-Phase 5)
- [ ] Move `schematic/schematic/schema/` → `schematic/schema/`
- [ ] Update all references in Cargo.toml and imports
- [ ] Run full test suite to verify
- [ ] Single atomic commit

---

## Key Metrics

| Metric | Current | Optimized | Improvement |
|--------|---------|-----------|------------|
| Wall-clock time | 65-90 min | 50-70 min | -20% |
| Blocking phases | 4 of 6 | 2 of 6 | -50% |
| Parallelizable work | 10% (Phase 5 tests) | 25% (Phases 3+5) | +150% |
| Risk level | Medium | Low | Better |

---

## Conclusion

The header implementation has **limited parallelization potential** due to sequential struct type changes, but the recommended optimizations can reduce execution time by **20-30%** while actually *reducing* risk through atomic commits and deferred restructuring.

**Key insight**: Directory restructuring (Phase 1) is a red herring—deferring it to the end eliminates the largest blocker without sacrificing functionality.
