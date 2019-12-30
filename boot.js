#!/usr/bin/env node

const Fs = require('fs');
const Path = require('path');
const { exec } = require('child_process');

// display version of Node JS being used at runtime and others runtime attributes
console.log(`process.version   = ${process.version}`);
console.log(`process.env       = ${process.env}`);
console.log(`process.platform  = ${process.platform}`);
console.log(`process.release   = ${process.release}`);
console.log(`process.argv      = ${process.argv}`);
console.log(`process.env.USER  = ${process.env.USER}`);
console.log(`process.env.PATH  = ${process.env.PATH.split(':').join('\n')}`);
console.log(`process.env.PWD   = ${process.env.PWD}`);
console.log(`process.env.HOME  = ${process.env.HOME}`);
console.log(`process.debugPort = ${process.debugPort}`);

var now = () => { 
    return (new Date()).toISOString().replace(/T/, ' ').replace(/\..+/, '') + ' UTC';
}

try {
    // 1. define functions: run and processDist 
    var run = () => {
        const GBServer = require("./dist/src/app").GBServer
        console.log(`${now()} - Running '${GBServer.name}' on '${__dirname}' directory`);
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

    // 2. start running 
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
