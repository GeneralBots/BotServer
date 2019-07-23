#! /usr/bin / env node

const Fs = require('fs');
const Path = require('path');
const { exec } = require('child_process');

try {

    var run = () => {
        const GBServer = require("./dist/src/app").GBServer
        process.env.PWD = __dirname;
        GBServer.run();
    }

    var processDist = () => {
        if (!Fs.existsSync('dist')) {
            console.log(`Compiling...`);
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

    if (!Fs.existsSync('node_modules')) {
        console.log(`Installing modules for the first time, please wait...`);
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
}
catch (e) {
    console.log(e);
}


