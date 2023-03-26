@ECHO off

ECHO General Bots Command Line

IF EXIST node_modules goto COMPILE
ECHO Installing Packages for the first time use (it may take several minutes)...
CALL npm install --silent

:COMPILE
IF EXIST dist goto ALLSET
ECHO Compiling...
npm run build

:ALLSET
npm run start
