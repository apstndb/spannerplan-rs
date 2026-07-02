using SpannerPlan;

static int Usage(string message)
{
    Console.Error.WriteLine(message);
    PrintHelp();
    return 2;
}

static void PrintHelp()
{
    Console.Error.WriteLine("""
        Usage: rendertree [flags]
          Read plan YAML or JSON from stdin; print ASCII table to stdout.

        Flags:
          -mode string        AUTO, PLAN, or PROFILE (default AUTO)
          -format string      CURRENT or TRADITIONAL (default CURRENT)
          -wrap-width int     wrap Operator column (0 = off)
          -help               show this help
        """);
}

static (string Flag, string? Value) SplitFlag(string arg)
{
    var eq = arg.IndexOf('=');
    if (eq < 0)
    {
        return (arg, null);
    }

    return (arg[..eq], arg[(eq + 1)..]);
}

static string ValueOrNext(string? value, string[] args, ref int index, string flag)
{
    if (value is not null)
    {
        return value;
    }

    if (index + 1 >= args.Length)
    {
        throw new UsageException($"flag needs an argument: {flag}");
    }

    index++;
    return args[index];
}

var mode = "AUTO";
var format = "CURRENT";
var wrapWidth = 0;

try
{
    for (var i = 0; i < args.Length; i++)
    {
        var (flag, value) = SplitFlag(args[i]);
        switch (flag)
        {
            case "-h":
            case "-help":
            case "--help":
                PrintHelp();
                return 0;
            case "-mode":
            case "--mode":
                mode = ValueOrNext(value, args, ref i, "-mode");
                break;
            case "-format":
            case "--format":
                format = ValueOrNext(value, args, ref i, "-format");
                break;
            case "-wrap-width":
            case "--wrap-width":
            {
                var s = ValueOrNext(value, args, ref i, "-wrap-width");
                if (!int.TryParse(s, out wrapWidth))
                {
                    return Usage($"invalid int value \"{s}\" for -wrap-width");
                }

                break;
            }
            default:
                if (flag.StartsWith('-'))
                {
                    return Usage($"flag provided but not defined: {flag}");
                }

                return Usage($"unexpected argument: {flag}");
        }
    }
}
catch (UsageException ex)
{
    return Usage(ex.Message);
}

string plan;
try
{
    using var stdin = Console.OpenStandardInput();
    using var reader = new StreamReader(stdin);
    plan = reader.ReadToEnd();
}
catch (Exception ex)
{
    Console.Error.WriteLine(ex.Message);
    return 1;
}

RenderConfig? config = wrapWidth > 0 ? new RenderConfig { ["wrapWidth"] = wrapWidth } : null;

try
{
    var output = PlanRenderer.RenderTreeTableJson(plan, mode, format, config);
    Console.Out.Write(output);
    return 0;
}
catch (RenderError ex)
{
    Console.Error.WriteLine(ex.Message);
    return 1;
}
catch (Exception ex)
{
    Console.Error.WriteLine(ex.Message);
    return 1;
}

sealed class UsageException(string message) : Exception(message);
