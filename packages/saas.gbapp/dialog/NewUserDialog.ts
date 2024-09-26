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

'use strict';

import { IGBDialog, GBMinInstance } from 'botlib';
import { Messages } from '../strings.js';
import { MainService } from '../service/MainService.js';
import { SaaSPackage } from '../index.js';
import { CollectionUtil } from 'pragmatismo-io-framework';

export class NewUserDialog extends IGBDialog {
  static getBotNameDialog(min: GBMinInstance) {
    return {
      id: '/welcome_saas_botname',
      waterfall: [
        async step => {
          const locale = 'en-US';

          await step.prompt('textPrompt', Messages[locale].whats_botname);
        },
        async step => {
          const locale = 'en-US';

          const extractEntity = text => {
            return text.match(/[_a-zA-Z][_a-zA-Z0-9]{0,16}/gi);
          };

          const value = extractEntity(step.context.activity.originalText);

          if (value === null || value.length != 1) {
            await step.context.sendActivity(Messages[locale].validation_enter_valid_botname);

            return await step.replaceDialog('/welcome_saas_botname', step.activeDialog.state.options);
          } else {
            const botName = value[0];
            if (await min.deployService.botExists(botName)) {
              await step.context.sendActivity(`O Bot ${botName} já existe. Escolha por favor, outro nome!`);

              return await step.replaceDialog('/welcome_saas_botname', step.activeDialog.state.options);
            } else {
              step.activeDialog.state.options.botName = botName;

              return await step.replaceDialog('/welcome_saas_bottemplate', step.activeDialog.state.options);
            }
          }
        }
      ]
    };
  }

  static getBotTemplateDialog(min: GBMinInstance) {
    return {
      id: '/welcome_saas_bottemplate',
      waterfall: [
        async step => {
          const locale = 'en-US';
          await step.context.sendActivity('Aqui estão alguns modelos para você escolher:');
          let gboService = min.gbappServices['gboService'];
          const list = await gboService.listTemplates(min);

          let templateMessage = undefined;

          await CollectionUtil.asyncForEach(list, async item => {
            if (item.name !== 'Shared.gbai') {
              templateMessage = templateMessage ? `${templateMessage}\n- ${item.name}` : `- ${item.name}`;
            }
          });
          await step.context.sendActivity(templateMessage);

          step.activeDialog.state.options.templateList = list;
          return await step.prompt('textPrompt', `Qual modelo de bot você gostaria de usar?`);
        },
        async step => {
          const list = step.activeDialog.state.options.templateList;
          let template = null;
          await CollectionUtil.asyncForEach(list, async item => {
            let gboService = min.gbappServices['gboService'];
            if (gboService.kmpSearch(step.context.activity.originalText, item.name) != -1) {
              template = item.name;
            }
          });

          if (template === null) {
            await step.context.sendActivity(`Escolha, por favor, um destes modelos listados.`);

            return await step.replaceDialog('/welcome_saas_bottemplate', step.activeDialog.state.options);
          } else {
            step.activeDialog.state.options.templateName = template;
            debugger;
            await NewUserDialog.createBot(step, min, true);

            return await step.replaceDialog('/ask', { isReturning: true });
          }
        }
      ]
    };
  }

  static getReturnFromCC(min: GBMinInstance) {
    return {
      id: '/welcome_saas_return_cc',
      waterfall: [
        async step => {
          const locale = 'en-US';
          await step.context.sendActivity(Messages[locale].thanks_payment);
          await NewUserDialog.createBot(step, min, false);

          return await step.replaceDialog('/ask', { isReturning: true });
        }
      ]
    };
  }

  static getReturnFromDocument(min: GBMinInstance) {
    return {
      id: '/welcome_saas_return_document',
      waterfall: [
        async step => {
          step.activeDialog.state.options.nextDialog = 'welcome_saas_return_payment';

          return await step.replaceDialog('/bank_payment_type', step.activeDialog.state.options);
        }
      ]
    };
  }

  static getReturnFromPayment(min: GBMinInstance) {
    return {
      id: '/welcome_saas_return_payment',
      waterfall: [
        async step => {
          if (step.activeDialog.state.options.paymentType === 'cc') {
            step.activeDialog.state.options.nextDialog = 'welcome_saas_return_cc';
            await step.replaceDialog(`/bank_ccnumber`, step.activeDialog.state.options);
          } else {
            const locale = 'en-US';
            await step.context.sendActivity(Messages[locale].boleto_mail);

            await step.context.sendActivity('textPrompt', Messages[locale].thanks_payment);
            await NewUserDialog.createBot(step, min, false);

            return await step.replaceDialog('/ask', { isReturning: true });
          }
        }
      ]
    };
  }

  static getVoucherDialog(min: GBMinInstance) {
    return {
      id: '/welcome_saas_voucher',
      waterfall: [
        async step => {
          const locale = 'en-US';
          await step.prompt('textPrompt', Messages[locale].own_voucher);
        },
        async step => {
          const locale = 'en-US';

          if (step.result.toLowerCase() === 'gb2020') {
            await NewUserDialog.createBot(step, min, true);

            return await step.replaceDialog('/ask', { isReturning: true });
          } else {
            // return await step.replaceDialog('/welcome_saas_voucher', 'Os meios de pagamento estão neste momento desabilitados, por favor informe um voucher ou contate info@pragmatismo.cloud.');

            step.activeDialog.state.options.nextDialog = 'welcome_saas_return_document';
            return await step.replaceDialog('/xrm_document', step.activeDialog.state.options);
          }
        }
      ]
    };
  }

  private static async createBot(step: any, min: GBMinInstance, free: boolean) {
    const locale = 'en-US';
    await step.context.sendActivity(Messages[locale].ok_procceding_creation);
    const url = `https://gb.pragmatismo.cloud/${step.activeDialog.state.options.botName}`;
    await step.context.sendActivity(Messages[locale].bot_created(url));
    const service = new MainService();
    await service.createSubscription(
      min,
      step.activeDialog.state.options.name,
      step.activeDialog.state.options.document,
      step.activeDialog.state.options.email,
      step.activeDialog.state.options.mobile,
      step.activeDialog.state.options.botName,
      step.activeDialog.state.options.ccNumber,
      step.activeDialog.state.options.ccExpiresOnMonth,
      step.activeDialog.state.options.ccExpiresOnYear,
      step.activeDialog.state.options.ccSecuritycode,
      step.activeDialog.state.options.templateName,
      free
    );
  }

  static getDialogBatch(min: GBMinInstance) {
    return {
      id: '/welcome_saas_batch',
      waterfall: [
        async step => {
          const locale = 'en-US';
          await step.context.sendActivity(Messages[locale].welcome);

          await step.prompt('textPrompt', `Please, inform bot names separeted by comma (,).`);
        },
        async step => {
          const locale = 'en-US';

          const service = new MainService();

          const bots = step.context.activity.originalText.split(',');
          bots.forEach(async botName => {
            await service.createSubscription(
              min,
              botName,
              '999999999',
              'email@doman.cloud',
              '5521999998888',
              botName,
              null,
              '12',
              '99',
              '1234',
              'Starter.gbai',
              true
            );
            
          });


        }
      ]
    };
  }

  static getDialog(min: GBMinInstance) {
    return {
      id: '/welcome_saas',
      waterfall: [
        async step => {
          const locale = 'en-US';

          step.activeDialog.state.options.document = null;
          step.activeDialog.state.options.email = null;
          step.activeDialog.state.options.botName = null;
          step.activeDialog.state.options.ccNumber = null;
          step.activeDialog.state.options.ccExpiresOnMonth = null;
          step.activeDialog.state.options.ccExpiresOnYear = null;
          step.activeDialog.state.options.ccSecuritycode = null;
          step.activeDialog.state.options.templateName = null;

          await step.context.sendActivity(Messages[locale].welcome);

          const mobile = step.context.activity.from.id;

          step.activeDialog.state.options.nextDialog = 'welcome_saas_botname';

          if (isNaN(mobile as any)) {
            await step.context.sendActivity(Messages[locale].ok_get_information);

            return await step.replaceDialog('/profile_name', step.activeDialog.state.options);
          } else {
            const name = SaaSPackage.welcomes ? SaaSPackage.welcomes[mobile] : null;
            step.activeDialog.state.options.name = name;
            step.activeDialog.state.options.mobile = mobile;

            await step.context.sendActivity(`Olá ${name}, vamos criar o seu Bot agora.`);

            return await step.replaceDialog('/profile_email', step.activeDialog.state.options);
          }
        }
      ]
    };
  }
}
