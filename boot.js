#!/usr/bin/env node

const Fs = require('fs');
const Path = require('path');
const { exec } = require('child_process');

// Displays version of Node JS being used at runtime and others attributes.

console.log(`[GB Runtime] version   = ${process.version}`);
console.log(`[GB Runtime] env       = ${process.env}`);
console.log(`[GB Runtime] platform  = ${process.platform}`);
console.log(`[GB Runtime] release   = ${process.release}`);
console.log(`[GB Runtime] argv      = ${process.argv}`);
console.log(`[GB Runtime] env.USER  = ${process.env.USER}`);
console.log(`[GB Runtime] env.PATH  = ${process.env.PATH.split(':').join('\n')}`);
console.log(`[GB Runtime] env.PWD   = ${process.env.PWD}`);
console.log(`[GB Runtime] env.HOME  = ${process.env.HOME}`);
console.log(`[GB Runtime] debugPort = ${process.debugPort}`);

var now = () => { 
    return (new Date()).toISOString().replace(/T/, ' ').replace(/\..+/, '') + ' UTC';
}

try {

    var run = () => {
        console.log(`[GB Runtime] Now starting General Bots Server...`);
        const GBServer = require("./dist/src/app").GBServer
        console.log(`[GB Runtime] ${now()} - Running '${GBServer.name}' on '${__dirname}' directory`);
        process.env.PWD = __dirname;
        GBServer.run();
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
