<?php

declare(strict_types=1);

use Google\Cloud\Spanner\V1\QueryPlan;

final class QueryModeOption
{
    public static function spannerQueryMode(string $mode): int
    {
        return $mode === 'PROFILE'
            ? \Google\Cloud\Spanner\V1\ExecuteSqlRequest\QueryMode::PROFILE
            : \Google\Cloud\Spanner\V1\ExecuteSqlRequest\QueryMode::PLAN;
    }

    public static function renderMode(string $mode): string
    {
        return $mode === 'PROFILE' ? 'PROFILE' : 'PLAN';
    }
}

final class SpannerAdapter
{
    public static function queryPlanToWire(QueryPlan $queryPlan): string
    {
        return $queryPlan->serializeToString();
    }

    public static function renderQueryPlan(QueryPlan $queryPlan, string $mode = 'PLAN', string $format = 'CURRENT'): string
    {
        require_once dirname(__DIR__, 3) . '/bindings/php/src/Spannerplan.php';

        $renderer = new Spannerplan();
        return $renderer->renderTreeTableWire(self::queryPlanToWire($queryPlan), $mode, $format);
    }
}
