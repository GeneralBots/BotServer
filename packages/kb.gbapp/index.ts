/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
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
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import { GBDialogStep, GBLog, GBMinInstance, IGBCoreService, IGBPackage } from 'botlib';
import { Sequelize } from 'sequelize-typescript';
import { AskDialog } from './dialogs/AskDialog';
import { FaqDialog } from './dialogs/FaqDialog';
import { MenuDialog } from './dialogs/MenuDialog';
import { GuaribasAnswer, GuaribasQuestion, GuaribasSubject } from './models/index';

/**
 * Package for kb.gbapp.
 */
export class GBKBPackage implements IGBPackage {
  public sysPackages: IGBPackage[];
  public async getDialogs(min: GBMinInstance) {
    GBLog.verbose(`getDialogs called.`);
  }
  public async unloadPackage(core: IGBCoreService): Promise<void> {
    GBLog.verbose(`unloadPackage called.`);
  }
  public async unloadBot(min: GBMinInstance): Promise<void> {
    GBLog.verbose(`unloadBot called.`);
  }
  public async onNewSession(min: GBMinInstance, step: GBDialogStep): Promise<void> {
    GBLog.verbose(`onNewSession called.`);
  }

  public async loadPackage(core: IGBCoreService, sequelize: Sequelize): Promise<void> {
    core.sequelize.addModels([GuaribasAnswer, GuaribasQuestion, GuaribasSubject]);
  }
  public async loadBot(min: GBMinInstance): Promise<void> {
    AskDialog.setup(min.bot, min);
    FaqDialog.setup(min.bot, min);
    MenuDialog.setup(min.bot, min);
  }
}
