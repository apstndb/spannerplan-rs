# frozen_string_literal: true

require 'google/cloud/spanner/v1'

module QueryModeOption
  module_function

  def spanner_query_mode(mode)
    const = Google::Cloud::Spanner::V1::ExecuteSqlRequest::QueryMode
    mode == 'PROFILE' ? :PROFILE : :PLAN
  end

  def render_mode(mode)
    mode == 'PROFILE' ? 'PROFILE' : 'PLAN'
  end
end
