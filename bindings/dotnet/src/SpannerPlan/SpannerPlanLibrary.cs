using System.Reflection;
using System.Runtime.InteropServices;

namespace SpannerPlan;

internal static class SpannerPlanLibrary
{
    internal const string LibName = "spannerplan_ffi";

    private static readonly Lazy<string> ResolvedPath = new(ResolvePath);

    internal static string LibraryPath => ResolvedPath.Value;

    internal static void EnsureResolver()
    {
        _ = LibraryPath;
    }

    private static string ResolvePath()
    {
        var env = Environment.GetEnvironmentVariable("SPANNERPLAN_FFI_LIB");
        if (!string.IsNullOrEmpty(env))
        {
            if (File.Exists(env))
            {
                return env;
            }

            throw new FileNotFoundException($"SPANNERPLAN_FFI_LIB not found: {env}");
        }

        var libName = DefaultLibFileName();

        var ffiDir = Environment.GetEnvironmentVariable("SPANNERPLAN_FFI_DIR");
        if (!string.IsNullOrEmpty(ffiDir))
        {
            var dirLib = PathCombine(ffiDir, libName);
            if (File.Exists(dirLib))
            {
                return dirLib;
            }
        }

        var repoRoot = FindRepoRoot();
        if (repoRoot is not null)
        {
            foreach (var profile in new[] { "debug", "release" })
            {
                var profileLib = PathCombine(repoRoot, "target", profile, libName);
                if (File.Exists(profileLib))
                {
                    return profileLib;
                }
            }

            var artifactDir = CiArtifactDir();
            if (artifactDir is not null)
            {
                var artifactLib = PathCombine(repoRoot, "artifacts", artifactDir, libName);
                if (File.Exists(artifactLib))
                {
                    return artifactLib;
                }
            }
        }

        throw new FileNotFoundException(
            "spannerplan native library not found; set SPANNERPLAN_FFI_LIB, " +
            "SPANNERPLAN_FFI_DIR, or run `cargo build -p spannerplan-ffi` from the repo root");
    }

    private static string? CiArtifactDir()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return RuntimeInformation.ProcessArchitecture == Architecture.Arm64
                ? "spannerplan-ffi-macos-arm64"
                : "spannerplan-ffi-macos-x64";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
        {
            return "spannerplan-ffi-linux-x64";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return "spannerplan-ffi-windows-x64";
        }

        return null;
    }

    private static string? FindRepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null)
        {
            if (File.Exists(PathCombine(dir.FullName, "Cargo.toml")) &&
                Directory.Exists(PathCombine(dir.FullName, "testdata")))
            {
                return dir.FullName;
            }

            dir = dir.Parent;
        }

        return null;
    }

    private static string DefaultLibFileName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return "libspannerplan_ffi.dylib";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
        {
            return "libspannerplan_ffi.so";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return "spannerplan_ffi.dll";
        }

        throw new PlatformNotSupportedException($"unsupported platform: {RuntimeInformation.OSDescription}");
    }

    private static string PathCombine(string root, params string[] parts)
    {
        var path = root;
        foreach (var part in parts)
        {
            path = System.IO.Path.Combine(path, part);
        }

        return path;
    }
}
