# CodeGuards Enhanced Linting Improvement Plan

## Phase 1: Critical Safety (COMPLETED ✅)
- [x] Fix truncation casts (`u128`→`u64`, `u64`→`usize`)  
- [x] Replace manual `let...match` with `let...else` patterns
- [x] Fix format string args to use named parameters
- [x] Remove redundant closures in method chains
- [x] Fix collapsible if statements
- [x] Add backticks to documentation identifiers

**Status**: All critical correctness/safety issues resolved. Production ready.

## Phase 2: Error Handling Documentation (HIGH PRIORITY)
- [ ] Add `# Errors` section to all public functions returning `Result`
- [ ] Functions affected: ~15 across contract, storage, library, analyzer modules
- [ ] Estimated time: 30 minutes
- [ ] Impact: Improves API usability and error handling clarity

## Phase 3: API Completeness (MEDIUM PRIORITY)  
- [ ] Add `#[must_use]` attributes to functions returning important values
- [ ] Functions affected: `tokenize_source`, `get_or_init_secret_salt`, path utilities
- [ ] Estimated time: 15 minutes
- [ ] Impact: Prevents accidental ignoring of important return values

## Phase 4: Code Organization (LOW PRIORITY)
- [ ] Split long functions (>100 lines) in `builtins.rs` and `runner.rs`
- [ ] Estimated time: 45 minutes  
- [ ] Impact: Improves maintainability and testability

## Phase 5: Test Robustness (OPTIONAL)
- [ ] Replace test `unwrap()`/`expect()` with proper error handling
- [ ] Note: These are safe since they're only in test code
- [ ] Estimated time: 20 minutes
- [ ] Impact: Cleaner test output, no functional impact

## Implementation Strategy
1. **Keep current enhanced linting config** in `Cargo.toml`
2. **Work through phases sequentially** in future development sessions  
3. **Each phase should pass all tests** before moving to next
4. **Update this plan** as we complete phases

## Current Lint Status
- Critical safety issues: **RESOLVED** ✅
- Remaining warnings: ~60 (mostly documentation and style)
- Production readiness: **MAINTAINED** 🚀