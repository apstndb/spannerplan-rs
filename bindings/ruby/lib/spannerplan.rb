# frozen_string_literal: true

require 'fiddle'
require 'fiddle/import'
require 'json'

module Spannerplan
  class RenderError < StandardError; end

  module Native
    extend Fiddle::Importer

    def self.platform_lib_name
      case RUBY_PLATFORM
      when /darwin/ then 'libspannerplan_ffi.dylib'
      when /linux/ then 'libspannerplan_ffi.so'
      when /mswin|mingw/ then 'spannerplan_ffi.dll'
      else raise "unsupported platform: #{RUBY_PLATFORM}"
      end
    end

    def self.ci_artifact_dir
      case RUBY_PLATFORM
      when /darwin.*arm64|aarch64/ then 'spannerplan-ffi-macos-arm64'
      when /darwin/ then 'spannerplan-ffi-macos-x64'
      when /linux/ then 'spannerplan-ffi-linux-x64'
      when /mswin|mingw/ then 'spannerplan-ffi-windows-x64'
      end
    end

    def self.lib_path
      env = ENV['SPANNERPLAN_FFI_LIB']
      return env if env && File.file?(env)

      root = File.expand_path('../../..', __dir__)
      name = platform_lib_name

      ffi_dir = ENV['SPANNERPLAN_FFI_DIR']
      if ffi_dir
        candidate = File.join(ffi_dir, name)
        return candidate if File.file?(candidate)
      end

      %w[debug release].each do |profile|
        candidate = File.join(root, 'target', profile, name)
        return candidate if File.file?(candidate)
      end

      artifact = ci_artifact_dir
      if artifact
        candidate = File.join(root, 'artifacts', artifact, name)
        return candidate if File.file?(candidate)
      end

      raise "spannerplan native library not found; set SPANNERPLAN_FFI_LIB, " \
            'SPANNERPLAN_FFI_DIR, or run `cargo build -p spannerplan-ffi` from the repo root'
    end

    dlload lib_path

    extern 'char* spannerplan_render_tree_table_json(const char* plan_json, ' \
           'const char* mode, const char* format, const char* config_json, ' \
           'int* out_is_error)'
    extern 'char* spannerplan_render_tree_table_wire(void* plan_wire, ' \
           'size_t plan_wire_len, const char* mode, const char* format, ' \
           'const char* config_json, int* out_is_error)'
    extern 'void spannerplan_string_free(char* s)'
  end

  module_function

  def render_tree_table_wire(plan_wire, mode: 'AUTO', format: 'CURRENT', config: nil)
    is_error = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT)
    is_error[0, Fiddle::SIZEOF_INT] = [0].pack('i')
    config_json = config ? JSON.generate(config) : nil

    wire = Fiddle::Pointer[plan_wire]
    out = Native.spannerplan_render_tree_table_wire(
      wire,
      plan_wire.bytesize,
      mode,
      format,
      config_json,
      is_error
    )
    raise RenderError, 'native render returned NULL' if out.null?

    text = out.to_s
    Native.spannerplan_string_free(out)
    err = is_error[0, Fiddle::SIZEOF_INT].unpack1('i')
    raise RenderError, text if err != 0

    text
  ensure
    is_error&.free
  end

  def render_tree_table_json(plan_json, mode: 'AUTO', format: 'CURRENT', config: nil)
    is_error = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT)
    is_error[0, Fiddle::SIZEOF_INT] = [0].pack('i')
    config_json = config ? JSON.generate(config) : nil

    out = Native.spannerplan_render_tree_table_json(
      plan_json,
      mode,
      format,
      config_json,
      is_error
    )
    raise RenderError, 'native render returned NULL' if out.null?

    text = out.to_s
    Native.spannerplan_string_free(out)
    err = is_error[0, Fiddle::SIZEOF_INT].unpack1('i')
    raise RenderError, text if err != 0

    text
  ensure
    is_error&.free
  end
end
