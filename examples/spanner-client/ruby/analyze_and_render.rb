#!/usr/bin/env ruby
# frozen_string_literal: true

require 'google/cloud/spanner/v1'

require_relative 'cli_options'
require_relative 'query_mode_option'
require_relative 'spanner_adapter'

def fetch_query_plan(opts)
  database_path = "projects/#{opts.project}/instances/#{opts.instance}/databases/#{opts.database}"
  client = Google::Cloud::Spanner::V1::Spanner::Client.new

  session = client.create_session(
    Google::Cloud::Spanner::V1::CreateSessionRequest.new(database: database_path)
  )
  begin
    result = client.execute_sql(
      Google::Cloud::Spanner::V1::ExecuteSqlRequest.new(
        session: session.name,
        sql: opts.sql,
        transaction: { single_use: { read_only: {} } },
        query_mode: QueryModeOption.spanner_query_mode(opts.query_mode)
      )
    )

    result.rows.each { |_| } if opts.query_mode == 'PROFILE'

    stats = result.stats
    raise 'QueryPlan missing from ResultSetStats' if stats.nil? || stats.query_plan.nil?

    stats.query_plan
  ensure
    client.delete_session(
      Google::Cloud::Spanner::V1::DeleteSessionRequest.new(name: session.name)
    )
  end
end

def main
  opts = CliOptionsParser.parse
  plan = fetch_query_plan(opts)
  print SpannerAdapter.render_query_plan(plan, mode: QueryModeOption.render_mode(opts.query_mode))
  0
rescue ArgumentError => e
  warn e.message
  2
rescue StandardError => e
  warn e.message
  1
end

exit main if $PROGRAM_NAME == __FILE__
