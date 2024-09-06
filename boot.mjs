#!/usr/bin/env node

process.stdout.write(`General Bots. BotServer@${pjson.version}, botlib@${pjson.dependencies.botlib}, node@${process.version.replace('v', '')}, ${process.platform} ${process.arch} `);

import fs from 'fs';
import os from 'node:os';
import path from 'path';
import { exec } from 'child_process';
import pjson from './package.json' assert { type: 'json' };


// Displays version of Node JS being used at runtime and others attributes.

console.log(`\nLoading virtual machine source code files...`);

var __dirname = process.env.PWD || process.cwd();
try {
  var run = () => {

    import('./dist/src/app.js').then((gb)=> {
      gb.GBServer.run()
    });
  };
  var processDist = () => {
    if (!fs.existsSync('dist')) {
      console.log(`\n`);
      console.log(`Generall Bots: Compiling...`);
      exec(path.join(__dirname, 'node_modules/.bin/tsc'), (err, stdout, stderr) => {
        if (err) {
          console.error(err);
          return;
        }
        run();
      });
    } else {
      run();
    }
  };

  // Installing modules if it has not been done yet.

  if (!fs.existsSync('node_modules')) {
    console.log(`\n`);
    console.log(`Generall Bots: Installing modules for the first time, please wait...`);
    exec('npm install', (err, stdout, stderr) => {
      if (err) {
        console.error(err);
        return;
      }
      processDist();
    });
  } else {
    processDist();
  }
} catch (e) {
  console.log(e);
}
