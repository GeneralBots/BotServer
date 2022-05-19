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

import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBMinInstance, IGBDialog } from 'botlib';
import { GBMinService } from '../../core.gbapp/services/GBMinService';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService';
import { SecService } from '../../security.gbapp/services/SecService';
import { CSService } from '../services/CSService';
import { Messages } from '../strings';

/**
 * Dialog for feedback collecting.
 */
export class FeedbackDialog extends IGBDialog {
  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  public static setup(bot: BotAdapter, min: GBMinInstance) {
    const service = new CSService();

    min.dialogs.add(
      new WaterfallDialog('/pleaseNoBadWords', [
        async step => {
          const locale = step.context.activity.locale;
          await min.conversationalService.sendText(min, step, Messages[locale].please_no_bad_words);

          return await step.next();
        }
      ])
    );

    min.dialogs.add(
      new WaterfallDialog('/t', [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          }
          else {
            return await step.next(step.options);
          }
        },
        async step => {

          const locale = step.context.activity.locale;
          const sec = new SecService();
          let from = GBMinService.userMobile(step);
          const user = await min.userProfile.get(step.context, {});

          const args = step.activeDialog.state.options.args;

          // Transfer to...

          if (args && args.to) {


            // An user from Teams willing to transfer to a WhatsApp user.

            await sec.assignHumanAgent(min, args.to, user.userSystemId);
            await min.conversationalService.sendText(min, step, 
              Messages[locale].notify_agent_transfer_done(min.instance.botId));

          }
          else {


            await min.conversationalService.sendText(min, step, Messages[locale].please_wait_transfering);
            const agentSystemId = await sec.assignHumanAgent(min, from);
            user.systemUser = await sec.getUserFromAgentSystemId(agentSystemId);
            await min.userProfile.set(step.context, user);

            if (agentSystemId.charAt(2) === ":" || agentSystemId.indexOf("@") > -1) { // Agent is from Teams or Google Chat.

              const agent = await sec.getUserFromSystemId(agentSystemId);
              await min.conversationalService['sendOnConversation'](min, agent,
                Messages[locale].notify_agent(step.context.activity.from.name));

            }
            else {
            
              await min.whatsAppDirectLine.sendToDevice(agentSystemId, Messages[locale].notify_agent(step.context.activity.from.name));

            }
          }
          return await step.next();

        }
      ])
    );

    min.dialogs.add(
      new WaterfallDialog('/qt', [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          }
          else {
            return await step.next(step.options);
          }
        },
        async step => {

          const locale = step.context.activity.locale;

          const sec = new SecService();
          const userSystemId = GBMinService.userMobile(step);
          const user = await min.userProfile.get(step.context, {});

          if (user.systemUser.agentMode === 'self') {
            const manualUser = await sec.getUserFromAgentSystemId(userSystemId);

            await min.whatsAppDirectLine.sendToDeviceEx(manualUser.userSystemId,
              Messages[locale].notify_end_transfer(min.instance.botId), locale, step.context.activity.conversation.id);

            if (userSystemId.charAt(2) === ":" || userSystemId.indexOf('@') > -1) { // Agent is from Teams or Google Chat.
              await min.conversationalService.sendText(min, step, Messages[locale].notify_end_transfer(min.instance.botId));
            }
            else {
              await min.whatsAppDirectLine.sendToDeviceEx(userSystemId,
                Messages[locale].notify_end_transfer(min.instance.botId), locale
                , step.context.activity.conversation.id);
            }

            await sec.updateHumanAgent(userSystemId, min.instance.instanceId, null);
            await sec.updateHumanAgent(manualUser.userSystemId, min.instance.instanceId, null);

            user.systemUser = await sec.getUserFromSystemId(userSystemId);
            await min.userProfile.set(step.context, user);

          }

          else if (user.systemUser.agentMode === 'human') {
            const agent = await sec.getUserFromSystemId(user.systemUser.agentSystemId);

            await min.whatsAppDirectLine.sendToDeviceEx(user.systemUser.userSystemId,
              Messages[locale].notify_end_transfer(min.instance.botId), locale, step.context.activity.conversation.id);


            if (user.systemUser.agentSystemId.charAt(2) === ":" || userSystemId.indexOf('@') > -1) { // Agent is from Teams or Google Chat.
              await min.conversationalService.sendText(min, step, Messages[locale].notify_end_transfer(min.instance.botId));
            }
            else {
              await min.whatsAppDirectLine.sendToDeviceEx(user.systemUser.agentSystemId,
                Messages[locale].notify_end_transfer(min.instance.botId), locale, step.context.activity.conversation.id);
            }

            await sec.updateHumanAgent(user.systemUser.userSystemId, min.instance.instanceId, null);
            await sec.updateHumanAgent(agent.userSystemId, min.instance.instanceId, null);

            user.systemUser = await sec.getUserFromSystemId(userSystemId);
            await min.userProfile.set(step.context, user);

          }
          else {
            if (user.systemUser.userSystemId.charAt(2) === ":" || userSystemId.indexOf('@') > -1) { // Agent is from Teams or Google Chat.
              await min.conversationalService.sendText(min, step, 'Nenhum atendimento em andamento.');
            }
            else {
              await min.whatsAppDirectLine.sendToDeviceEx(user.systemUser.userSystemId,
                'Nenhum atendimento em andamento.', locale, step.context.activity.conversation.id);
            }
          }

          return await step.next();
        }
      ])
    );

    min.dialogs.add(
      new WaterfallDialog('/feedbackNumber', [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          }
          else {
            return await step.next(step.options);
          }
        },
        async step => {
          const locale = step.context.activity.locale;

          return await step.next();
        },
        async step => {
          const locale = step.context.activity.locale;
          const rate = step.result.entity;
          const user = await min.userProfile.get(step.context, {});
          await service.updateConversationRate(user.conversation, rate);
          await min.conversationalService.sendText(min, step, Messages[locale].thanks);

          return await step.next();
        }
      ])
    );

    min.dialogs.add(
      new WaterfallDialog('/feedback', [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          }
          else {
            return await step.next(step.options);
          }
        },
        async step => {
          const locale = step.context.activity.locale;

          await min.conversationalService.sendText(min, step, Messages[locale].about_suggestions);
          step.activeDialog.state.cbId = (step.options as any).id;

          return await min.conversationalService.prompt(min, step, Messages[locale].what_about_service);
        },
        async step => {
          const fixedLocale = 'en-US';
          const user = await min.userProfile.get(step.context, {});

          // Updates values to perform Bot Analytics.

          const analytics = new AnalyticsService();
          const rate = await analytics.updateConversationSuggestion(
            min.instance.instanceId, user.conversation.conversationId, step.result, user.systemUser.locale);

          if (rate > 0.5) {
            await min.conversationalService.sendText(min, step, Messages[fixedLocale].glad_you_liked);
          } else {

            const message = min.core.getParam<string>(min.instance, 'Feedback Improve Message',
              Messages[fixedLocale].we_will_improve); // TODO: Improve to be multi-language.

            await min.conversationalService.sendText(min, step, message);
          }

          return await step.replaceDialog('/ask', { isReturning: true });
        }
      ])
    );
  }
}
