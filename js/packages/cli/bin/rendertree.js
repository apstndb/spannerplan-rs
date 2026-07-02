#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { runCli } from "../dist/main.js";

const stdin = readFileSync(0);
const exitCode = runCli(process.argv.slice(2), stdin);
process.exit(exitCode);
