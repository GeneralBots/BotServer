#!/usr/bin/env node

import Fs from 'fs';
import Path from 'path';
import { exec } from 'child_process';
import pjson from './package.json' assert { type: "json" };
import * as GBServer from "./dist/src/app.js";

// Displays version of Node JS being used at runtime and others attributes.

console.log(`[GB Runtime] BotServer        = v${pjson.version}`);
console.log(`[GB Runtime] BotLib           = v${pjson.dependencies.botlib}`);
console.log(`[GB Runtime] BotBuilder (MS)  = v${pjson.dependencies.botbuilder}`);
console.log(`[GB Runtime] NodeJS           = ${process.version}`);
console.log(`[GB Runtime] platform         = ${process.platform}`);
console.log(`[GB Runtime] architecture     = ${process.arch}`);
console.log(`[GB Runtime] argv             = ${process.argv}`);
console.log(`[GB Runtime] debugPort        = ${process.debugPort}`);

var now = () => {
    return (new Date()).toISOString().replace(/T/, ' ').replace(/\..+/, '') + ' UTC';
}
var __dirname = process.env.PWD;
try {

    var run = () => {
        
        console.log(`[GB Runtime] Initializing General Bots (BotServer)...`);
        console.log(`[GB Runtime] ${now()} - Running on '${import.meta.url}'`);
        GBServer.GBServer.run();
    }
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
        }
        else {
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
    }
    else {
        processDist();
    }
} catch (e) {
    console.log(e);
}
