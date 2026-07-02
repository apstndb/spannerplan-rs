<?php

declare(strict_types=1);

final class Spannerplan
{
    private FFI $ffi;

    public function __construct(?string $libPath = null)
    {
        $root = dirname(__DIR__, 3);
        $libPath ??= self::defaultLibPath($root);
        $headerPath = $root . '/crates/spannerplan-ffi/spannerplan.h';
        $header = file_get_contents($headerPath);
        if ($header === false) {
            throw new RuntimeException("failed to read header: {$headerPath}");
        }
        $this->ffi = FFI::cdef($header, $libPath);
    }

    public function renderTreeTableWire(
        string $planWire,
        string $mode = 'AUTO',
        string $format = 'CURRENT',
        ?string $configJson = null
    ): string {
        $isError = $this->ffi->new('int');
        $wire = $this->ffi->new('uint8_t[' . strlen($planWire) . ']');
        FFI::memcpy($wire, $planWire, strlen($planWire));
        $out = $this->ffi->spannerplan_render_tree_table_wire(
            $wire,
            strlen($planWire),
            $mode,
            $format,
            $configJson,
            FFI::addr($isError)
        );
        if ($out === null) {
            throw new RuntimeException('native render returned NULL');
        }

        $text = FFI::string($out);
        $this->ffi->spannerplan_string_free($out);

        if ($isError->cdata !== 0) {
            throw new RuntimeException($text);
        }

        return $text;
    }

    public function renderTreeTableJson(
        string $planJson,
        string $mode = 'AUTO',
        string $format = 'CURRENT',
        ?string $configJson = null
    ): string {
        $isError = $this->ffi->new('int');
        $out = $this->ffi->spannerplan_render_tree_table_json(
            $planJson,
            $mode,
            $format,
            $configJson,
            FFI::addr($isError)
        );
        if ($out === null) {
            throw new RuntimeException('native render returned NULL');
        }

        $text = FFI::string($out);
        $this->ffi->spannerplan_string_free($out);

        if ($isError->cdata !== 0) {
            throw new RuntimeException($text);
        }

        return $text;
    }

    private static function platformLibName(): string
    {
        if (PHP_OS_FAMILY === 'Darwin') {
            return 'libspannerplan_ffi.dylib';
        }
        if (PHP_OS_FAMILY === 'Windows') {
            return 'spannerplan_ffi.dll';
        }

        return 'libspannerplan_ffi.so';
    }

    private static function ciArtifactDir(): ?string
    {
        if (PHP_OS_FAMILY === 'Darwin') {
            return php_uname('m') === 'arm64' ? 'spannerplan-ffi-macos-arm64' : 'spannerplan-ffi-macos-x64';
        }
        if (PHP_OS_FAMILY === 'Linux') {
            return 'spannerplan-ffi-linux-x64';
        }
        if (PHP_OS_FAMILY === 'Windows') {
            return 'spannerplan-ffi-windows-x64';
        }

        return null;
    }

    private static function defaultLibPath(string $root): string
    {
        $env = getenv('SPANNERPLAN_FFI_LIB');
        if (is_string($env) && $env !== '' && is_file($env)) {
            return $env;
        }

        $name = self::platformLibName();

        $ffiDir = getenv('SPANNERPLAN_FFI_DIR');
        if (is_string($ffiDir) && $ffiDir !== '') {
            $candidate = $ffiDir . '/' . $name;
            if (is_file($candidate)) {
                return $candidate;
            }
        }

        foreach (['debug', 'release'] as $profile) {
            $candidate = $root . '/target/' . $profile . '/' . $name;
            if (is_file($candidate)) {
                return $candidate;
            }
        }

        $artifact = self::ciArtifactDir();
        if ($artifact !== null) {
            $candidate = $root . '/artifacts/' . $artifact . '/' . $name;
            if (is_file($candidate)) {
                return $candidate;
            }
        }

        throw new RuntimeException(
            'spannerplan native library not found; set SPANNERPLAN_FFI_LIB, ' .
            'SPANNERPLAN_FFI_DIR, or run `cargo build -p spannerplan-ffi` from the repo root'
        );
    }
}
