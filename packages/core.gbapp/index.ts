/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
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
import { BroadcastDialog } from './dialogs/BroadcastDialog.js';
import { LanguageDialog } from './dialogs/LanguageDialog.js';
import { SwitchBotDialog } from './dialogs/SwitchBot.js';
import { WelcomeDialog } from './dialogs/WelcomeDialog.js';
import { WhoAmIDialog } from './dialogs/WhoAmIDialog.js';
import { GuaribasApplications, GuaribasChannel, GuaribasInstance, GuaribasLog, GuaribasPackage } from './models/GBModel.js';

/**
 * Package for core.gbapp.
 */
export class GBCorePackage implements IGBPackage {
  public sysPackages: IGBPackage[];
  public CurrentEngineName = 'guaribas-1.0.0';

  public async loadPackage (core: IGBCoreService, sequelize: Sequelize): Promise<void> {
    core.sequelize.addModels([GuaribasInstance, GuaribasPackage, GuaribasChannel, GuaribasLog, GuaribasApplications]);
  }

  public async getDialogs (min: GBMinInstance) {
    GBLog.verbose(`getDialogs called.`);
  }
  public async unloadPackage (core: IGBCoreService): Promise<void> {
    GBLog.verbose(`unloadPackage called.`);
  }
  public async unloadBot (min: GBMinInstance): Promise<void> {
    GBLog.verbose(`unloadBot called.`);
  }
  public async onNewSession (min: GBMinInstance, step: GBDialogStep): Promise<void> {
    GBLog.verbose(`onNewSession called.`);
  }
  public async onExchangeData (min: GBMinInstance, kind: string, data: any) {
    GBLog.verbose(`onExchangeData called.`);
  }

  public async loadBot (min: GBMinInstance): Promise<void> {
    WelcomeDialog.setup(min.bot, min);
    WhoAmIDialog.setup(min.bot, min);
    SwitchBotDialog.setup(min.bot, min);
    BroadcastDialog.setup(min.bot, min);
    LanguageDialog.setup(min.bot, min);
  }
}
