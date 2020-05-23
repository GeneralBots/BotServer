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

const crypto = require('crypto');
import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBMinInstance, IGBDialog, GBLog } from 'botlib';
import urlJoin = require('url-join');
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { GBImporter } from '../../core.gbapp/services/GBImporterService';
import { Messages } from '../strings';
import { GBAdminService } from '../services/GBAdminService';
import { CollectionUtil } from 'pragmatismo-io-framework';


/**
 * Dialogs for administration tasks.
 */
export class AdminDialog extends IGBDialog {


  public static isIntentYes(locale, utterance) {
    return utterance.toLowerCase().match(Messages[locale].affirmative_sentences);
  }

  public static isIntentNo(locale, utterance) {
    return utterance.toLowerCase().match(Messages[locale].negative_sentences);
  }

  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  public static setup(min: GBMinInstance) {
    // Setup services.

    const importer = new GBImporter(min.core);
    const deployer = new GBDeployer(min.core, importer);
    const adminService = new GBAdminService(min.core);

    AdminDialog.setupSecurityDialogs(min);

    min.dialogs.add(
      new WaterfallDialog('/admin', [
        async step => {
          const locale = step.context.activity.locale;
          const prompt = Messages[locale].authenticate;

          return await min.conversationalService.prompt(min, step, prompt);
        },
        async step => {
          const locale = step.context.activity.locale;
          const sensitive = step.result;

          if (sensitive === min.instance.adminPass) {
            await min.conversationalService.sendText(min, step, Messages[locale].welcome);

            return await min.conversationalService.prompt(min, step, Messages[locale].which_task);
          } else {
            await min.conversationalService.sendText(min, step, Messages[locale].wrong_password);

            return await step.endDialog();
          }
        },
        async step => {
          const locale: string = step.context.activity.locale;
          // tslint:disable-next-line:no-unsafe-any
          const text: string = step.result;
          const cmdName = text.split(' ')[0];

          await min.conversationalService.sendText(min, step, Messages[locale].working(cmdName));
          let unknownCommand = false;

          try {


            if (text === 'quit') {
              return await step.replaceDialog('/');
            } else if (cmdName === 'deployPackage' || cmdName === 'dp') {
              await GBAdminService.deployPackageCommand(min, text, deployer);

              return await step.replaceDialog('/admin', { firstRun: false });
            } else if (cmdName === 'redeployPackage' || cmdName === 'rp') {
              await min.conversationalService.sendText(min, step, 'The package is being *unloaded*...');
              await GBAdminService.undeployPackageCommand(text, min);
              await min.conversationalService.sendText(min, step, 'Now, *deploying* package...');
              await GBAdminService.deployPackageCommand(min, text, deployer);
              await min.conversationalService.sendText(min, step, 'Package deployed. Just need to rebuild the index... Doing it right now.');
              await GBAdminService.rebuildIndexPackageCommand(min, deployer);
              await min.conversationalService.sendText(min, step, 'Finished importing of that .gbkb package. Thanks.');
              return await step.replaceDialog('/admin', { firstRun: false });
            } else if (cmdName === 'undeployPackage' || cmdName === 'up') {
              await min.conversationalService.sendText(min, step, 'The package is being *undeployed*...');
              await GBAdminService.undeployPackageCommand(text, min);
              await min.conversationalService.sendText(min, step, 'Package *undeployed*.');
              return await step.replaceDialog('/admin', { firstRun: false });
            } else if (cmdName === 'rebuildIndex' || cmdName === 'ri') {
              await GBAdminService.rebuildIndexPackageCommand(min, deployer);

              return await step.replaceDialog('/admin', { firstRun: false });
            } else if (cmdName === 'syncBotServer') {
              await GBAdminService.syncBotServerCommand(min, deployer);

              return await step.replaceDialog('/admin', { firstRun: false });
            } else if (cmdName === 'setupSecurity') {
              return await step.beginDialog('/setupSecurity');
            } else {
              unknownCommand = true;
            }

            if (unknownCommand) {
              await min.conversationalService.sendText(min, step, Messages[locale].unknown_command);
            } else {
              await min.conversationalService.sendText(min, step, Messages[locale].finished_working);
            }

          } catch (error) {
            await min.conversationalService.sendText(min, step, error.message);
          }
          await step.replaceDialog('/ask', { isReturning: true });
        }

      ])
    );

    min.dialogs.add(
      new WaterfallDialog('/publish', [
        async step => {
          if (step.activeDialog.state.options.confirm) {
            return await step.next('sim');
          }
          else {
            const locale = step.context.activity.locale;
            return await min.conversationalService.prompt(min, step, Messages[locale].publish_type_yes);
          }
        },
        async step => {
          const locale = step.context.activity.locale;

          // If the user says yes, starts publishing.

          if (AdminDialog.isIntentYes(locale, step.result)) {

            let from = step.context.activity.from.id;

            let canPublish = AdminDialog.canPublish(min, from);              

            if (canPublish) {

              const botId = min.instance.botId;
              const locale = step.context.activity.locale;
              await min.conversationalService.sendText(min, step, Messages[locale].working('Publishing'));

              step.activeDialog.state.options.args = (step.options as any).args;
              let args = step.activeDialog.state.options.args.split(' ');
              let filename = args[0];
              const packages = [];
              if (filename === null || filename === "") {
                await min.conversationalService.sendText(min, step, `Starting publishing for ${botId}.gbkb...`);
                packages.push(`${botId}.gbkb`);
              } else {
                await min.conversationalService.sendText(min, step, `Starting publishing for ${filename}...`);
                packages.push(filename);
              }

              try {

                await CollectionUtil.asyncForEach(packages, async packageName => {

                  const cmd1 = `deployPackage ${process.env.STORAGE_SITE} /${process.env.STORAGE_LIBRARY}/${botId}.gbai/${packageName}`;

                  if (await (deployer as any).getStoragePackageByName(min.instance.instanceId,
                    packageName) !== null) { 
                    const cmd2 = `undeployPackage ${packageName}`;
                    await GBAdminService.undeployPackageCommand(cmd2, min);
                  }
                  await GBAdminService.deployPackageCommand(min, cmd1, deployer);
                  await min.conversationalService.sendText(min, step, `Finished publishing ${packageName}.`);
                });
              } catch (error) {
                await min.conversationalService.sendText(min, step, `ERROR: ${error}` );
                GBLog.error(error);
                return await step.replaceDialog('/ask', { isReturning: true });
              }
              await min.conversationalService.sendText(min, step, Messages[locale].publish_success);
              if (!step.activeDialog.state.options.confirm) {
                return await step.replaceDialog('/ask', { isReturning: true });
              }
              else {
                return await step.endDialog();
              }

            } else {
              await min.conversationalService.sendText(min, step, Messages[locale].publish_must_be_admin);
            }
          }
          await min.conversationalService.sendText(min, step, Messages[locale].publish_canceled);
        }]));
  }


  /**
* Check if the specified phone can receive a message by running 
* the /broadcast command with specific phone numbers.
* @param phone Phone number to check (eg.: +5521900002233)
*/
  public static canPublish(min: GBMinInstance, phone: string): Boolean {

    const list = process.env.SECURITY_CAN_PUBLISH.split(';');
    let result = list.includes(phone);

    if (!result && min.instance.params) {
      const params = JSON.parse(min.instance.params);
      return list.includes(params['Can Publish']);
    }
    return result;
  }
  
  private static setupSecurityDialogs(min: GBMinInstance) {
    min.dialogs.add(
      new WaterfallDialog('/setupSecurity', [
        async step => {
          const locale = step.context.activity.locale;
          const prompt = Messages[locale].enter_authenticator_tenant;

          return await min.conversationalService.prompt(min, step, prompt);
        },
        async step => {
          step.activeDialog.state.authenticatorTenant = step.result;
          const locale = step.context.activity.locale;
          const prompt = Messages[locale].enter_authenticator_authority_host_url;

          return await min.conversationalService.prompt(min, step, prompt);
        },
        async step => {
          step.activeDialog.state.authenticatorAuthorityHostUrl = step.result;

          await min.adminService.updateSecurityInfo(
            min.instance.instanceId,
            step.activeDialog.state.authenticatorTenant,
            step.activeDialog.state.authenticatorAuthorityHostUrl
          );

          const locale = step.context.activity.locale;
          const buf = Buffer.alloc(16);
          const state = `${min.instance.instanceId}${crypto.randomFillSync(buf).toString('hex')}`;

          min.adminService.setValue(min.instance.instanceId, 'AntiCSRFAttackState', state);

          const url = `https://login.microsoftonline.com/${
            min.instance.authenticatorTenant
            }/oauth2/authorize?client_id=${min.instance.marketplaceId}&response_type=code&redirect_uri=${urlJoin(
              min.instance.botEndpoint,
              min.instance.botId,
              '/token'
            )}&state=${state}&response_mode=query`;

          await min.conversationalService.sendText(min, step, Messages[locale].consent(url));

          return await step.replaceDialog('/ask', { isReturning: true });
        }
      ])
    );
  }
}
