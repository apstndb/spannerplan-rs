#!/usr/bin/env ruby
# frozen_string_literal: true

$LOAD_PATH.unshift File.expand_path('../lib', __dir__)
require 'spannerplan'

repo_root = File.expand_path('../../..', __dir__)
fixture = File.join(repo_root, 'testdata/reference/dca.yaml')
golden = File.join(repo_root, 'testdata/golden/dca_plan_current.txt')

plan = File.read(fixture)
output = Spannerplan.render_tree_table_json(plan, mode: 'PLAN', format: 'CURRENT')
expected = File.read(golden)

unless output == expected
  warn 'output does not match golden dca_plan_current.txt'
  exit 1
end

unless output.include?('Distributed Cross Apply')
  warn 'expected marker not found in rendered output'
  exit 1
end

begin
  Spannerplan.render_tree_table_json('not json')
  warn 'expected RenderError for invalid json'
  exit 1
rescue Spannerplan::RenderError
  # expected
end

puts 'ok'
