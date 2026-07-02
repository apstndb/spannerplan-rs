#include "cli_options.hpp"

#include <cctype>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <string>

namespace {

std::string EnvOrEmpty(const char *name) {
  const char *value = std::getenv(name);
  if (value == nullptr) return {};
  std::string trimmed(value);
  while (!trimmed.empty() && (trimmed.front() == ' ' || trimmed.front() == '\t')) {
    trimmed.erase(trimmed.begin());
  }
  while (!trimmed.empty() && (trimmed.back() == ' ' || trimmed.back() == '\t')) {
    trimmed.pop_back();
  }
  return trimmed;
}

std::string DefaultQueryFile() {
#ifdef SPANNER_CLIENT_QUERY_FILE_DEFAULT
  return SPANNER_CLIENT_QUERY_FILE_DEFAULT;
#else
  return "../query.sql";
#endif
}

std::string LoadSql(const std::string &query, const std::string &query_file) {
  if (!query.empty()) return query;
  const std::string path = query_file.empty() ? DefaultQueryFile() : query_file;
  std::ifstream in(path);
  if (!in) {
    throw std::runtime_error("failed to read query file: " + path);
  }
  std::ostringstream ss;
  ss << in.rdbuf();
  std::string sql = ss.str();
  while (!sql.empty() && (sql.back() == ' ' || sql.back() == '\t' || sql.back() == '\n' ||
                          sql.back() == '\r')) {
    sql.pop_back();
  }
  return sql;
}

void PrintUsage() {
  std::cerr
      << "usage: analyze_and_render [options]\n"
      << "  --query-mode PLAN|PROFILE   Spanner execute-sql mode (default: PLAN)\n"
      << "  --project PROJECT           GCP project id\n"
      << "  --instance INSTANCE         Spanner instance id\n"
      << "  --database DATABASE         Spanner database id\n"
      << "  --query SQL                 SQL text (overrides --query-file)\n"
      << "  --query-file PATH           SQL file (default: ../query.sql)\n"
      << "\n"
      << "Environment (when flags omitted):\n"
      << "  SPANNER_QUERY_MODE, SPANNER_PROJECT_ID, SPANNER_INSTANCE_ID,\n"
      << "  SPANNER_DATABASE_ID, SPANNER_QUERY, SPANNER_QUERY_FILE\n";
}

}  // namespace

CliOptions ParseCliOptions(int argc, char **argv) {
  std::string query_mode = EnvOrEmpty("SPANNER_QUERY_MODE");
  if (query_mode.empty()) query_mode = "PLAN";
  std::string project = EnvOrEmpty("SPANNER_PROJECT_ID");
  std::string instance = EnvOrEmpty("SPANNER_INSTANCE_ID");
  std::string database = EnvOrEmpty("SPANNER_DATABASE_ID");
  std::string query = EnvOrEmpty("SPANNER_QUERY");
  std::string query_file = EnvOrEmpty("SPANNER_QUERY_FILE");

  for (int i = 1; i < argc; ++i) {
    const std::string arg = argv[i];
    if (arg == "-h" || arg == "--help") {
      PrintUsage();
      std::exit(0);
    }
    if (arg == "--query-mode" && i + 1 < argc) {
      query_mode = argv[++i];
      continue;
    }
    if (arg == "--project" && i + 1 < argc) {
      project = argv[++i];
      continue;
    }
    if (arg == "--instance" && i + 1 < argc) {
      instance = argv[++i];
      continue;
    }
    if (arg == "--database" && i + 1 < argc) {
      database = argv[++i];
      continue;
    }
    if (arg == "--query" && i + 1 < argc) {
      query = argv[++i];
      continue;
    }
    if (arg == "--query-file" && i + 1 < argc) {
      query_file = argv[++i];
      continue;
    }
    throw std::runtime_error("unknown argument: " + arg);
  }

  for (auto &mode_char : query_mode) {
    mode_char = static_cast<char>(std::toupper(static_cast<unsigned char>(mode_char)));
  }
  if (query_mode != "PLAN" && query_mode != "PROFILE") {
    throw std::runtime_error("query mode must be PLAN or PROFILE, got: " + query_mode);
  }
  if (project.empty()) {
    throw std::runtime_error("missing required value: set --project or SPANNER_PROJECT_ID");
  }
  if (instance.empty()) {
    throw std::runtime_error("missing required value: set --instance or SPANNER_INSTANCE_ID");
  }
  if (database.empty()) {
    throw std::runtime_error("missing required value: set --database or SPANNER_DATABASE_ID");
  }

  CliOptions opts;
  opts.query_mode = query_mode;
  opts.project = project;
  opts.instance = instance;
  opts.database = database;
  opts.sql = LoadSql(query, query_file);
  return opts;
}
