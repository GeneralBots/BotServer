/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

"use strict"

import { IGBPackage, GBMinInstance, IGBCoreService, GBLog, IGBAdminService, GBDialogStep } from 'botlib'
import { Sequelize } from 'sequelize-typescript'
import { GBOnlineSubscription } from './model/MainModel'

import { MSSubscriptionService } from './service/MSSubscription'
import { CollectionUtil } from 'pragmatismo-io-framework';
import { NewUserDialog } from './dialog/NewUserDialog'
const MicrosoftGraph = require("@microsoft/microsoft-graph-client");

const cron = require('node-cron');

export class Package implements IGBPackage {
  sysPackages: IGBPackage[]
  adminService: IGBAdminService;
  public static welcomes = {};
  instanceId: any

  public getDialogs(min: GBMinInstance) {
    return [NewUserDialog.getDialog(min),
    NewUserDialog.getBotNameDialog(min),
    NewUserDialog.getVoucherDialog(min),
    NewUserDialog.getBotTemplateDialog(min),
    NewUserDialog.getReturnFromPayment(min),
    NewUserDialog.getReturnFromCC(min),
    NewUserDialog.getReturnFromDocument(min),
    NewUserDialog.getDialogBatch(min)
    ];
  }

  async loadPackage(core: IGBCoreService, sequelize: Sequelize): Promise<void> {
    sequelize.addModels([GBOnlineSubscription]);
    core.setWWWRoot(process.env.SAAS_WWWROOT);
    core.setEntryPointDialog('/welcome_saas');

    // Installs webhook for Microsoft intercommunication.

    core.installWebHook(true, '/mslanding', async (req, res) => {
      const service = new MSSubscriptionService();
      await service.handleMSLanding(req, res);
    });
    core.installWebHook(true, '/mshook', async (req, res) => {
      const service = new MSSubscriptionService();
      await service.handleMSHook(req, res);
    });
    core.installWebHook(true, '/signup', async (req, res) => {
      const service = new MSSubscriptionService();
      await service.handleMSSignUp(req, res);
    });


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

    cron.schedule(schedule, async () => {
      GBLog.info( 'Sending Tasks notifications if applies...');
      await this.notifyJob(sendToDevice);
    }, options);
    GBLog.info( 'Running Reviews notifications 09:30 from Monday to Friday...');
  }

  /**
 * Called by scheduler to send notification message to phones.
 * @param sendToDevice The function used to notify.
 */
  private async notifyJob(sendToDevice) {

  }



  public async getTasksMarkdown(packagePath, file) {

    let token =
      await this.adminService.acquireElevatedToken(this.instanceId);

    let client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });
    let siteId = process.env.STORAGE_SITE_ID;
    let libraryId = process.env.STORAGE_LIBRARY;

    let res = await client.api(
      `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/root:${packagePath}:/children`)
      .get();

    let document = res.value.filter(m => {
      return m.name === file
    });

    let results = await client.api(
      `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/items/${document[0].id}/workbook/worksheets('Tasks')/range(address='A1:A10')`)
      .get();

    let index = 1;
    let md = `*Tasks*\n`;

    for (; index <= 10; index++) {
      const row = results.text[index];
      if (row !== undefined) {
        md = `${md}\n *${index}*. ${row[0]}`;
      }
    }

    return md;
  }

  async unloadPackage(core: IGBCoreService): Promise<void> {

  }

  async loadBot(min: GBMinInstance): Promise<void> {

    let gboService = min.gbappServices['gboService'];


    // Gets the sendToDevice method of whatsapp.gblib and setups scheduler.

    if (min.whatsAppDirectLine !== undefined && min.botId === "Pragmatismo") {
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
        Package.welcomes[from] = fromName;
        break;

      default:
        GBLog.verbose('saas.gbapp onExchangeData called');
        break;
    }
  }

}
