/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.             |
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
import { OAuthDialog } from './dialogs/OAuthDialog.js';
import { ProfileDialog } from './dialogs/ProfileDialog.js';
import { GuaribasGroup, GuaribasUser, GuaribasUserGroup } from './models/index.js';
import { SMSAuthDialog } from './dialogs/SMSAuthDialog.js';

/**
 * Package for the security module.
 */
export class GBSecurityPackage implements IGBPackage {
  public sysPackages: IGBPackage[];
  public async getDialogs(min: GBMinInstance) {
    const out = [
      ProfileDialog.getNameDialog(min),
      ProfileDialog.getEmailDialog(min),
      ProfileDialog.getMobileDialog(min),
      ProfileDialog.getMobileConfirmDialog(min),
      SMSAuthDialog.getSMSAuthDialog(min)
    ];

    if (process.env.ENABLE_AUTH) {
      out.push(OAuthDialog.getOAuthDialog(min));
    }
    return out;
  }
  public async unloadPackage(core: IGBCoreService): Promise<void> {
    GBLog.verbose(`unloadPackage called.`);
  }
  public async loadBot(min: GBMinInstance): Promise<void> {
    GBLog.verbose(`loadBot called.`);
  }
  public async unloadBot(min: GBMinInstance): Promise<void> {
    GBLog.verbose(`unloadBot called.`);
  }
  public async onNewSession(min: GBMinInstance, step: GBDialogStep): Promise<void> {
    GBLog.verbose(`onNewSession called.`);
  }
  public async onExchangeData(min: GBMinInstance, kind: string, data: any) {
    GBLog.verbose(`onExchangeData called.`);
  }

  public async loadPackage(core: IGBCoreService, sequelize: Sequelize): Promise<void> {
    core.sequelize.addModels([GuaribasGroup, GuaribasUser, GuaribasUserGroup]);
  }
}
