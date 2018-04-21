General Bots Community Edition Preview
====================================

Welcome to General Bots!
-------

General Bots is a package based chat bot server focused in convention
over configuration and code-less approaches, which brings software packages  
and application server concepts to help parallel bot development.

Also, everyone can create bots copying and pasting some files and using their
favorite tools like Excel (or any .tsv editor) or Photoshop (or any .png
editor).

### What is Bot Server?

Bot Server accelerates the process of developing a bot. It provisions all code
base, resources and deployment to the cloud, and gives you templates you can
choose from when you create a bot. Uses a database and tables as backend and
allow you to further modify your bot package directly downloading it in a ZIP
file and editing it and uploading it back to the server (deploying process). 
Besides providing a framework to develop bot packages in a more advanced 
editor like Visual Studio Code, Atom or Brackets.

### The same build process for everyone

GeneralBots aims to delivery bots in azure in a very easy and fast fashion. Use
Office tools like Word or Excel to edit your Bot - using code (JavaScript or TypeScript) just to empower custom requirements.

How To
------


### Updating the Bot Knoledge Base (.gbkb folder)


The subjects.json file contains all information related to the subject tree and can be used to build the menu carrousel as well give a set of words to be used as subject catcher in the conversation. A hierarchy can be specified.


### Creating a new Theme folder (.gbtheme folder)

A theme is composed of some CSS files and images. That set of files can change
everything in the General Bots UI. Use them extensively before going to change
the UI application itself (HTML & JS).

Package Types
-------------

### .gbai

Embraces all packages types (content, logic & conversation) into a pluggable bot
directory.

### .gbapp

The artificial intelligence extensions in form of pluggable apps. Dialogs,
Services and all model related to data. A set of interactions, use cases, 
integrations in form of conversationals dialogs.
The .gbapp adds the General Bots base library (botlib) for building Node.js TypeScript Apps packages.


Four components builds up a General Bots App:

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
presentation and using it a branding can be done.

### .gbkb

A set of subjects that bot knows.


### .gblib

Shared code that can be used across bot apps.

Reference
---------

### GeneralBots admin commands

General Bots can be controlled by the same chat window people talk to, so 
here is a list of admin commands related to deploying .gb* files.

| Command         | Description                                                                                                     |
|-----------------|-----------------------------------------------------------------------------------------------------------------|
| deployPackage   | Deploy a KB package. Usage **deployPackage** [package-name]. Then, you need to run rebuildIndex.                |
| undeployPackage | Undeploy a KB. Usage **undeployPackage** [package-name].                                                        |
| redeployPackage | Undeploy and then deploys the KB. Usage **redeployPackage** [package-name]. Then, you need to run rebuildIndex. |
| rebuildIndex    | Rebuild Azure Search indexes, must be run after **deployPackage** or **redeployPackage**.                       |

### Credits & Inspiration

* Rodrigo Rodriguez (me@rodrigorodriguez.com) - Coding, Docs & Architecture.
* David Lerner (david.lerner@hotmail.com) - UI, UX & Theming
* Eduardo Romeiro (eromeirosp@outlook.com) - Content & UX


Powered by  Microsoft [BOT Framework](https://dev.botframework.com/) and [Azure](http://www.azure.com).

General Bots Code Name is [Guaribas](https://en.wikipedia.org/wiki/Guaribas), the name of a city in Brasil, state of Piaui.
[Roberto Mangabeira Unger](http://www.robertounger.com/en/): "No one should have to do work that can be done by a machine".


## License & Warranty

 General Bots Copyright (c) Pragmatismo.io. All rights reserved.       
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
                                                        
 "General Bots" is a registered trademark of Pragmatismo.io.           
 The licensing of the program under the AGPLv3 does not imply a        
 trademark license. Therefore any rights, title and interest in        
 our trademarks remain entirely with us.                               
                                                        
