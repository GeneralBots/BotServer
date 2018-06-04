/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             | 
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

"use strict";

const UrlJoin = require("url-join");

import { GuaribasAnswer, GuaribasQuestion, GuaribasSubject } from './models/index';
import { GBMinInstance, IGBPackage } from "botlib";

import { AskDialog } from "./dialogs/AskDialog";
import { FaqDialog } from "./dialogs/FaqDialog";
import { MenuDialog } from "./dialogs/MenuDialog";
import { Sequelize } from 'sequelize-typescript';
import { IGBCoreService } from 'botlib';

export class GBKBPackage implements IGBPackage {

  sysPackages: IGBPackage[] = null;
  
  loadPackage(core: IGBCoreService, sequelize: Sequelize): void {
    core.sequelize.addModels([
      GuaribasAnswer,
      GuaribasQuestion,
      GuaribasSubject
    ]);
    
  }
  unloadPackage(core: IGBCoreService): void {
    
  }
  loadBot(min: GBMinInstance): void {

    AskDialog.setup(min.bot, min);
    FaqDialog.setup(min.bot, min);
    MenuDialog.setup(min.bot, min);
    
  }
  unloadBot(min: GBMinInstance): void {
    
  }
  onNewSession(min: GBMinInstance, dc: any): void {
    
  }
}
