/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

"use strict"

import { IGBPackage, GBMinInstance, IGBCoreService, GBLog, IGBAdminService, GBDialogStep } from 'botlib'
import { Sequelize } from 'sequelize-typescript'
import { GBOnlineSubscription } from './model/MainModel.js'

import { CollectionUtil } from 'pragmatismo-io-framework';
import { NewUserDialog } from './dialog/NewUserDialog.js'
import { GBOService } from './service/GBOService.js'

export class SaaSPackage implements IGBPackage {
  sysPackages: IGBPackage[]
  adminService: IGBAdminService;
  public static welcomes = {};
  instanceId: any

  public getDialogs(min: GBMinInstance) {
    return [NewUserDialog.getDialog(min),
    NewUserDialog.getBotNameDialog(min),
    NewUserDialog.getBotTemplateDialog(min),
    
    NewUserDialog.getPlanSelectionDialog(min),
    
    ];
  }

  async loadPackage(core: IGBCoreService, sequelize: Sequelize): Promise<void> {
    sequelize.addModels([GBOnlineSubscription]);
    
    core.setEntryPointDialog('/welcome_saas');


  }

  /**
   * Setups schedule to trigger notifications as pro-active messages.
   */
  private setupScheduler(min, sendToDevice) {
    const schedule = '30 09 * * 1-5';
    const options = {
      scheduled: true,
      timezone: 'America/Sao_Paulo'
    };

    this.adminService = min.adminService;
    this.instanceId = min.instanceId;

  }

  /**
 * Called by scheduler to send notification message to phones.
 * @param sendToDevice The function used to notify.
 */
  private async notifyJob(sendToDevice) {

  }


  async unloadPackage(core: IGBCoreService): Promise<void> {

  }

  async loadBot(min: GBMinInstance): Promise<void> {

    let gboService = new GBOService();

    // Gets the sendToDevice method of whatsapp.gblib and setups scheduler.

    if (min.whatsAppDirectLine !== undefined) {
      const sendToDevice = min.whatsAppDirectLine.sendToDevice.bind(min.whatsAppDirectLine);
      this.setupScheduler(min, sendToDevice);
      this.notifyJob(sendToDevice);
    }
  }

  async unloadBot(min: GBMinInstance): Promise<void> {

  }

  async onNewSession(min: GBMinInstance, step: GBDialogStep): Promise<void> {

  }

  public async onExchangeData(min: GBMinInstance, kind: string, data: any) {

    switch (kind) {
      case "whatsappMessage":

        const from = data.from;
        const fromName = data.fromName;
        SaaSPackage.welcomes[from] = fromName;
        break;

      default:
        GBLog.verbose('saas.gbapp onExchangeData called');
        break;
    }
  }

}
