#!/usr/bin/env node

import Fs from 'fs';
import Path from 'path';
import { exec } from 'child_process';
import pjson from './package.json' assert { type: 'json' };

// Displays version of Node JS being used at runtime and others attributes.


console.log(``);
console.log(``);
console.log(``);
console.log(` ██████  ███████ ███    ██ ███████ ██████   █████  ██          ██████   ██████  ████████ ███████ ®`);
console.log(`██       ██      ████   ██ ██      ██   ██ ██   ██ ██          ██   ██ ██    ██    ██    ██      `);
console.log(`██   ███ █████   ██ ██  ██ █████   ██████  ███████ ██          ██████  ██    ██    ██    ███████ `);
console.log(`██    ██ ██      ██  ██ ██ ██      ██   ██ ██   ██ ██          ██   ██ ██    ██    ██         ██ `);
console.log(` ██████  ███████ ██   ████ ███████ ██   ██ ██   ██ ███████     ██████   ██████     ██    ███████ 3.0`);
console.log(``);
console.log(`botserver@${pjson.version}, botlib@${pjson.dependencies.botlib}, botbuilder@${pjson.dependencies.botbuilder}, nodeJS: ${process.version}, platform: ${process.platform}, architecture: ${process.arch}.`);
console.log(``);
console.log(``);
var now = () => {
  return new Date().toISOString().replace(/T/, ' ').replace(/\..+/, '') + ' UTC';
};
var __dirname = process.env.PWD;
try {
  var run = () => {
    import('./dist/src/app.js').then((gb)=> gb.GBServer.run());
  };
  var processDist = () => {
    if (!Fs.existsSync('dist')) {
      console.log(`${now()} - Compiling...`);
      exec(Path.join(__dirname, 'node_modules/.bin/tsc'), (err, stdout, stderr) => {
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

  if (!Fs.existsSync('node_modules')) {
    console.log(`${now()} - Installing modules for the first time, please wait...`);
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
