<?php

declare(strict_types=1);

final class CliOptions
{
    public function __construct(
        public readonly string $queryMode,
        public readonly string $project,
        public readonly string $instance,
        public readonly string $database,
        public readonly string $sql,
    ) {
    }

    public static function parse(array $argv = []): self
    {
        $args = $argv !== [] ? $argv : array_slice($_SERVER['argv'] ?? [], 1);
        $queryMode = strtoupper(self::envOrDefault('SPANNER_QUERY_MODE', 'PLAN'));
        $project = self::envOrNull('SPANNER_PROJECT_ID');
        $instance = self::envOrNull('SPANNER_INSTANCE_ID');
        $database = self::envOrNull('SPANNER_DATABASE_ID');
        $query = self::envOrNull('SPANNER_QUERY');
        $queryFile = self::envOrNull('SPANNER_QUERY_FILE');

        for ($i = 0; $i < count($args); $i++) {
            $arg = $args[$i];
            if ($arg === '-h' || $arg === '--help') {
                self::printUsage();
                exit(0);
            }
            if ($arg === '--query-mode' && isset($args[$i + 1])) {
                $queryMode = strtoupper($args[++$i]);
                continue;
            }
            if ($arg === '--project' && isset($args[$i + 1])) {
                $project = $args[++$i];
                continue;
            }
            if ($arg === '--instance' && isset($args[$i + 1])) {
                $instance = $args[++$i];
                continue;
            }
            if ($arg === '--database' && isset($args[$i + 1])) {
                $database = $args[++$i];
                continue;
            }
            if ($arg === '--query' && isset($args[$i + 1])) {
                $query = $args[++$i];
                continue;
            }
            if ($arg === '--query-file' && isset($args[$i + 1])) {
                $queryFile = $args[++$i];
                continue;
            }
            throw new InvalidArgumentException("unknown argument: {$arg}");
        }

        if ($queryMode !== 'PLAN' && $queryMode !== 'PROFILE') {
            throw new InvalidArgumentException("query mode must be PLAN or PROFILE, got: {$queryMode}");
        }
        self::requireValue('project', 'SPANNER_PROJECT_ID', $project);
        self::requireValue('instance', 'SPANNER_INSTANCE_ID', $instance);
        self::requireValue('database', 'SPANNER_DATABASE_ID', $database);

        return new self(
            $queryMode,
            $project,
            $instance,
            $database,
            self::loadSql($query, $queryFile),
        );
    }

    private static function requireValue(string $flag, string $env, ?string $value): void
    {
        if ($value === null || trim($value) === '') {
            throw new InvalidArgumentException("missing required value: set --{$flag} or {$env}");
        }
    }

    private static function loadSql(?string $query, ?string $queryFile): string
    {
        if ($query !== null && trim($query) !== '') {
            return trim($query);
        }
        $path = ($queryFile !== null && trim($queryFile) !== '') ? $queryFile : __DIR__ . '/../query.sql';
        $sql = file_get_contents($path);
        if ($sql === false) {
            throw new RuntimeException("failed to read query file: {$path}");
        }

        return trim($sql);
    }

    private static function envOrNull(string $name): ?string
    {
        $value = getenv($name);
        if (!is_string($value) || trim($value) === '') {
            return null;
        }

        return trim($value);
    }

    private static function envOrDefault(string $name, string $default): string
    {
        return self::envOrNull($name) ?? $default;
    }

    private static function printUsage(): void
    {
        fwrite(STDERR, <<<'USAGE'
usage: analyze_and_render.php [options]
  --query-mode PLAN|PROFILE   Spanner execute-sql mode (default: PLAN)
  --project PROJECT           GCP project id
  --instance INSTANCE         Spanner instance id
  --database DATABASE         Spanner database id
  --query SQL                 SQL text (overrides --query-file)
  --query-file PATH           SQL file (default: ../query.sql)

Environment (when flags omitted):
  SPANNER_QUERY_MODE, SPANNER_PROJECT_ID, SPANNER_INSTANCE_ID,
  SPANNER_DATABASE_ID, SPANNER_QUERY, SPANNER_QUERY_FILE

USAGE);
    }
}
