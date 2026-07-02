package io.spannerplan;

import com.sun.jna.Library;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

/** JNA mapping of the spannerplan-ffi C ABI. */
interface SpannerplanNative extends Library {
  SpannerplanNative INSTANCE = load();

  Pointer spannerplan_render_tree_table_json(
      String planJson,
      String mode,
      String format,
      String configJson,
      IntByReference outIsError);

  Pointer spannerplan_render_tree_table_wire(
      byte[] planWire,
      long planWireLen,
      String mode,
      String format,
      String configJson,
      IntByReference outIsError);

  void spannerplan_string_free(Pointer s);

  static SpannerplanNative load() {
    String env = System.getenv("SPANNERPLAN_FFI_LIB");
    if (env != null && !env.isBlank()) {
      Path path = Paths.get(env);
      if (!Files.isRegularFile(path)) {
        throw new IllegalStateException("SPANNERPLAN_FFI_LIB not found: " + env);
      }
      return Native.load(path.toAbsolutePath().toString(), SpannerplanNative.class);
    }

    String libName = defaultLibName();

    String ffiDir = System.getenv("SPANNERPLAN_FFI_DIR");
    if (ffiDir != null && !ffiDir.isBlank()) {
      Path dirLib = Paths.get(ffiDir).resolve(libName);
      if (Files.isRegularFile(dirLib)) {
        return Native.load(dirLib.toAbsolutePath().toString(), SpannerplanNative.class);
      }
    }

    Path repoRoot = Paths.get("").toAbsolutePath().getParent().getParent();
    if (!Files.isDirectory(repoRoot.resolve("testdata"))) {
      repoRoot = Paths.get(System.getProperty("user.dir")).getParent().getParent();
    }

    for (String profile : new String[] {"debug", "release"}) {
      Path candidate = repoRoot.resolve("target").resolve(profile).resolve(libName);
      if (Files.isRegularFile(candidate)) {
        return Native.load(candidate.toAbsolutePath().toString(), SpannerplanNative.class);
      }
    }

    String artifactDir = ciArtifactDir();
    if (artifactDir != null) {
      Path artifactLib = repoRoot.resolve("artifacts").resolve(artifactDir).resolve(libName);
      if (Files.isRegularFile(artifactLib)) {
        return Native.load(artifactLib.toAbsolutePath().toString(), SpannerplanNative.class);
      }
    }

    throw new IllegalStateException(
        "spannerplan native library not found; set SPANNERPLAN_FFI_LIB, SPANNERPLAN_FFI_DIR, or "
            + "run `cargo build -p spannerplan-ffi` from the repo root");
  }

  private static String ciArtifactDir() {
    String os = System.getProperty("os.name", "").toLowerCase();
    if (os.contains("mac")) {
      String arch = System.getProperty("os.arch", "").toLowerCase();
      return arch.contains("aarch64") || arch.contains("arm64")
          ? "spannerplan-ffi-macos-arm64"
          : "spannerplan-ffi-macos-x64";
    }
    if (os.contains("linux")) {
      return "spannerplan-ffi-linux-x64";
    }
    if (os.contains("win")) {
      return "spannerplan-ffi-windows-x64";
    }
    return null;
  }

  private static String defaultLibName() {
    String os = System.getProperty("os.name", "").toLowerCase();
    if (os.contains("mac")) {
      return "libspannerplan_ffi.dylib";
    }
    if (os.contains("linux")) {
      return "libspannerplan_ffi.so";
    }
    if (os.contains("win")) {
      return "spannerplan_ffi.dll";
    }
    throw new IllegalStateException("unsupported platform: " + os);
  }
}
