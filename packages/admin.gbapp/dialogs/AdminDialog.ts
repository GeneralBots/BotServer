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
| but WITHOUT ANY WARRANTY without even the implied warranty of               |
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

const UrlJoin = require('url-join');
import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBMinInstance } from 'botlib';
import { IGBDialog } from 'botlib';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { GBImporter } from '../../core.gbapp/services/GBImporterService';
import { GBAdminService } from '../services/GBAdminService';
import { Messages } from '../strings';

/**
 * Dialogs for administration tasks.
 */
export class AdminDialog extends IGBDialog {
  public static async createFarmCommand(text: any, min: GBMinInstance) {}

  public static async undeployPackageCommand(text: any, min: GBMinInstance) {
    const packageName = text.split(' ')[1];
    const importer = new GBImporter(min.core);
    const deployer = new GBDeployer(min.core, importer);
    await deployer.undeployPackageFromLocalPath(min.instance, UrlJoin('packages', packageName));
  }

  public static async deployPackageCommand(min: GBMinInstance, text: string, deployer: GBDeployer) {
    const packageName = text.split(' ')[1];
    const additionalPath = GBConfigService.get('ADDITIONAL_DEPLOY_PATH');
    await deployer.deployPackageFromLocalPath(min, UrlJoin(additionalPath, packageName));
  }

  public static async rebuildIndexPackageCommand(min: GBMinInstance, text: string, deployer: GBDeployer) {
    await deployer.rebuildIndex(min.instance);
  }

  public static async   addConnectionCommand(min: GBMinInstance, text: any) {
    const packageName = text.split(' ')[1];
    const importer = new GBImporter(min.core);
    const admin = new GBAdminService(min.core);
    // TODO: await admin.addConnection
  }


  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  public static setup(bot: BotAdapter, min: GBMinInstance) {
    // Setup services.

    const importer = new GBImporter(min.core);
    const deployer = new GBDeployer(min.core, importer);

    min.dialogs.add(
      new WaterfallDialog('/admin', [
        async step => {
          const locale = step.context.activity.locale;
          const prompt = Messages[locale].authenticate;

          return await step.prompt('textPrompt', prompt);
        },
        async step => {
          const locale = step.context.activity.locale;
          const password = step.result;

          if (password === GBConfigService.get('ADMIN_PASS')) {
            await step.context.sendActivity(Messages[locale].welcome);

            return await step.prompt('textPrompt', Messages[locale].which_task);
          } else {
            await step.context.sendActivity(Messages[locale].wrong_password);

            return await step.endDialog();
          }
        },
        async step => {
          const locale = step.context.activity.locale;
          const text = step.result;
          const cmdName = text.split(' ')[0];

          step.context.sendActivity(Messages[locale].working(cmdName));
          let unknownCommand = false;

          if (text === 'quit') {
            return await step.replaceDialog('/');
          } else if (cmdName === 'createFarm') {
            await AdminDialog.createFarmCommand(text, deployer);

            return await step.replaceDialog('/admin', { firstRun: false });
          } else if (cmdName === 'deployPackage') {
            await AdminDialog.deployPackageCommand(min, text, deployer);

            return await step.replaceDialog('/admin', { firstRun: false });
          } else if (cmdName === 'redeployPackage') {
            await AdminDialog.undeployPackageCommand(text, min);
            await AdminDialog.deployPackageCommand(min, text, deployer);

            return await step.replaceDialog('/admin', { firstRun: false });
          } else if (cmdName === 'rebuildIndex') {
            await AdminDialog.rebuildIndexPackageCommand(min, text, deployer);

            return await step.replaceDialog('/admin', { firstRun: false });
          } else if (cmdName === 'addConnection') {
            await AdminDialog.addConnectionCommand(min, text);
            return await step.replaceDialog('/admin', { firstRun: false });
          } else if (cmdName === 'undeployPackage') {
            await AdminDialog.undeployPackageCommand(text, min);

            return await step.replaceDialog('/admin', { firstRun: false });
          } else if (cmdName === 'setupSecurity') {
            await AdminDialog.setupSecurity(min, step);
          } else {
            unknownCommand = true;
          }

          if (unknownCommand) {
            await step.context.sendActivity(Messages[locale].unknown_command);
          } else {
            await step.context.sendActivity(Messages[locale].finshed_working(cmdName));
          }
          await step.endDialog();

          return await step.replaceDialog('/answer', { query: text });
        }
      ])
    );
  }

  private static async setupSecurity(min: any, step: any) {
    const locale = step.activity.locale;
    const state = `${min.instance.instanceId}${Math.floor(Math.random() * 1000000000)}`;
    await min.adminService.setValue(min.instance.instanceId, 'AntiCSRFAttackState', state);
    const url = `https://login.microsoftonline.com/${min.instance.authenticatorTenant}/oauth2/authorize?client_id=${
      min.instance.authenticatorClientId
    }&response_type=code&redirect_uri=${min.instance.botEndpoint}/${
      min.instance.botId
    }/token&state=${state}&response_mode=query`;

    await step.sendActivity(Messages[locale].consent(url));
  }
}
