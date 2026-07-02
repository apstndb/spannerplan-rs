namespace SpannerPlan.Tests;

internal static class TestPaths
{
    internal static string RepoRoot
    {
        get
        {
            var dir = new DirectoryInfo(AppContext.BaseDirectory);
            while (dir is not null)
            {
                if (File.Exists(Path.Combine(dir.FullName, "Cargo.toml")) &&
                    Directory.Exists(Path.Combine(dir.FullName, "testdata")))
                {
                    return dir.FullName;
                }

                dir = dir.Parent;
            }

            throw new InvalidOperationException("could not locate repository root from test output directory");
        }
    }

    internal static string Fixture(string relativePath) =>
        Path.Combine(RepoRoot, "testdata", relativePath);

    internal static string Golden(string name) =>
        Path.Combine(RepoRoot, "testdata", "golden", $"{name}.txt");
}
