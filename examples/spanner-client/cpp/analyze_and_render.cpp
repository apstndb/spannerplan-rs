#include "cli_options.hpp"

#include <google/cloud/spanner/client.h>

#include <cstdlib>
#include <iostream>
#include <stdexcept>
#include <string>

extern "C" {
#include "spannerplan.h"
}

namespace spanner = ::google::cloud::spanner;

namespace {

std::string RenderModeFor(const std::string &query_mode) {
  return query_mode == "PROFILE" ? "PROFILE" : "PLAN";
}

std::string RenderWire(const spanner::ExecutionPlan &plan, const std::string &render_mode) {
  const std::string wire = plan.SerializeAsString();
  if (wire.empty()) {
    throw std::runtime_error("empty plan wire bytes");
  }

  int is_error = 0;
  char *out = spannerplan_render_tree_table_wire(
      reinterpret_cast<const uint8_t *>(wire.data()), wire.size(), render_mode.c_str(),
      "CURRENT", nullptr, &is_error);
  if (out == nullptr) {
    throw std::runtime_error("native render returned NULL");
  }

  const std::string text(out);
  spannerplan_string_free(out);
  if (is_error != 0) {
    throw std::runtime_error(text);
  }
  return text;
}

spanner::ExecutionPlan FetchQueryPlan(spanner::Client &client, const std::string &sql,
                                      const std::string &query_mode) {
  const spanner::SqlStatement statement(sql);

  if (query_mode == "PLAN") {
    auto plan = client.AnalyzeSql(spanner::MakeReadOnlyTransaction(), statement);
    if (!plan) {
      throw std::runtime_error("AnalyzeSql failed: " + plan.status().message());
    }
    return *plan;
  }

  auto result = client.ProfileQuery(statement);
  for (auto const &row : result) {
    (void)row;
  }
  auto plan = result.ExecutionPlan();
  if (!plan) {
    throw std::runtime_error("QueryPlan missing from PROFILE query");
  }
  return *plan;
}

}  // namespace

int main(int argc, char **argv) try {
  const CliOptions opts = ParseCliOptions(argc, argv);

  spanner::Client client(spanner::MakeConnection(
      spanner::Database(opts.project, opts.instance, opts.database)));

  const spanner::ExecutionPlan plan =
      FetchQueryPlan(client, opts.sql, opts.query_mode);
  const std::string table = RenderWire(plan, RenderModeFor(opts.query_mode));
  std::cout << table;
  return 0;
} catch (const std::exception &ex) {
  std::cerr << ex.what() << "\n";
  return 1;
}
