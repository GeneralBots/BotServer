#! /usr/bin / env node
const GBServer = require("./dist/src/app").GBServer
process.env.PWD = __dirname;
GBServer.run();