#!/usr/bin/env bash
# Locate the vcpkg CMake toolchain file for google-cloud-cpp manifest builds.
# Prints the toolchain path on success; exits non-zero if not found.
set -euo pipefail

if [[ -n "${CMAKE_TOOLCHAIN_FILE:-}" && -f "${CMAKE_TOOLCHAIN_FILE}" ]]; then
  printf '%s\n' "${CMAKE_TOOLCHAIN_FILE}"
  exit 0
fi

candidates=()
if [[ -n "${VCPKG_ROOT:-}" ]]; then
  candidates+=("${VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake")
fi
candidates+=(
  "${HOME}/vcpkg/scripts/buildsystems/vcpkg.cmake"
  "/opt/homebrew/opt/vcpkg/scripts/buildsystems/vcpkg.cmake"
  "/opt/homebrew/share/vcpkg/scripts/buildsystems/vcpkg.cmake"
  "/usr/local/opt/vcpkg/scripts/buildsystems/vcpkg.cmake"
  "/usr/local/share/vcpkg/scripts/buildsystems/vcpkg.cmake"
)

for candidate in "${candidates[@]}"; do
  if [[ -f "${candidate}" ]]; then
    printf '%s\n' "${candidate}"
    exit 0
  fi
done

if command -v vcpkg >/dev/null 2>&1; then
  vcpkg_bin="$(command -v vcpkg)"
  vcpkg_root="$(cd "$(dirname "${vcpkg_bin}")/.." && pwd)"
  toolchain="${vcpkg_root}/scripts/buildsystems/vcpkg.cmake"
  if [[ -f "${toolchain}" ]]; then
    printf '%s\n' "${toolchain}"
    exit 0
  fi
fi

exit 1
