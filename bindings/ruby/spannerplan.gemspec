# frozen_string_literal: true

Gem::Specification.new do |s|
  s.name = 'spannerplan'
  s.version = '0.1.0.alpha.3'
  s.summary = 'Render Cloud Spanner query plans as ASCII tables (FFI-backed)'
  s.description = 'Fiddle wrapper around libspannerplan_ffi from spannerplan-rs.'
  s.authors = ['apstndb']
  s.email = ['apstndb@users.noreply.github.com']
  s.license = 'Apache-2.0'
  s.homepage = 'https://github.com/apstndb/spannerplan-rs'
  s.required_ruby_version = '>= 3.0'

  s.add_dependency 'fiddle'

  s.files = Dir['lib/**/*.rb', 'bin/rendertree', 'README.md']
  s.require_paths = ['lib']
  s.executables = ['rendertree']

  s.metadata = {
    'source_code_uri' => 'https://github.com/apstndb/spannerplan-rs/tree/main/bindings/ruby'
  }
end
