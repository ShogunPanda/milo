#!/usr/bin/env node

import { cronometro } from 'cronometro'
import { main } from './parsing.mjs'

if (process.argv[2]) {
  process.env.CONFIGURATION = process.argv[2]
}

await cronometro({ main })
