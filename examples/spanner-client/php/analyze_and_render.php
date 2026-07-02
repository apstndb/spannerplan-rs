#!/usr/bin/env php
<?php

declare(strict_types=1);

require __DIR__ . '/vendor/autoload.php';

use Google\Cloud\Spanner\V1\Client\SpannerClient;
use Google\Cloud\Spanner\V1\CreateSessionRequest;
use Google\Cloud\Spanner\V1\DeleteSessionRequest;
use Google\Cloud\Spanner\V1\ExecuteSqlRequest;
use Google\Cloud\Spanner\V1\TransactionOptions;
use Google\Cloud\Spanner\V1\TransactionOptions\PBReadOnly;
use Google\Cloud\Spanner\V1\TransactionSelector;

function fetchQueryPlan(CliOptions $opts): Google\Cloud\Spanner\V1\QueryPlan
{
    $databasePath = sprintf(
        'projects/%s/instances/%s/databases/%s',
        $opts->project,
        $opts->instance,
        $opts->database,
    );

    $client = new SpannerClient();
    $session = $client->createSession((new CreateSessionRequest())->setDatabase($databasePath));

    try {
        $request = (new ExecuteSqlRequest())
            ->setSession($session->getName())
            ->setSql($opts->sql)
            ->setQueryMode(QueryModeOption::spannerQueryMode($opts->queryMode))
            ->setTransaction(
                (new TransactionSelector())->setSingleUse(
                    (new TransactionOptions())->setReadOnly(new PBReadOnly()),
                ),
            );

        $result = $client->executeSql($request);
        if ($opts->queryMode === 'PROFILE') {
            foreach ($result->getRows() as $_row) {
            }
        }

        $plan = $result->getStats()?->getQueryPlan();
        if ($plan === null) {
            throw new RuntimeException('QueryPlan missing from ResultSetStats');
        }

        return $plan;
    } finally {
        $client->deleteSession((new DeleteSessionRequest())->setName($session->getName()));
    }
}

try {
    $opts = CliOptions::parse();
    $plan = fetchQueryPlan($opts);
    fwrite(
        STDOUT,
        SpannerAdapter::renderQueryPlan($plan, QueryModeOption::renderMode($opts->queryMode)),
    );
    exit(0);
} catch (InvalidArgumentException $e) {
    fwrite(STDERR, $e->getMessage() . PHP_EOL);
    exit(2);
} catch (Throwable $e) {
    fwrite(STDERR, $e->getMessage() . PHP_EOL);
    exit(1);
}
