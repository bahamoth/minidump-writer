# 상세 비교 분석: src/mac/ vs src/apple/mac/

이 문서는 원저작자의 `src/mac/` 디렉토리와 리팩토링된 `src/apple/mac/` 디렉토리 간의 모든 차이점을 상세히 기록합니다.

## 1. 파일 구조 변경사항

### 1.1 파일 목록 비교

**src/mac/ 파일 목록:**
```
src/mac/errors.rs
src/mac/mach.rs              <- 삭제됨 (src/apple/common/mach.rs로 이동)
src/mac/minidump_writer.rs
src/mac/streams.rs
src/mac/streams/breakpad_info.rs
src/mac/streams/exception.rs
src/mac/streams/memory_list.rs
src/mac/streams/misc_info.rs
src/mac/streams/module_list.rs
src/mac/streams/system_info.rs
src/mac/streams/thread_list.rs
src/mac/streams/thread_names.rs
src/mac/task_dumper.rs
```

**src/apple/mac/ 파일 목록:**
```
src/apple/mac/errors.rs
src/apple/mac/minidump_writer.rs
src/apple/mac/mod.rs          <- 새로 추가됨
src/apple/mac/streams.rs
src/apple/mac/streams/breakpad_info.rs
src/apple/mac/streams/exception.rs
src/apple/mac/streams/memory_list.rs
src/apple/mac/streams/misc_info.rs
src/apple/mac/streams/module_list.rs
src/apple/mac/streams/system_info.rs
src/apple/mac/streams/thread_list.rs
src/apple/mac/streams/thread_names.rs
src/apple/mac/task_dumper.rs
```

### 1.2 주요 구조적 변경

1. **mach.rs 파일 이동**
   - 원본: `src/mac/mach.rs` (672줄)
   - 이동: `src/apple/common/mach.rs`
   - mod.rs에서 재수출: `pub mod mach { pub use crate::apple::common::mach::*; }`

2. **mod.rs 파일 추가**
   - 위치: `src/apple/mac/mod.rs`
   - 내용: 모듈 재수출 및 조직화

## 2. 파일별 상세 차이점

### 2.1 errors.rs

**위치**: 3번째 줄
```rust
// 원본 (src/mac/errors.rs:5)
TaskDumpError(#[from] crate::mac::task_dumper::TaskDumpError),

// 변경 (src/apple/mac/errors.rs:5)
TaskDumpError(#[from] crate::apple::common::TaskDumpError),
```
- TaskDumpError가 common 모듈로 이동됨에 따른 import 경로 변경

### 2.2 minidump_writer.rs

**위치**: 1-3번째 줄
```rust
// 원본 (src/mac/minidump_writer.rs:1-3)
use crate::{
    dir_section::{DirSection, DumpBuf},
    mac::{errors::WriterError, task_dumper::TaskDumper},

// 변경 (src/apple/mac/minidump_writer.rs:1-3)
use crate::{
    apple::mac::{errors::WriterError, task_dumper::TaskDumper},
    dir_section::{DirSection, DumpBuf},
```
- import 경로 변경: `mac::` → `apple::mac::`

### 2.3 task_dumper.rs - 대규모 변경

**전체 구조 변경**:
- 원본: 462줄의 전체 구현
- 변경: 268줄로 축소, 대부분 TaskDumperBase로 위임

**주요 변경사항**:

1. **구조체 정의 변경** (줄 8-12 → 줄 8-12)
```rust
// 원본
pub struct TaskDumper {
    task: mt::task_t,
    page_size: i64,
}

// 변경
pub struct TaskDumper {
    base: TaskDumperBase,
}
```

2. **새로운 메서드 추가**:

   a. **task() getter 추가** (줄 23-25)
   ```rust
   pub fn task(&self) -> mt::task_t {
       self.base.task
   }
   ```

   b. **pid() 메서드 추가** (줄 63-65)
   ```rust
   pub fn pid(&self) -> Result<i32, TaskDumpError> {
       self.pid_for_task()
   }
   ```

   c. **read_vm_regions() 추가** (줄 129-176)
   ```rust
   pub fn read_vm_regions(&self) -> Result<Vec<VMRegionInfo>, TaskDumpError> {
       let mut regions = Vec::new();
       let mut region_base = 0;
       let mut region_size = 0;
       // ... 전체 VM 영역을 순회하며 수집하는 새로운 기능
   }
   ```

3. **기존 메서드들의 위임 패턴 변경**:
   - read_task_memory: `self.base.read_task_memory(address, count)`로 위임
   - read_string: `self.base.read_string(address, max_length)`로 위임
   - task_info: `self.base.task_info()`로 위임

### 2.4 streams/breakpad_info.rs - 완전 재구현

**원본**: 34줄의 직접 구현
**변경**: 28줄로 축소, trait 구현 및 위임 패턴 사용

**전체 내용 변경**:
```rust
// 원본 (줄 14-31)
let bp_section = MemoryWriter::<BreakpadInfo>::alloc_with_val(
    buffer,
    BreakpadInfo {
        validity: BreakpadInfoValid::DumpThreadId.bits()
            | BreakpadInfoValid::RequestingThreadId.bits(),
        dump_thread_id: self.handler_thread,
        requesting_thread_id: self.crash_context.as_ref().map(|cc| cc.thread).unwrap_or(0),
    },
)?;

// 변경 (줄 4-11, 26)
impl BreakpadInfoWriter for MinidumpWriter {
    fn handler_thread(&self) -> u32 {
        self.handler_thread
    }
    fn requesting_thread(&self) -> u32 {
        self.crash_context.as_ref().map(|cc| cc.thread).unwrap_or(0)
    }
}
// ...
breakpad_info::write_breakpad_info(self, buffer).map_err(WriterError::MemoryWriterError)
```

### 2.5 streams/misc_info.rs - 완전 재구현

**원본**: 170줄의 직접 구현
**변경**: 29줄로 축소, trait 구현 및 위임 패턴 사용

**주요 변경**:
- 모든 시스템 정보 수집 로직이 common 모듈로 이동
- TaskDumperHelper trait 구현 추가 (줄 4-13)
- write_misc_info 함수 호출로 대체 (줄 28)

### 2.6 streams/module_list.rs - 중요한 기능 추가

**1. 현재 프로세스 감지 로직 추가** (줄 128)
```rust
// 추가된 코드
let is_current_process = dumper.task() == unsafe { mach::mach_task_self() };
```

**2. dyld API를 사용한 파일 경로 획득** (줄 174-192)
```rust
// 원본: image.file_path != 0 체크만 수행
// 변경: 현재 프로세스일 경우 dyld API 사용
let file_path = if is_current_process {
    let image_count = unsafe { _dyld_image_count() };
    let mut found_path = None;
    
    for i in 0..image_count {
        let header = unsafe { _dyld_get_image_header(i) };
        if header as u64 == image.load_address {
            let name_ptr = unsafe { _dyld_get_image_name(i) };
            if !name_ptr.is_null() {
                let c_str = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
                found_path = c_str.to_str().ok().map(String::from);
            }
            break;
        }
    }
    found_path
} else if image.file_path != 0 {
    // 기존 로직
}
```

**3. dyld API extern 선언 추가** (줄 342-348)
```rust
#[allow(non_snake_case)]
extern "C" {
    fn _dyld_image_count() -> u32;
    fn _dyld_get_image_name(image_index: u32) -> *const libc::c_char;
    fn _dyld_get_image_header(image_index: u32) -> *const libc::c_void;
}
```

**4. 기타 작은 변경**:
- 줄 95: `.get(0)` → `.first()` (더 관용적인 Rust 코드)
- 줄 211: import 경로 변경

### 2.7 streams/memory_list.rs - 버그 수정

**위치**: 줄 28
```rust
// 원본
if ip < region.range.start || ip > region.range.end {

// 변경 (버그 수정)
if ip < region.range.start || ip >= region.range.end {
```
- 경계값 체크 로직 수정 (end는 exclusive이므로 >= 가 맞음)

### 2.8 streams/exception.rs - 코드 정리

**1. 불필요한 타입 캐스팅 제거** (줄 38, 42)
```rust
// 원본
let code = exc.code as u64;
if exc.kind as u32 == et::EXC_CRASH {

// 변경
let code = exc.code;
if exc.kind == et::EXC_CRASH {
```

**2. 코드 스타일 개선** (줄 172-181)
```rust
// 원본: .then() 사용
is_valid_exc_crash(code).then(|| WrappedException { ... })

// 변경: 명시적 if-else
if is_valid_exc_crash(code) {
    Some(WrappedException { ... })
} else {
    None
}
```

### 2.9 streams/thread_list.rs - 타이포 수정

**위치**: 줄 6 (주석)
```rust
// 원본
/// [`miniduimp_common::format::MINIDUMP_THREAD`]

// 변경
/// [`minidump_common::format::MINIDUMP_THREAD`]
```

**위치**: 줄 169 (함수 시그니처)
```rust
// 원본
thread_state: &crate::mac::mach::ThreadState,

// 변경  
thread_state: &mach::ThreadState,
```

### 2.10 streams/thread_names.rs - 타이포 수정

**위치**: 줄 45 (주석)
```rust
// 원본
/// Attempts to retrieve and write the threadname, returning the threa names

// 변경
/// Attempts to retrieve and write the thread name, returning the thread names
```

### 2.11 streams.rs - Import 경로 변경

**위치**: 줄 1-18
```rust
// 원본
use super::{
    errors::WriterError,
    mach,
    minidump_writer::MinidumpWriter,
    task_dumper::{self, ImageInfo, TaskDumpError, TaskDumper},
};

// 변경
// Stream writers for macOS minidump format (주석 추가)

use super::{
    errors::WriterError,
    minidump_writer::MinidumpWriter,
    task_dumper::{ImageInfo, TaskDumper},
};
use crate::apple::common::{mach, TaskDumpError};
```

### 2.12 새로 추가된 mod.rs

**전체 내용**:
```rust
// macOS-specific implementation

mod minidump_writer;
mod streams;
mod task_dumper;

pub mod errors;

// Re-export mach from common
pub mod mach {
    pub use crate::apple::common::mach::*;
}

// Re-export mach2 for backward compatibility
pub use mach2;

// Re-export public types
pub use minidump_writer::MinidumpWriter;
pub use task_dumper::TaskDumper;
```

## 3. 컴파일 및 동작 영향 분석

### 3.1 API 호환성
- **유지됨**: 모든 기존 public API는 동일한 경로에서 사용 가능
- **추가됨**: `TaskDumper::task()`, `TaskDumper::pid()`, `TaskDumper::read_vm_regions()`

### 3.2 동작 변경사항
1. **현재 프로세스 덤프 개선**: dyld API 사용으로 더 정확한 모듈 경로 획득
2. **메모리 영역 버그 수정**: 경계값 체크 로직 수정
3. **새로운 기능**: 전체 VM 영역 읽기 기능 추가

### 3.3 바이너리 호환성
- **ABI 변경**: TaskDumper 구조체 레이아웃 변경으로 바이너리 호환성 깨짐
- **기능적 호환성**: 대부분의 경우 동일한 결과 생성, 일부 개선사항 포함

## 4. 결론

원저작자의 코드 대부분이 보존되었으나, 다음과 같은 중요한 변경사항이 있습니다:

1. **구조적 리팩토링**: 공통 코드를 apple/common으로 분리
2. **기능 개선**: 현재 프로세스 처리 개선, VM 영역 전체 읽기 추가
3. **버그 수정**: 메모리 영역 경계 체크 수정
4. **코드 품질**: 타이포 수정, 불필요한 캐스팅 제거

이러한 변경사항들은 iOS 지원을 위한 준비 작업으로 보이며, 대부분 긍정적인 개선사항입니다.