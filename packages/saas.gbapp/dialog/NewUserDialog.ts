// BotServer/packages/saas.gbapp/dialog/NewUserDialog.ts
import { IGBDialog, GBMinInstance } from 'botlib-legacy';
import { Messages } from '../strings.js';
import { MainService } from '../service/MainService.js';
import { SaaSPackage } from '../index.js';

import { GBOService } from '../service/GBOService.js';
import { GBUtil } from '../../../src/util.js';

export class NewUserDialog extends IGBDialog {
  static getPlanSelectionDialog(min: GBMinInstance) {
    return {
      id: '/welcome_saas_plan',
      waterfall: [
        async step => {
          await step.context.sendActivity('Please choose your plan:');
          await step.context.sendActivity('1  Free - $0/month (basic features)');
          await step.context.sendActivity('2  Personal - $50/month (more features)');
          await step.context.sendActivity('3  Professional - $150/month (advanced features)');
          return await step.prompt('textPrompt', 'Enter 1, 2 or 3 to select your plan:');
        },
        async step => {
          const planChoice = step.context.activity.text.trim();
          if (planChoice === '1') {
            step.activeDialog.state.options.planId = 'free';
          } else if (planChoice === '2') {
            step.activeDialog.state.options.planId = 'personal';
          } else if (planChoice === '3') {
            step.activeDialog.state.options.planId = 'professional';
          } else {
            await step.context.sendActivity('Invalid choice. Please select 1, 2 or 3.');
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

  static getBotTemplateDialog(min: GBMinInstance) {
    return {
      id: '/welcome_saas_bottemplate',
      waterfall: [
        async step => {
          await step.context.sendActivity('Here are some templates to choose from:');
          let gboService = new GBOService();
          const list = await gboService.listTemplates(min);

          let templateMessage = undefined;
          await GBUtil.asyncForEach(list, async item => {
            if (item.name !== 'Shared.gbai') {
              templateMessage = templateMessage ? `${templateMessage}\n- ${item.name}` : `- ${item.name}`;
            }
          });
          await step.context.sendActivity(templateMessage);

          step.activeDialog.state.options.templateList = list;
          return await step.prompt('textPrompt', `Which bot template would you like to use?`);
        },
        async step => {
          const list = step.activeDialog.state.options.templateList;
          let template = null;
          let gboService = new GBOService();
          await GBUtil.asyncForEach(list, async item => {
            if (gboService.kmpSearch(step.context.activity.originalText, item.name) != -1) {
              template = item.name;
            }
          });

          if (template === null) {
            await step.context.sendActivity(`Please choose one of the listed templates.`);
            return await step.replaceDialog('/welcome_saas_bottemplate', step.activeDialog.state.options);
          } else {
            step.activeDialog.state.options.templateName = template;

            const service = new MainService();
            const result: any = await service.startSubscriptionProcess(
              min,
              step.activeDialog.state.options.name,
              step.activeDialog.state.options.email,
              step.activeDialog.state.options.mobile,
              step.activeDialog.state.options.botName,
              template,
              step.activeDialog.state.options.planId
            );

            if (step.activeDialog.state.options.planId === 'free') {
              await step.context.sendActivity(`Your free bot has been created! Access it here: ${result.botUrl}`);
              return await step.replaceDialog('/ask', { isReturning: true });
            } else {
              await step.context.sendActivity(`Please complete your payment here: ${result.paymentUrl}`);
              await step.context.sendActivity('I will check for payment completion every few seconds...');

              try {
                const finalResult = await service.waitForPaymentCompletion(min, result.subscriptionId, template);

                await step.context.sendActivity(`Payment verified and bot created successfully!`);
                await step.context.sendActivity(`Access your bot here: ${finalResult.botUrl}`);
                return await step.replaceDialog('/ask', { isReturning: true });
              } catch (error) {
                await step.context.sendActivity(`Error: ${error.message}`);
                return await step.replaceDialog('/welcome_saas');
              }
            }
          }
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

          step.activeDialog.state.options = {
            document: null,
            email: null,
            botName: null,
            templateName: null,
            planId: null,
            name: null,
            mobile: null,
            nextDialog: 'welcome_saas_plan'
          };

          await step.context.sendActivity(Messages[locale].welcome);

          const mobile = step.context.activity.from.id;

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
