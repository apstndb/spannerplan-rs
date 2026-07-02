cmake_minimum_required(VERSION 3.16)
project(spannerplan_cpp_example LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(SPANNERPLAN_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../..")
set(SPANNERPLAN_FFI_INCLUDE "${SPANNERPLAN_ROOT}/crates/spannerplan-ffi")

if(APPLE)
  set(SPANNERPLAN_FFI_LIB_NAME "libspannerplan_ffi.dylib")
elseif(UNIX)
  set(SPANNERPLAN_FFI_LIB_NAME "libspannerplan_ffi.so")
elseif(WIN32)
  set(SPANNERPLAN_FFI_LIB_NAME "spannerplan_ffi.dll")
endif()

if(DEFINED ENV{SPANNERPLAN_FFI_LIB} AND EXISTS "$ENV{SPANNERPLAN_FFI_LIB}")
  set(SPANNERPLAN_FFI_LIB "$ENV{SPANNERPLAN_FFI_LIB}")
elseif(DEFINED ENV{SPANNERPLAN_FFI_DIR} AND EXISTS "$ENV{SPANNERPLAN_FFI_DIR}/${SPANNERPLAN_FFI_LIB_NAME}")
  set(SPANNERPLAN_FFI_LIB "$ENV{SPANNERPLAN_FFI_DIR}/${SPANNERPLAN_FFI_LIB_NAME}")
elseif(EXISTS "${SPANNERPLAN_ROOT}/target/debug/${SPANNERPLAN_FFI_LIB_NAME}")
  set(SPANNERPLAN_FFI_LIB "${SPANNERPLAN_ROOT}/target/debug/${SPANNERPLAN_FFI_LIB_NAME}")
elseif(EXISTS "${SPANNERPLAN_ROOT}/target/release/${SPANNERPLAN_FFI_LIB_NAME}")
  set(SPANNERPLAN_FFI_LIB "${SPANNERPLAN_ROOT}/target/release/${SPANNERPLAN_FFI_LIB_NAME}")
elseif(EXISTS "${SPANNERPLAN_ROOT}/artifacts/spannerplan-ffi-linux-x64/${SPANNERPLAN_FFI_LIB_NAME}")
  set(SPANNERPLAN_FFI_LIB "${SPANNERPLAN_ROOT}/artifacts/spannerplan-ffi-linux-x64/${SPANNERPLAN_FFI_LIB_NAME}")
elseif(EXISTS "${SPANNERPLAN_ROOT}/artifacts/spannerplan-ffi-macos-arm64/${SPANNERPLAN_FFI_LIB_NAME}")
  set(SPANNERPLAN_FFI_LIB "${SPANNERPLAN_ROOT}/artifacts/spannerplan-ffi-macos-arm64/${SPANNERPLAN_FFI_LIB_NAME}")
elseif(EXISTS "${SPANNERPLAN_ROOT}/artifacts/spannerplan-ffi-macos-x64/${SPANNERPLAN_FFI_LIB_NAME}")
  set(SPANNERPLAN_FFI_LIB "${SPANNERPLAN_ROOT}/artifacts/spannerplan-ffi-macos-x64/${SPANNERPLAN_FFI_LIB_NAME}")
elseif(EXISTS "${SPANNERPLAN_ROOT}/artifacts/spannerplan-ffi-windows-x64/${SPANNERPLAN_FFI_LIB_NAME}")
  set(SPANNERPLAN_FFI_LIB "${SPANNERPLAN_ROOT}/artifacts/spannerplan-ffi-windows-x64/${SPANNERPLAN_FFI_LIB_NAME}")
endif()

if(NOT SPANNERPLAN_FFI_LIB OR NOT EXISTS "${SPANNERPLAN_FFI_LIB}")
  message(FATAL_ERROR
    "spannerplan native library not found; run `cargo build -p spannerplan-ffi` "
    "from the repo root or set SPANNERPLAN_FFI_LIB / SPANNERPLAN_FFI_DIR")
endif()

function(spannerplan_link_ffi target)
  target_include_directories(${target} PRIVATE "${SPANNERPLAN_FFI_INCLUDE}")
  target_link_libraries(${target} PRIVATE "${SPANNERPLAN_FFI_LIB}")
endfunction()

add_executable(rendertree rendertree.cpp)
spannerplan_link_ffi(rendertree)

option(SPANNERPLAN_BUILD_RENDER_EXAMPLE
  "Build the optional render_example sample" ON)
if(SPANNERPLAN_BUILD_RENDER_EXAMPLE)
  add_executable(render_example main.cpp)
  spannerplan_link_ffi(render_example)
endif()

enable_testing()

add_executable(test_golden test_golden.cpp)
spannerplan_link_ffi(test_golden)
add_test(NAME golden_dca_plan_current COMMAND test_golden)
