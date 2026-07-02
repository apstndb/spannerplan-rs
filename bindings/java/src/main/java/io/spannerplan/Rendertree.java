package io.spannerplan;

import java.io.IOException;
import java.io.InputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

/** Minimal `rendertree` CLI: stdin plan YAML/JSON -> ASCII table on stdout. */
public final class Rendertree {
  private static final int USAGE_EXIT = 2;

  private static final Set<String> PRINT_PRESET_NAMES =
      Set.of("basic", "enhanced", "full", "none");

  private static final Set<String> PRINT_SECTIONS =
      Set.of("predicates", "ordering", "aggregate", "typed", "full");

  private Rendertree() {}

  public static void main(String[] args) throws IOException {
    System.exit(run(args, System.in, System.out, System.err));
  }

  static int run(String[] args, InputStream stdin, PrintStream stdout, PrintStream stderr)
      throws IOException {
    ParsedArgs parsed;
    try {
      parsed = parseArgs(args, stderr);
    } catch (UsageException e) {
      return USAGE_EXIT;
    }

    if (parsed.help) {
      return 0;
    }

    byte[] input = stdin.readAllBytes();
    String format = parsed.compact ? "COMPACT" : "CURRENT";

    try {
      String output =
          Spannerplan.renderTreeTableJson(
              new String(input, StandardCharsets.UTF_8),
              parsed.mode.toUpperCase(Locale.ROOT),
              format,
              parsed.config);
      stdout.print(output);
      return 0;
    } catch (RenderError e) {
      stderr.println(e.getMessage());
      return 1;
    }
  }

  private static final class ParsedArgs {
    boolean help;
    String mode = "AUTO";
    String print = "basic";
    boolean compact;
    int wrapWidth;
    Map<String, ?> config;
  }

  private static ParsedArgs parseArgs(String[] args, PrintStream stderr) {
    ParsedArgs parsed = new ParsedArgs();
    int i = 0;
    while (i < args.length) {
      String arg = args[i];
      String flag;
      String inlineValue = null;
      int eq = arg.indexOf('=');
      if (eq >= 0) {
        flag = arg.substring(0, eq);
        inlineValue = arg.substring(eq + 1);
      } else {
        flag = arg;
      }

      switch (flag) {
        case "-h":
        case "-help":
        case "--help":
          printUsage(stderr);
          parsed.help = true;
          return parsed;
        case "-mode":
        case "--mode":
          parsed.mode = valueOrNext(inlineValue, args, i, "-mode", stderr);
          break;
        case "-print":
        case "--print":
          parsed.print = valueOrNext(inlineValue, args, i, "-print", stderr);
          break;
        case "-compact":
        case "--compact":
          parsed.compact = parseBoolFlag(inlineValue, true);
          break;
        case "-wrap-width":
        case "--wrap-width":
          {
            String value = valueOrNext(inlineValue, args, i, "-wrap-width", stderr);
            try {
              parsed.wrapWidth = Integer.parseInt(value);
            } catch (NumberFormatException e) {
              usageError(stderr, "invalid int value \"" + value + "\" for -wrap-width");
            }
          }
          break;
        default:
          if (flag.startsWith("-")) {
            usageError(stderr, "flag provided but not defined: " + flag);
          } else {
            usageError(stderr, "unexpected argument: " + flag);
          }
      }
      if (inlineValue == null && needsValue(flag)) {
        i++;
      }
      i++;
    }

    if (!Set.of("auto", "plan", "profile").contains(parsed.mode.toLowerCase(Locale.ROOT))) {
      stderr.println(
          "Invalid value for -mode flag: invalid input: "
              + parsed.mode
              + ". Must be one of AUTO, PLAN, PROFILE (case-insensitive).");
      printUsage(stderr);
      throw new UsageException();
    }

    if (parsed.wrapWidth < 0) {
      stderr.println(
          "Invalid value for -wrap-width flag: wrapWidth cannot be negative: "
              + parsed.wrapWidth);
      printUsage(stderr);
      throw new UsageException();
    }

    try {
      parsed.config = buildConfig(parsed.print, parsed.wrapWidth);
    } catch (UsageException e) {
      stderr.println("Invalid value for -print flag: " + e.getMessage());
      printUsage(stderr);
      throw e;
    }

    return parsed;
  }

  private static Map<String, ?> buildConfig(String printValue, int wrapWidth) {
    Map<String, Object> config = new HashMap<>();
    List<String> printSections = parsePrint(printValue);
    if (printSections != null) {
      config.put("printSections", printSections);
    }
    if (wrapWidth != 0) {
      config.put("wrapWidth", wrapWidth);
    }
    return config.isEmpty() ? null : config;
  }

  private static List<String> parsePrint(String value) {
    String trimmed = value.trim();
    if (trimmed.isEmpty()) {
      return List.of();
    }

    if (!trimmed.contains(",")) {
      String key = trimmed.toLowerCase(Locale.ROOT);
      if (PRINT_PRESET_NAMES.contains(key)) {
        return presetSections(key);
      }
      if (PRINT_SECTIONS.contains(key)) {
        return List.of(key);
      }
      throw new UsageException("unknown print preset or section: \"" + trimmed + "\"");
    }

    List<String> sections = new ArrayList<>();
    Set<String> seen = new HashSet<>();
    for (String raw : trimmed.split(",")) {
      String token = raw.trim().toLowerCase(Locale.ROOT);
      if (token.isEmpty()) {
        throw new UsageException("print section must not be empty");
      }
      if (PRINT_PRESET_NAMES.contains(token)) {
        throw new UsageException(
            "print preset \"" + raw.trim() + "\" cannot be combined with section list");
      }
      if (!PRINT_SECTIONS.contains(token)) {
        throw new UsageException("unknown print section: \"" + raw.trim() + "\"");
      }
      if (!seen.add(token)) {
        throw new UsageException("duplicate print section: " + token);
      }
      sections.add(token);
    }

    if (sections.size() > 1) {
      for (String section : sections) {
        if ("typed".equals(section) || "full".equals(section)) {
          throw new UsageException(
              "print section \"" + section + "\" cannot be combined with other sections");
        }
      }
    }

    return sections;
  }

  private static List<String> presetSections(String preset) {
    return switch (preset) {
      case "basic" -> null;
      case "enhanced" -> List.of("predicates", "ordering", "aggregate");
      case "full" -> List.of("full");
      case "none" -> List.of();
      default -> throw new UsageException("unknown print preset: \"" + preset + "\"");
    };
  }

  private static boolean needsValue(String flag) {
    return switch (flag) {
      case "-mode", "--mode", "-print", "--print", "-wrap-width", "--wrap-width" -> true;
      default -> false;
    };
  }

  private static String valueOrNext(
      String inline, String[] args, int index, String flag, PrintStream stderr) {
    if (inline != null) {
      return inline;
    }
    if (index + 1 >= args.length) {
      usageError(stderr, "flag needs an argument: " + flag);
    }
    return args[index + 1];
  }

  private static boolean parseBoolFlag(String inline, boolean defaultValue) {
    if (inline == null) {
      return defaultValue;
    }
    if ("true".equals(inline)) {
      return true;
    }
    if ("false".equals(inline)) {
      return false;
    }
    throw new UsageException("invalid boolean value \"" + inline + "\"");
  }

  private static void usageError(PrintStream stderr, String message) {
    stderr.println(message);
    printUsage(stderr);
    throw new UsageException();
  }

  private static void printUsage(PrintStream stderr) {
    stderr.println("Usage of rendertree:");
    stderr.println("  -compact");
    stderr.println("    \tEnable compact format");
    stderr.println("  -h");
    stderr.println("    \tShow this help message");
    stderr.println("  -mode string");
    stderr.println("    \tPROFILE, PLAN, AUTO (ignore case) (default \"AUTO\")");
    stderr.println("  -print string");
    stderr.println(
        "    \tAppendix preset (basic, enhanced, full, none) or comma-separated sections "
            + "(default \"basic\")");
    stderr.println("  -wrap-width int");
    stderr.println(
        "    \tWrap Operator column at this width; 0 disables wrapping (default 0)");
  }

  private static final class UsageException extends RuntimeException {
    UsageException() {
      super();
    }

    UsageException(String message) {
      super(message);
    }
  }
}
