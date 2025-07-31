# Refactoring Plan – iOS & macOS Minidump Writers

*Revision range analysed: `d0c91bd7` (upstream) → `9801c848` (ios-support feature)*

---

## 0. Guiding Principles

1. **Backward compatibility is non-negotiable**.  
   • Public API (module paths, type & fn names, builder methods) **must stay byte-for-byte the same**.  
   • The existing **macOS codebase is considered the "source of truth"** – aside from trivial `use`/`pub use` additions, **we do not edit its logic or data-flow**.  
   • Binary output of the macOS writer remains identical (snapshot-tested).
2. **Small, review-friendly PRs** (≤ ~800 diff lines) – pure moves first, behaviour changes later.
3. **No directory churn**. Current layout (`src/apple/{ios,mac}` + `src/apple/common`) is kept so that `git blame` & history remain intact.
4. **Conditional-compile pattern remains the core** (Linux/Android와 동일): `mod ios` / `mod mac` behind `#[cfg]` gates.  
   – **공통 로직**은 ① helper 함수, ② 작은 `macro_rules!`, ③ **내부(private) trait** 가운데 _가장 단순한 방법_ 으로 공유한다.  
   – 단, `GenericWriter<Platform>` 처럼 **전체 구조체를 제네릭화**해서 외부 호출자가 type parameter 를 보게 만드는 방식은 사용하지 않는다.
5. **iOS signal safety constraints are paramount**.  
   – Common code **must be signal-safe** for iOS compatibility.  
   – No dynamic allocations, no I/O operations in shared code paths.

---

## 1. Current Duplication Snapshot

| Area                      | Similarity | Notes |
|---------------------------|-----------:|-------|
| streams/breakpad_info.rs  | 73 % | Field-for-field identical |
| streams/misc_info.rs      | 79 % | identical except enum variants |
| streams/thread_names.rs   | 52 % | format loop identical, CPU field differs |
| minidump_writer.rs        | 39 % | directory bookkeeping the same |
| task_dumper.rs            | 29 % | port/thread enumeration similar |

Estimated **~1 400 duplicated LoC**.

---

## 2. iOS-Specific Constraints

### Signal Safety Requirements
iOS implementation operates under strict signal-safe constraints:
- **No dynamic allocation**: malloc, Box::new, Vec::new 등 사용 불가
- **No I/O operations**: 파일 시스템 접근 제한 (signal handler 내)
- **Pre-allocated buffers only**: 모든 메모리는 사전 할당 필요
- **No Objective-C/Swift runtime**: 시그널 핸들러 내에서 사용 불가
- **Async-signal-safe functions only**: POSIX 표준 준수

### Implications for Common Code
공통 코드 추출 시 다음 기준 적용:
1. **Allocation-free code only**: 동적 할당이 없는 코드만 공통화 가능
2. **Pure computation**: I/O 없는 순수 연산 코드 우선
3. **Static data structures**: 컴파일 타임에 크기가 결정되는 구조체만 사용
4. **Platform API abstraction**: OS 특화 API는 trait으로 추상화, 구현은 각 플랫폼에서

### Common Code Extraction Criteria
코드를 common으로 추출하기 위한 명확한 기준:
1. **Structural Identity (100%)**: 구조체 필드가 완전히 동일
2. **Logic Identity (95%+)**: 동일한 알고리즘, OS API 호출만 차이
3. **No iOS Constraints Violation**: signal-safe 제약에 위배되지 않음
4. **Platform API Abstraction Possible**: OS 특화 API를 trait으로 추상화 가능

예시:
- ✅ `breakpad_info`: 필드 구조 동일, 동적 할당 없음
- ✅ `misc_info`: enum variant만 차이, 로직 동일
- ⚠️ `thread_names`: 조건부 가능 (iOS는 사전 할당 버퍼 필요)
- ❌ `module_list`: iOS는 사전 할당 버퍼 사용, macOS는 Vec 사용

---

## 3. Target Architecture

```
src/apple/
  common/
    mach.rs              # existing – low-level Mach helpers
    streams/             # NEW – shared stream writers moved from mac/
      breakpad_info.rs   # 100% identical, no allocations
      misc_info.rs       # 95% identical, #[cfg] for CPU freq
    types.rs             # Shared type definitions and traits (existing)

  ios/                   # iOS-specific code only – imports from common::*
    streams/
      thread_list.rs     # iOS-specific due to pre-allocated buffers
      module_list.rs     # iOS-specific due to static arrays
      memory_list.rs     # iOS-specific memory handling
      exception.rs       # iOS crash context integration
      system_info.rs     # iOS sysctl differences
    minidump_writer.rs   # iOS writer with signal-safe constraints
    task_dumper.rs       # iOS task/thread enumeration

  mac/                   # macOS code with minimal changes
    streams/
      thread_list.rs     # Stays - uses Vec allocations
      module_list.rs     # Stays - dynamic allocations
      memory_list.rs     # Stays - different memory model
      exception.rs       # Stays - different exception handling
      system_info.rs     # Stays - different sysctl access
    minidump_writer.rs   # Re-exports common, keeps mac-specific
    task_dumper.rs       # Stays - different task access model
```

---

## 4. Detailed Work Breakdown

### Pre-Refactoring: Function Name Alignment
**PR: "refactor: align iOS/macOS function names"**
Before extracting common code, align function naming conventions:
1. **TaskDumper methods**:
   - iOS: Remove redundant `pid()`, use only `pid_for_task()` like macOS
   - Both: Ensure identical method signatures

2. **Stream writers**:
   - iOS: Convert standalone functions to MinidumpWriter methods (match macOS pattern)
   - Example: `system_info::write_system_info()` → `MinidumpWriter::write_system_info()`

3. **Directory operations**:
   - Align `write_to_file()` signatures between platforms
   
- **Files affected**: 
  - `ios/task_dumper.rs` (remove `pid()`)
  - `ios/minidump_writer.rs` (convert to methods)
  - `ios/streams/*.rs` (adjust to method pattern)
- **Guarantees**: Functional equivalence, no behavior change

### Phase 1: Extract Buffer Management 
**PR: "refactor: consolidate DumpBuf and directory utilities"**
- Move `DumpBuf` from `mac/minidump_writer.rs` to `common/minidump_writer/dump_buf.rs`
- Extract directory bookkeeping utilities to `common/minidump_writer/directory.rs`
- macOS: Change imports to use `common::minidump_writer::{DumpBuf, ...}`
- iOS: Remove duplicate implementations, import from common
- **Files affected**: 
  - `mac/minidump_writer.rs` (−150 lines)
  - `ios/minidump_writer.rs` (−150 lines)
  - `common/minidump_writer/dump_buf.rs` (+150 lines)
- **Guarantees**: Binary-identical, no allocations

### Phase 2: Consolidate breakpad_info Stream
**PR: "refactor: move breakpad_info to common"**
- Copy `mac/streams/breakpad_info.rs` → `common/streams/breakpad_info.rs`
- Update imports and error handling to be platform-neutral
- macOS: `pub use crate::apple::common::streams::breakpad_info;`
- iOS: Remove implementation, `pub use` common version
- **Files affected**:
  - `mac/streams/breakpad_info.rs` (→ re-export only)
  - `ios/streams/breakpad_info.rs` (delete)
  - `common/streams/breakpad_info.rs` (+35 lines)
- **Guarantees**: 100% identical logic, signal-safe

### Phase 3: Consolidate misc_info Stream
**PR: "refactor: move misc_info to common with platform conditionals"**
- Copy `mac/streams/misc_info.rs` → `common/streams/misc_info.rs`
- Add `#[cfg]` for CPU frequency code (macOS only)
- Extract shared structs: `TimeValue`, `MachTaskBasicInfo`, `TaskThreadsTimeInfo`
- Both platforms: `pub use` common version
- **Files affected**:
  - `mac/streams/misc_info.rs` (→ re-export only)
  - `ios/streams/misc_info.rs` (delete)
  - `common/streams/misc_info.rs` (+180 lines)
- **Guarantees**: Binary-identical, allocation-free

### Phase 4: ~~Extract thread_names Core Logic~~ [SKIPPED]
**Decision**: thread_names has significant platform differences:
- macOS: Uses custom `ActiveThreads` iterator with handler thread filtering
- iOS: Direct thread list filtering with different thread enumeration
- Thread name retrieval: macOS uses `proc_threadinfo`, iOS returns empty names

The differences are too substantial for meaningful code sharing. Both implementations
remain platform-specific.

### Phase 5: Type Definitions & Constants
**PR: "refactor: consolidate shared type definitions"**
- Move common error types to `common/types.rs`
- Extract shared constants and helper functions
- Consolidate CPU-related type mappings
- **Files affected**:
  - Various files for import updates
  - `common/types.rs` (+200 lines)
- **Guarantees**: Type-compatible, no behavior change

### Phase 6: Documentation & Cleanup
**PR: "docs: update architecture for iOS/macOS consolidation"**
- Update ARCHITECTURE.md with new structure
- Add migration guide for contributors
- Remove any dead code
- Update module documentation
- **Files affected**: Documentation only
- **Guarantees**: No code changes

### Summary Statistics
- **Total PR count**: 6 (each ≤800 lines)
- **Code reduction**: ~215 LoC eliminated (breakpad_info + misc_info)
- **Common code created**: ~250 LoC
- **Platform-specific retained**: Most stream implementations
- **Risk level**: Low (incremental, tested at each step)

### Notes on Signal Safety Testing
While signal safety tests (`tests/signal_safety.rs`) are critical for iOS, they will be added in a future phase when proper testing infrastructure is available. For now, manual verification and code review will ensure signal safety compliance.
