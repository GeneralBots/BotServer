// BotServer/packages/saas.gbapp/dialog/NewUserDialog.ts
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

'use strict';

import { IGBDialog, GBMinInstance } from 'botlib';
import { Messages } from '../strings.js';
import { MainService } from '../service/MainService.js';
import { SaaSPackage } from '../index.js';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBOService } from '../service/GBOService.js';

export class NewUserDialog extends IGBDialog {
  static getPlanSelectionDialog(min: GBMinInstance) {
    return {
      id: '/welcome_saas_plan',
      waterfall: [
        async step => {
          const locale = 'en-US';
          await step.context.sendActivity('Please choose your plan:');
          await step.context.sendActivity('1. Personal - $9.99/month (basic features)');
          await step.context.sendActivity('2. Professional - $29.99/month (advanced features)');
          return await step.prompt('textPrompt', 'Enter 1 or 2 to select your plan:');
        },
        async step => {
          const planChoice = step.context.activity.text.trim();
          if (planChoice === '1') {
            step.activeDialog.state.options.planId = 'personal';
            step.activeDialog.state.options.amount = 9.99;
          } else if (planChoice === '2') {
            step.activeDialog.state.options.planId = 'professional';
            step.activeDialog.state.options.amount = 29.99;
          } else {
            await step.context.sendActivity('Invalid choice. Please select 1 or 2.');
            return await step.replaceDialog('/welcome_saas_plan');
          }
          return await step.replaceDialog('/welcome_saas_botname', step.activeDialog.state.options);
        }
      ]
    };
  }

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
              await step.context.sendActivity(`The Bot ${botName} already exists. Please choose another name!`);
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

  static getStripePaymentDialog(min: GBMinInstance) {
    return {
      id: '/welcome_saas_stripe_payment',
      waterfall: [
        async step => {
          const locale = 'en-US';
          await step.context.sendActivity(`Please enter your credit card details for the ${step.activeDialog.state.options.planId} plan ($${step.activeDialog.state.options.amount}/month):`);
          return await step.prompt('textPrompt', 'Card number (e.g., 4242424242424242):');
        },
        async step => {
          step.activeDialog.state.options.ccNumber = step.context.activity.text.trim();
          return await step.prompt('textPrompt', 'Expiration month (MM):');
        },
        async step => {
          step.activeDialog.state.options.ccExpiresOnMonth = step.context.activity.text.trim();
          return await step.prompt('textPrompt', 'Expiration year (YYYY):');
        },
        async step => {
          step.activeDialog.state.options.ccExpiresOnYear = step.context.activity.text.trim();
          return await step.prompt('textPrompt', 'CVC:');
        },
        async step => {
          step.activeDialog.state.options.ccSecuritycode = step.context.activity.text.trim();
          await step.context.sendActivity('Processing payment...');
          await NewUserDialog.createBot(step, min, false);
          return await step.replaceDialog('/ask', { isReturning: true });
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
          let gboService = new GBOService();
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
          let gboService = new GBOService();
          await CollectionUtil.asyncForEach(list, async item => {
            
            if (gboService.kmpSearch(step.context.activity.originalText, item.name) != -1) {
              template = item.name;
            }
          });

          if (template === null) {
            await step.context.sendActivity(`Escolha, por favor, um destes modelos listados.`);

            return await step.replaceDialog('/welcome_saas_bottemplate', step.activeDialog.state.options);
          } else {
            step.activeDialog.state.options.templateName = template;
            
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

  private static async createBot(step: any, min: GBMinInstance, free: boolean) {
    const locale = 'en-US';
    await step.context.sendActivity(Messages[locale].ok_procceding_creation);
    const url = `${process.env.BOT_ID}/${step.activeDialog.state.options.botName}`;
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
      free, step.activeDialog.state.options.planId,
    );
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
          step.activeDialog.state.options.planId = null;
          step.activeDialog.state.options.amount = null;

          await step.context.sendActivity(Messages[locale].welcome);

          const mobile = step.context.activity.from.id;

          step.activeDialog.state.options.nextDialog = 'welcome_saas_plan';

          if (isNaN(mobile as any)) {
            await step.context.sendActivity(Messages[locale].ok_get_information);
            return await step.replaceDialog('/profile_name', step.activeDialog.state.options);
          } else {
            const name = SaaSPackage.welcomes ? SaaSPackage.welcomes[mobile] : null;
            step.activeDialog.state.options.name = name;
            step.activeDialog.state.options.mobile = mobile;

            await step.context.sendActivity(`Hello ${name}, let's create your Bot now.`);
            return await step.replaceDialog('/profile_email', step.activeDialog.state.options);
          }
        }
      ]
    };
  }
}