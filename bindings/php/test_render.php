#!/usr/bin/env php
<?php

declare(strict_types=1);

require __DIR__ . '/src/Spannerplan.php';

$repoRoot = dirname(__DIR__, 2);
$fixture = $repoRoot . '/testdata/reference/dca.yaml';
$goldenPath = $repoRoot . '/testdata/golden/dca_plan_current.txt';

$plan = file_get_contents($fixture);
if ($plan === false) {
    fwrite(STDERR, "failed to read fixture: {$fixture}\n");
    exit(1);
}

$golden = file_get_contents($goldenPath);
if ($golden === false) {
    fwrite(STDERR, "failed to read golden: {$goldenPath}\n");
    exit(1);
}

$sp = new Spannerplan();
$output = $sp->renderTreeTableJson($plan, 'PLAN', 'CURRENT');

if ($output !== $golden) {
    fwrite(STDERR, "output does not match golden dca_plan_current.txt\n");
    exit(1);
}

if (strpos($output, 'Distributed Cross Apply') === false) {
    fwrite(STDERR, "expected marker not found in rendered output\n");
    exit(1);
}

echo "ok\n";
