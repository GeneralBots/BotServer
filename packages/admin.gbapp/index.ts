/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
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
import { AdminDialog } from './dialogs/AdminDialog.js';
import { GuaribasAdmin } from './models/AdminModel.js';
import { GBLogEx } from '../core.gbapp/services/GBLogEx.js';


/**
 * The package for admin.gbapp.
 */
export class GBAdminPackage implements IGBPackage {
  public sysPackages: IGBPackage[];

  public async getDialogs (min: GBMinInstance) {
    GBLogEx.verbose(min,`getDialogs called.`);
  }
  public async unloadPackage (core: IGBCoreService): Promise<void> {
    GBLog.verbose(`unloadPackage called.`);
  }
  public async unloadBot (min: GBMinInstance): Promise<void> {
    GBLogEx.verbose(min,`unloadBot called.`);
  }
  public async onNewSession (min: GBMinInstance, step: GBDialogStep): Promise<void> {
    GBLogEx.verbose(min, `onNewSession called.`);
  }
  public async onExchangeData (min: GBMinInstance, kind: string, data: any) {
    GBLogEx.verbose(min,`onExchangeData called.`);
  }

  public async loadPackage (core: IGBCoreService, sequelize: Sequelize): Promise<void> {
    core.sequelize.addModels([GuaribasAdmin]);
  }

  public async loadBot (min: GBMinInstance): Promise<void> {
    AdminDialog.setup(min);
  }
}
