"""Render Cloud Spanner query plans via the spannerplan-ffi C ABI."""

from __future__ import annotations

import ctypes
import os
import sys
from pathlib import Path
from typing import Any, Mapping, Optional

__all__ = [
    "RenderConfig",
    "RenderError",
    "render_tree_table_json",
    "render_tree_table_wire",
]

_LIB: Optional[ctypes.CDLL] = None


class RenderError(Exception):
    """Raised when the native renderer returns an error string."""


class RenderConfig(dict[str, Any]):
    """Render options (serialized to JSON for the FFI config_json argument)."""


def _default_lib_name() -> str:
    if sys.platform == "darwin":
        return "libspannerplan_ffi.dylib"
    if sys.platform.startswith("linux"):
        return "libspannerplan_ffi.so"
    if sys.platform == "win32":
        return "spannerplan_ffi.dll"
    raise OSError(f"unsupported platform: {sys.platform}")


def _ci_artifact_dir() -> str | None:
    if sys.platform == "darwin":
        if os.uname().machine in {"arm64", "aarch64"}:
            return "spannerplan-ffi-macos-arm64"
        return "spannerplan-ffi-macos-x64"
    if sys.platform.startswith("linux"):
        return "spannerplan-ffi-linux-x64"
    if sys.platform == "win32":
        return "spannerplan-ffi-windows-x64"
    return None


def _candidate_lib_paths() -> list[Path]:
    lib_name = _default_lib_name()
    package_dir = Path(__file__).resolve().parent
    repo_root = package_dir.parents[3]
    candidates: list[Path] = []

    ffi_dir = os.environ.get("SPANNERPLAN_FFI_DIR")
    if ffi_dir:
        candidates.append(Path(ffi_dir) / lib_name)

    # Wheel layout: bundled next to the package.
    candidates.extend(
        [
            package_dir / "lib" / lib_name,
            package_dir / lib_name,
        ]
    )

    # Monorepo cargo output.
    for profile in ("debug", "release"):
        candidates.append(repo_root / "target" / profile / lib_name)

    # GitHub Actions artifact download layout at repo root.
    artifact_dir = _ci_artifact_dir()
    if artifact_dir:
        candidates.append(repo_root / "artifacts" / artifact_dir / lib_name)

    return candidates


def _resolve_lib_path() -> Path:
    env = os.environ.get("SPANNERPLAN_FFI_LIB")
    if env:
        path = Path(env)
        if path.is_file():
            return path
        raise FileNotFoundError(f"SPANNERPLAN_FFI_LIB not found: {path}")

    for candidate in _candidate_lib_paths():
        if candidate.is_file():
            return candidate

    raise FileNotFoundError(
        "spannerplan native library not found; set SPANNERPLAN_FFI_LIB, "
        "SPANNERPLAN_FFI_DIR, or run `cargo build -p spannerplan-ffi` from the repo root"
    )


def _lib() -> ctypes.CDLL:
    global _LIB
    if _LIB is None:
        lib = ctypes.CDLL(str(_resolve_lib_path()))
        lib.spannerplan_render_tree_table_json.argtypes = [
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_int),
        ]
        lib.spannerplan_render_tree_table_json.restype = ctypes.c_void_p
        lib.spannerplan_render_tree_table_wire.argtypes = [
            ctypes.c_void_p,
            ctypes.c_size_t,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_int),
        ]
        lib.spannerplan_render_tree_table_wire.restype = ctypes.c_void_p
        lib.spannerplan_string_free.argtypes = [ctypes.c_void_p]
        lib.spannerplan_string_free.restype = None
        _LIB = lib
    return _LIB


def _config_json(config: Optional[Mapping[str, Any]]) -> Optional[bytes]:
    if not config:
        return None
    import json

    return json.dumps(dict(config)).encode("utf-8")


def _call_render(fn, *args: Any) -> str:
    is_error = ctypes.c_int(0)
    out = fn(*args, ctypes.byref(is_error))
    if not out:
        raise RenderError("native render returned NULL")
    try:
        text = ctypes.string_at(out).decode("utf-8")
    finally:
        _lib().spannerplan_string_free(out)
    if is_error.value != 0:
        raise RenderError(text)
    return text


def render_tree_table_json(
    plan_json: str | bytes,
    mode: str = "AUTO",
    format: str = "CURRENT",
    config: Optional[Mapping[str, Any]] = None,
) -> str:
    """Render from JSON/YAML text (QueryPlan, ResultSetStats, or ResultSet shapes)."""
    if isinstance(plan_json, str):
        plan_bytes = plan_json.encode("utf-8")
    else:
        plan_bytes = plan_json
    return _call_render(
        _lib().spannerplan_render_tree_table_json,
        plan_bytes,
        mode.encode("utf-8"),
        format.encode("utf-8"),
        _config_json(config),
    )


def render_tree_table_wire(
    plan_wire: bytes,
    mode: str = "AUTO",
    format: str = "CURRENT",
    config: Optional[Mapping[str, Any]] = None,
) -> str:
    """Render from protobuf wire-encoded plan bytes."""
    buf = (ctypes.c_ubyte * len(plan_wire)).from_buffer_copy(plan_wire)
    return _call_render(
        _lib().spannerplan_render_tree_table_wire,
        buf,
        len(plan_wire),
        mode.encode("utf-8"),
        format.encode("utf-8"),
        _config_json(config),
    )
