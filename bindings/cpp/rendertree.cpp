#include <cstdlib>
#include <iostream>
#include <sstream>
#include <string>

extern "C" {
#include "spannerplan.h"
}

static std::string read_stdin() {
  std::ostringstream ss;
  ss << std::cin.rdbuf();
  return ss.str();
}

static void print_usage() {
  std::cerr << "Usage: rendertree [-mode AUTO|PLAN|PROFILE] "
               "[-format TRADITIONAL|CURRENT|COMPACT]\n";
  std::cerr << "  Reads YAML or JSON plan text from stdin.\n";
}

static bool split_flag_value(const std::string &arg, const char *prefix,
                             std::string &value) {
  const std::string p(prefix);
  if (arg.rfind(p, 0) != 0) {
    return false;
  }
  if (arg.size() == p.size()) {
    return true;
  }
  if (arg[p.size()] == '=') {
    value = arg.substr(p.size() + 1);
    return true;
  }
  return false;
}

static std::string value_or_next(int argc, char **argv, int &i,
                                 const std::string &flag) {
  const std::string arg = argv[i];
  std::string inline_value;
  if (split_flag_value(arg, flag.c_str(), inline_value) && !inline_value.empty()) {
    return inline_value;
  }
  if (++i >= argc) {
    std::cerr << "flag needs an argument: " << flag << "\n";
    std::exit(2);
  }
  return argv[i];
}

int main(int argc, char **argv) {
  std::string mode = "AUTO";
  std::string format = "CURRENT";

  for (int i = 1; i < argc; ++i) {
    const std::string arg = argv[i];
    if (arg == "-h" || arg == "-help" || arg == "--help") {
      print_usage();
      return 0;
    }
    if (arg == "-mode" || arg == "--mode") {
      mode = value_or_next(argc, argv, i, arg);
      continue;
    }
    if (arg == "-format" || arg == "--format") {
      format = value_or_next(argc, argv, i, arg);
      continue;
    }
    std::string inline_value;
    if (split_flag_value(arg, "-mode", inline_value) ||
        split_flag_value(arg, "--mode", inline_value)) {
      if (inline_value.empty()) {
        mode = value_or_next(argc, argv, i, "-mode");
      } else {
        mode = inline_value;
      }
      continue;
    }
    if (split_flag_value(arg, "-format", inline_value) ||
        split_flag_value(arg, "--format", inline_value)) {
      if (inline_value.empty()) {
        format = value_or_next(argc, argv, i, "-format");
      } else {
        format = inline_value;
      }
      continue;
    }
    std::cerr << "unknown flag: " << arg << "\n";
    print_usage();
    return 2;
  }

  const std::string plan = read_stdin();
  if (plan.empty()) {
    std::cerr << "no input on stdin\n";
    return 1;
  }

  int is_error = 0;
  char *out = spannerplan_render_tree_table_json(plan.c_str(), mode.c_str(),
                                                 format.c_str(), nullptr,
                                                 &is_error);
  if (out == nullptr) {
    std::cerr << "render returned NULL\n";
    return 1;
  }

  const std::string text(out);
  spannerplan_string_free(out);

  if (is_error != 0) {
    std::cerr << text << "\n";
    return 1;
  }

  std::cout << text;
  return 0;
}
