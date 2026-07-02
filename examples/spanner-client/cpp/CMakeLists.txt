cmake_minimum_required(VERSION 3.22)
project(spanner_client_cpp_example LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(SPANNERPLAN_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../..")
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
endif()

if(NOT SPANNERPLAN_FFI_LIB OR NOT EXISTS "${SPANNERPLAN_FFI_LIB}")
  message(FATAL_ERROR
    "spannerplan native library not found; run `cargo build -p spannerplan-ffi` "
    "from the repo root or set SPANNERPLAN_FFI_LIB / SPANNERPLAN_FFI_DIR")
endif()

find_package(google_cloud_cpp_spanner REQUIRED)

add_executable(analyze_and_render analyze_and_render.cpp cli_options.cpp)
target_compile_definitions(analyze_and_render PRIVATE
  SPANNER_CLIENT_QUERY_FILE_DEFAULT="${CMAKE_CURRENT_SOURCE_DIR}/../query.sql")
target_include_directories(analyze_and_render PRIVATE "${SPANNERPLAN_FFI_INCLUDE}")
target_link_libraries(analyze_and_render PRIVATE google-cloud-cpp::spanner "${SPANNERPLAN_FFI_LIB}")
