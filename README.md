| Area                         | Status                                                                                             |
|------------------------------|----------------------------------------------------------------------------------------------------|
| Community                    | [![Gitter](https://img.shields.io/gitter/room/pragmatismo-io/GeneralBots.svg)](https://gitter.im/GeneralBots) [![Open-source](https://badges.frapsoft.com/os/v2/open-source.svg)](https://badges.frapsoft.com) [![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](http://makeapullrequest.com) [![License](https://img.shields.io/badge/license-AGPL-blue.svg)](https://github.com/pragmatismo-io/BotServer/blob/master/LICENSE.txt)|
| Management                   | [![Waffle.io - Columns and their card count](https://badge.waffle.io/pragmatismo-io/BotServer.svg?columns=all)](https://waffle.io/pragmatismo-io/BotServer) |
| Security                     | [![Known Vulnerabilities](https://snyk.io/test/github/pragmatismo-io/BotServer/badge.svg)](https://snyk.io/test/github/pragmatismo-io/BotServer) |
| Building & Quality           | [![Build Status](https://travis-ci.com/pragmatismo-io/BotServer.svg?branch=master)](https://travis-ci.com/pragmatismo-io/BotServer)  [![Coverage Status](https://coveralls.io/repos/github/pragmatismo-io/BotServer/badge.svg)](https://coveralls.io/github/pragmatismo-io/BotServer) [![code style: prettier](https://img.shields.io/badge/code_style-prettier-ff69b4.svg?style=flat-square)](https://github.com/prettier/prettier) |
| Packaging                    | [![forthebadge](https://badge.fury.io/js/botserver.svg)](https://badge.fury.io) [![Dependencies](https://david-dm.org/pragmatismo-io/botserver.svg)](https://david-dm.org)  [![Greenkeeper badge](https://badges.greenkeeper.io/pragmatismo-io/BotServer.svg)](https://greenkeeper.io/) |
| Releases                     | [![General Bots](https://img.shields.io/npm/dt/botserver.svg?logo=npm&label=botserver)](https://www.npmjs.com/package/botserver/) [![.gbapp lib](https://img.shields.io/npm/dt/botlib.svg?logo=npm&label=botlib)](https://www.npmjs.com/package/botlib/) |

#### Watch a video about packages, development environment and self-deployment

*Now with General Bots you can press F5 on Visual Studio to get a bot factory on your environment* published on November 10th, 2018.

[![General Bot Video](https://github.com/pragmatismo-io/BotServer/blob/master/docs/images/video-01-thumb.jpg)](https://www.youtube.com/watch?v=AfKTwljoMOs)


Welcome to General Bot Community Edition
----------------------------------------

![General Bot Logo](https://raw.githubusercontent.com/pragmatismo-io/BotServer/master/logo.png)

General Bot is a package based chat bot server focused in convention over configuration and code-less approaches, which brings software packages and application server concepts to help parallel bot development.
In this [MSDN](https://blogs.msdn.microsoft.com/buckwoody/2018/09/25/applied-ai-using-a-bot-for-password-reset/) article you can have an overview of a General Bots application.

*Checkout our FREE [Intranet Quickstart Bot](https://github.com/pragmatismo-io/IntranetBotQuickStart.gbai)*


### What is a Bot Server?

![General Bots Starting From Scrach](https://github.com/pragmatismo-io/BotServer/blob/master/docs/images/generalbots-open-core-starting-from-scratch.gif)

Bot Server accelerates the process of developing a bot. It provisions all code
base, resources and deployment to the cloud, and gives you templates you can
choose from whenever you need a new bot. The server has a database and service 
backend allowing you to further modify your bot package directly by downloading 
a zip file, editing and uploading it back to the server (deploying process) with 
no code. The Bot Server also provides a framework to develop bot packages in a more
advanced fashion writing custom code in editors like Visual Studio Code, Atom or Brackets.

Everyone can create bots by just copying and pasting some files and using their
favorite tools like Excel (or any text editor) or Photoshop (or any image
editor).

Package Quick Reference
------------
|Whatsapp|Web|Core|KB|
|----|-----|----|----|
|[whatsapp.gblib](https://github.com/pragmatismo-io/BotServer/tree/master/packages/whatsapp.gblib)|[default.gbui](https://github.com/pragmatismo-io/BotServer/tree/master/packages/default.gbui)|[core.gbapp](https://github.com/pragmatismo-io/BotServer/tree/master/packages/core.gbapp)|[kb.gbapp](https://github.com/pragmatismo-io/BotServer/tree/master/packages/kb.gbapp)|

![General Bot Logo](https://raw.githubusercontent.com/pragmatismo-io/BotServer/master/docs/images/general-bots-stack.png)

### The same build process for everyone

![General Bots Block Architecture](https://raw.githubusercontent.com/pragmatismo-io/BotServer/master/docs/images/general-bots-block-architecture.png)

GeneralBots aims to delivery bots in azure in a very easy and fast fashion. Use
Office tools like Word or Excel to edit your Bot - using code (JavaScript or TypeScript) just to empower custom requirements.


#### Use Excel for (Hierarchical) Knowledge Base Editing

![General Bots Inside Excel can enable bot production the masses](https://github.com/pragmatismo-io/BotServer/blob/master/docs/images/general-bots-composing-subjects-json-and-excel.gif)

#### Use Visual Studio for a complete .gbai package building system

![General Bots Inside Visual Studio Code provides a complete artificial intelligence based conversational platform](https://raw.githubusercontent.com/pragmatismo-io/BotServer/master/docs/images/general-bots-inside-visual-studio-code-provides-a-complete-artificial-intelligence-based-conversational-platform.png)


How To
------

### Run the server locally

1. Install [Node.js](https://www.npmjs.com/get-npm) the current generation General Bot code execution platform;
2. Open a **Terminal** on Linux and Mac or a **Command Prompt** window on Windows;
3. Type `npm install -g botserver` and press *ENTER*;
4. Type `gbot` to run the server core.

Notes:

* [*nodejs.install* Chocolatey Package](https://chocolatey.org/packages/nodejs.install) is also available.
* The zip source code of General Bot is also available for [Download](https://codeload.github.com/pragmatismo-io/BotServer/zip/master);

### Configure the server to deploy specific directory

1. Create/Edit the .env file and add the ADDITIONAL_DEPLOY_PATH key pointing to the .gbai local parent folder of .gbapp, .gbot, .gbtheme, .gbkb package directories.
2. Specify STORAGE_SYNC to TRUE so database sync is run when the server is run.
3. In case of Microsoft SQL Server add the following keys: STORAGE_SERVER, STORAGE_NAME, STORAGE_USERNAME, STORAGE_PASSWORD, STORAGE_DIALECT to `mssql`.

Note:

* You can specify several bots separated by semicolon, the BotServer will serve all of them at once.

## Setup development environment (Windows)

1. [Optional] Install [Chocolatey](https://chocolatey.org/install), a Windows Package Manager;
2. Install [git](`https://git-scm.com/`), a Software Configuration Management (SCM).;
3. Install [Node.js](npmjs.com/get-npm), a [Runtime system](https://en.wikipedia.org/wiki/Runtime_system).
(https://www.npmjs.com/get-npm) (suggested: LTS 8.x.x);
4. Install [Visual Studio Code](https://chocolatey.org/packages/nodejs.install), Brackets or Atom as an editor of your choice;
5. [Fork](https://en.wikipedia.org/wiki/Fork_(software_development)) by visiting https://github.com/pragmatismo-io/BotServer/fork
6. Clone the just forked repository by running `git clone <your-forked-repository-url>/BotServer.git` ;
7. Run `npm install -g typescript`;
8. Run `npm install` on Command Prompt or PowerShell on the General Bot source-code folder;
9. Enter './packages/default.gbui' folder;
10. Run `npm install` folled by `npm run build` (To build default Bot UI);
11. Enter the On the downloaded folder (../..);
12. Compile the bot server by `tsc`.
13. Run the bot server by `npm start`.

Note:

* Whenever you are ready to turn your open-source bot ideas in form of  .gbapp (source-code) and artifacts like .gbkb, .gbtheme, .gbot or the .gbai full package read [CONTRIBUTING.md](https://github.com/pragmatismo-io/BotServer/blob/master/CONTRIBUTING.md) about performing Pull Requests (PR) and creating other public  custom packages repositories of your own personal or organization General Bot Community Edition powered packages.

### Running unit tests

1. Enter the BotServer root folder.
2. Run tests by `npm test`.

### Just copy the source code to your machine

1. [Download] the Zip file of (https://codeload.github.com/pragmatismo-io/BotServer/zip/master)

### Updating the Bot Knoledge Base (.gbkb folder)

The subjects.json file contains all information related to the subject tree and can be used to build the menu carrousel as well give a set of words to be used as subject catcher in the conversation. A hierarchy can be specified.

### Creating a new Theme folder (.gbtheme folder)

A theme is composed of some CSS files and images. That set of files can change
everything in the General Bot UI. Use them extensively before going to change
the UI application itself (HTML & JS).

Package Types
-------------


### .gbai

Embraces all packages types (content, logic & conversation) into a pluggable bot
directory. [A sample .gbai is available](https://github.com/pragmatismo-io/IntranetBotQuickStart.gbai).

### .gbapp

The artificial intelligence extensions in form of pluggable apps. Dialogs,
Services and all model related to data. A set of interactions, use cases, 
integrations in form of conversationals dialogs.
The .gbapp adds the General Bot base library (botlib) for building Node.js TypeScript Apps packages.


Four components builds up a General Bot App:

* dialogs
* models
* services
* tests

#### Dialogs

All code contained in a dialog builds the flow to custom conversations in 
built-in and additional packages .


#### Models

Models builds the foundation of data relationships in form of entities.


#### Services

Services are a façade for bot back-end logic and other custom processing.

#### Tests

Tests try to automate code execution validation before crashing in production.


### .gbot

An expression of an artificial inteligence entity. A .gbot file defines 
all bots dependencies related to services and other resources.

### .gbtheme

A theme of a bot at a given time. CSS files & images that can compose all UI
presentation and using it a branding can be done. [A sample .gbtheme is available](https://github.com/pragmatismo-io/Office365.gbtheme)

### .gbkb

A set of subjects that bot knows in a form of hierarchical menu-based QnA. [A sample .gbkb is available](https://github.com/pragmatismo-io/ProjectOnline.gbkb).

### .gblib

Shared code that can be used across bot apps.

Reference
---------

### GeneralBots admin commands

General Bot can be controlled by the same chat window people talk to, so 
here is a list of admin commands related to deploying .gb* files.

| Command         | Description                                                                                                     |
|-----------------|-----------------------------------------------------------------------------------------------------------------|
| deployPackage   | Deploy a KB package. Usage **deployPackage** [package-name]. Then, you need to run rebuildIndex.                |
| undeployPackage | Undeploy a KB. Usage **undeployPackage** [package-name].                                                        |
| redeployPackage | Undeploy and then deploys the KB. Usage **redeployPackage** [package-name]. Then, you need to run rebuildIndex. |
| setupSecurity   | Setup connection to user directories.                                                                           |

Discontinued commands:

| Command         | Description                                                                                                     |Reason | 
|-----------------| -----------------------------------------------------------------------------------------------------------------|------|
| rebuildIndex    | Rebuild Azure Search indexes, must be run after **deployPackage** or **redeployPackage**.                       | Now it is called automatically | 

### Credits & Inspiration

* Rodrigo Rodriguez (me@rodrigorodriguez.com) - Coding, Docs & Architecture.
* David Lerner (david.lerner@hotmail.com) - UI, UX & Theming.
* Eduardo Romeiro (eromeirosp@outlook.com) - Content & UX.
* Jorge Ramos (jramos@pobox.com) - Coding, Docs & Architecture.
* PH Nascimento (ph.an@outlook.com) - Product Manager

Powered by  Microsoft [BOT Framework](https://dev.botframework.com/) and [Azure](http://www.azure.com).

General Bot Code Name is [Guaribas](https://en.wikipedia.org/wiki/Guaribas), the name of a city in Brasil, state of Piaui.
[Roberto Mangabeira Unger](http://www.robertounger.com/en/): "No one should have to do work that can be done by a machine".

## Contributing

This project welcomes contributions and suggestions. 
See our [Contribution Guidelines](https://github.com/pragmatismo-io/BotServer/blob/master/CONTRIBUTING.md) for more details.

## Reporting Security Issues

Security issues and bugs should be reported privately, via email, to the Pragmatismo.io Security
team at [security@pragmatismo.io](mailto:security@pragmatismo.io). You should
receive a response within 24 hours. If for some reason you do not, please follow up via
email to ensure we received your original message. 

## License & Warranty

General Bot Copyright (c) Pragmatismo.io. All rights reserved.
Licensed under the AGPL-3.0.       
                                                            
According to our dual licensing model, this program can be used either
under the terms of the GNU Affero General Public License, version 3,
or under a proprietary license.   
                                                        
The texts of the GNU Affero General Public License with an additional
permission and of our proprietary license can be found at and 
in the LICENSE file you have received along with this program.
                                                       
This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.
                                                        
"General Bot" is a registered trademark of Pragmatismo.io.
The licensing of the program under the AGPLv3 does not imply a
trademark license. Therefore any rights, title and interest in
our trademarks remain entirely with us.
