#!/usr/bin/env node

process.stdout.write(`General Bots. BotServer@${pjson.version}, botlib@${pjson.dependencies.botlib}, node@${process.version.replace('v', '')}, ${process.platform} ${process.arch} `);

import fs from 'fs/promises'; 
import os from 'node:os';
import path from 'path';
import { exec } from 'child_process';
import pjson from './package.json' assert { type: 'json' };
import {GBUtil} from './dist/src/util.js'

// Displays version of Node JS being used at runtime and others attributes.

console.log(`\nLoading General Bots VM...`);

var __dirname = process.env.PWD || process.cwd();
try {
  var run = async () => {

    import('./dist/src/app.js').then(async (gb)=> {
      await gb.GBServer.run()
    });
  };
  var processDist = async () => {
    if (!await GBUtil.exists('dist')) {
      console.log(`\n`);
      console.log(`General Bots: Compiling...`);
      exec(path.join(__dirname, 'node_modules/.bin/tsc'), async (err, stdout, stderr) => {
        if (err) {
          console.error(err);
          return;
        }
        await run();
      });
    } else {
      await run();
    }
  };

  // Installing modules if it has not been done yet.

  if (!await GBUtil.exists('node_modules')) {
    console.log(`\n`);
    console.log(`General Bots: Installing modules for the first time, please wait...`);
    exec('npm install', async (err, stdout, stderr) => {
      if (err) {
        console.error(err);
        return;
      }
      await processDist();
    });
  } else {
    await processDist();
  }
} catch (e) {
  console.log(e);
}
