# frozen_string_literal: true

CliOptions = Struct.new(:query_mode, :project, :instance, :database, :sql, keyword_init: true)

module CliOptionsParser
  VALID_MODES = %w[PLAN PROFILE].freeze
  DEFAULT_QUERY_FILE = File.expand_path('../query.sql', __dir__)

  module_function

  def usage
    <<~USAGE
      usage: analyze_and_render.rb [options]
        --query-mode PLAN|PROFILE   Spanner execute-sql mode (default: PLAN)
        --project PROJECT           GCP project id
        --instance INSTANCE         Spanner instance id
        --database DATABASE         Spanner database id
        --query SQL                 SQL text (overrides --query-file)
        --query-file PATH           SQL file (default: ../query.sql)

      Environment (when flags omitted):
        SPANNER_QUERY_MODE, SPANNER_PROJECT_ID, SPANNER_INSTANCE_ID,
        SPANNER_DATABASE_ID, SPANNER_QUERY, SPANNER_QUERY_FILE
    USAGE
  end

  def env_or_nil(name)
    value = ENV[name]
    return nil if value.nil? || value.strip.empty?

    value.strip
  end

  def load_sql(query, query_file)
    return query.strip unless query.nil? || query.strip.empty?

    path = query_file.nil? || query_file.strip.empty? ? DEFAULT_QUERY_FILE : query_file
    File.read(path, encoding: 'UTF-8').strip
  end

  def parse(argv = ARGV)
    query_mode = (env_or_nil('SPANNER_QUERY_MODE') || 'PLAN').upcase
    project = env_or_nil('SPANNER_PROJECT_ID')
    instance = env_or_nil('SPANNER_INSTANCE_ID')
    database = env_or_nil('SPANNER_DATABASE_ID')
    query = env_or_nil('SPANNER_QUERY')
    query_file = env_or_nil('SPANNER_QUERY_FILE')

    args = argv.dup
    i = 0
    while i < args.length
      arg = args[i]
      case arg
      when '-h', '--help'
        warn usage
        exit 0
      when '--query-mode'
        raise ArgumentError, 'missing value for --query-mode' unless i + 1 < args.length

        query_mode = args[i + 1].upcase
        i += 2
      when '--project'
        raise ArgumentError, 'missing value for --project' unless i + 1 < args.length

        project = args[i + 1]
        i += 2
      when '--instance'
        raise ArgumentError, 'missing value for --instance' unless i + 1 < args.length

        instance = args[i + 1]
        i += 2
      when '--database'
        raise ArgumentError, 'missing value for --database' unless i + 1 < args.length

        database = args[i + 1]
        i += 2
      when '--query'
        raise ArgumentError, 'missing value for --query' unless i + 1 < args.length

        query = args[i + 1]
        i += 2
      when '--query-file'
        raise ArgumentError, 'missing value for --query-file' unless i + 1 < args.length

        query_file = args[i + 1]
        i += 2
      else
        raise ArgumentError, "unknown argument: #{arg}"
      end
    end

    raise ArgumentError, "query mode must be PLAN or PROFILE, got: #{query_mode}" unless VALID_MODES.include?(query_mode)
    raise ArgumentError, 'missing required value: set --project or SPANNER_PROJECT_ID' if project.nil? || project.empty?
    raise ArgumentError, 'missing required value: set --instance or SPANNER_INSTANCE_ID' if instance.nil? || instance.empty?
    raise ArgumentError, 'missing required value: set --database or SPANNER_DATABASE_ID' if database.nil? || database.empty?

    CliOptions.new(
      query_mode: query_mode,
      project: project,
      instance: instance,
      database: database,
      sql: load_sql(query, query_file)
    )
  end
end
