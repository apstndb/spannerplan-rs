using Google.Cloud.Spanner.V1;
using Google.Protobuf;
using SpannerPlan;

static string? EnvOrNull(string name)
{
    var value = Environment.GetEnvironmentVariable(name);
    return string.IsNullOrWhiteSpace(value) ? null : value.Trim();
}

static string DefaultQueryFile()
{
    // bin/Debug/net8.0 -> examples/spanner-client/query.sql
    var examplesRoot = Path.GetFullPath(
        Path.Combine(AppContext.BaseDirectory, "..", "..", "..", ".."));
    return Path.Combine(examplesRoot, "query.sql");
}

static string LoadSql(string? query, string? queryFile)
{
    if (!string.IsNullOrWhiteSpace(query))
    {
        return query.Trim();
    }

    var path = string.IsNullOrWhiteSpace(queryFile) ? DefaultQueryFile() : queryFile;
    return File.ReadAllText(path).Trim();
}

static void PrintUsage()
{
    Console.Error.WriteLine(
        "usage: AnalyzeAndRender [options]\n" +
        "  --query-mode PLAN|PROFILE   Spanner execute-sql mode (default: PLAN)\n" +
        "  --project PROJECT           GCP project id\n" +
        "  --instance INSTANCE         Spanner instance id\n" +
        "  --database DATABASE         Spanner database id\n" +
        "  --query SQL                 SQL text (overrides --query-file)\n" +
        "  --query-file PATH           SQL file (default: ../query.sql)\n" +
        "\n" +
        "Environment (when flags omitted):\n" +
        "  SPANNER_QUERY_MODE, SPANNER_PROJECT_ID, SPANNER_INSTANCE_ID,\n" +
        "  SPANNER_DATABASE_ID, SPANNER_QUERY, SPANNER_QUERY_FILE");
}

static (string QueryMode, string Project, string Instance, string Database, string Sql) ParseCliOptions(
    string[] cliArgs)
{
    var queryMode = (EnvOrNull("SPANNER_QUERY_MODE") ?? "PLAN").ToUpperInvariant();
    var project = EnvOrNull("SPANNER_PROJECT_ID");
    var instance = EnvOrNull("SPANNER_INSTANCE_ID");
    var database = EnvOrNull("SPANNER_DATABASE_ID");
    var query = EnvOrNull("SPANNER_QUERY");
    var queryFile = EnvOrNull("SPANNER_QUERY_FILE");

    for (var i = 0; i < cliArgs.Length; i++)
    {
        if (cliArgs[i] is "-h" or "--help")
        {
            PrintUsage();
            Environment.Exit(0);
        }

        if (cliArgs[i] == "--query-mode" && i + 1 < cliArgs.Length)
        {
            queryMode = cliArgs[++i].ToUpperInvariant();
            continue;
        }
        if (cliArgs[i] == "--project" && i + 1 < cliArgs.Length)
        {
            project = cliArgs[++i];
            continue;
        }
        if (cliArgs[i] == "--instance" && i + 1 < cliArgs.Length)
        {
            instance = cliArgs[++i];
            continue;
        }
        if (cliArgs[i] == "--database" && i + 1 < cliArgs.Length)
        {
            database = cliArgs[++i];
            continue;
        }
        if (cliArgs[i] == "--query" && i + 1 < cliArgs.Length)
        {
            query = cliArgs[++i];
            continue;
        }
        if (cliArgs[i] == "--query-file" && i + 1 < cliArgs.Length)
        {
            queryFile = cliArgs[++i];
            continue;
        }

        throw new ArgumentException($"unknown argument: {cliArgs[i]}");
    }

    if (queryMode is not ("PLAN" or "PROFILE"))
    {
        throw new ArgumentException($"query mode must be PLAN or PROFILE, got: {queryMode}");
    }
    if (string.IsNullOrWhiteSpace(project))
    {
        throw new InvalidOperationException("missing required value: set --project or SPANNER_PROJECT_ID");
    }
    if (string.IsNullOrWhiteSpace(instance))
    {
        throw new InvalidOperationException("missing required value: set --instance or SPANNER_INSTANCE_ID");
    }
    if (string.IsNullOrWhiteSpace(database))
    {
        throw new InvalidOperationException("missing required value: set --database or SPANNER_DATABASE_ID");
    }

    return (queryMode, project, instance, database, LoadSql(query, queryFile));
}

static ExecuteSqlRequest.Types.QueryMode ToSpannerQueryMode(string mode) =>
    mode == "PROFILE"
        ? ExecuteSqlRequest.Types.QueryMode.Profile
        : ExecuteSqlRequest.Types.QueryMode.Plan;

static string RenderModeFor(string mode) => mode == "PROFILE" ? "PROFILE" : "PLAN";

var (queryMode, project, instance, database, sql) = ParseCliOptions(args);
var databasePath = $"projects/{project}/instances/{instance}/databases/{database}";

var client = await SpannerClient.CreateAsync();
var session = await client.CreateSessionAsync(new CreateSessionRequest
{
    Database = databasePath,
});

var response = await client.ExecuteSqlAsync(new ExecuteSqlRequest
{
    Session = session.Name,
    Sql = sql,
    QueryMode = ToSpannerQueryMode(queryMode),
});

var plan = response.Stats?.QueryPlan;
if (plan is null)
{
    throw new InvalidOperationException(
        $"QueryPlan missing from {queryMode}-mode execute_sql response");
}

Console.Write(PlanRenderer.RenderTreeTableWire(
    plan.ToByteArray(),
    RenderModeFor(queryMode),
    "CURRENT"));
