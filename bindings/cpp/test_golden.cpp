#include <cstdlib>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>

extern "C" {
#include "spannerplan.h"
}

static std::string read_file(const char *path) {
  std::ifstream in(path);
  if (!in) {
    std::cerr << "failed to open: " << path << "\n";
    std::exit(1);
  }
  std::ostringstream ss;
  ss << in.rdbuf();
  return ss.str();
}

int main() {
  const char *fixture = "../../../testdata/reference/dca.yaml";
  const char *golden_path = "../../../testdata/golden/dca_plan_current.txt";

  const std::string plan = read_file(fixture);
  const std::string golden = read_file(golden_path);

  int is_error = 0;
  char *out = spannerplan_render_tree_table_json(plan.c_str(), "PLAN",
                                                 "CURRENT", nullptr, &is_error);
  if (out == nullptr) {
    std::cerr << "render returned NULL\n";
    return 1;
  }

  const std::string text(out);
  spannerplan_string_free(out);

  if (is_error != 0) {
    std::cerr << "render error: " << text << "\n";
    return 1;
  }

  if (text != golden) {
    std::cerr << "output does not match golden dca_plan_current.txt\n";
    return 1;
  }

  return 0;
}
