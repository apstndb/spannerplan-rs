#ifndef SPANNER_CLIENT_EXAMPLE_CLI_OPTIONS_HPP_
#define SPANNER_CLIENT_EXAMPLE_CLI_OPTIONS_HPP_

#include <string>
#include <vector>

struct CliOptions {
  std::string query_mode;
  std::string project;
  std::string instance;
  std::string database;
  std::string sql;
};

CliOptions ParseCliOptions(int argc, char **argv);

#endif  // SPANNER_CLIENT_EXAMPLE_CLI_OPTIONS_HPP_
