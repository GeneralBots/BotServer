@ECHO off

ECHO General Bots Command Line

IF EXIST node_modules goto COMPILE
ECHO Installing Packages for the first time use...
CALL npm install --silent

:COMPILE
IF EXIST dist goto ALLSET
ECHO Compiling...
CALL tsc

:ALLSET
node dist/src/app.js
