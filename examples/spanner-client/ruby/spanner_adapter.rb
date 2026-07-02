# frozen_string_literal: true

module SpannerAdapter
  module_function

  def query_plan_to_wire(query_plan)
    query_plan.class.encode(query_plan)
  end

  def render_query_plan(query_plan, mode: 'PLAN', format: 'CURRENT')
    require 'spannerplan'

    Spannerplan.render_tree_table_wire(query_plan_to_wire(query_plan), mode: mode, format: format)
  end
end
