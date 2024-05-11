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

import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBMinInstance, IGBDialog } from 'botlib';
import { GBMinService } from '../../core.gbapp/services/GBMinService.js';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService.js';
import { SecService } from '../../security.gbapp/services/SecService.js';
import { CSService } from '../services/CSService.js';
import { Messages } from '../strings.js';

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
          } else {
            return await step.next(step.options);
          }
        },
        async step => {
          const locale = step.context.activity.locale;
          const sec = new SecService();
          let from = GBMinService.userMobile(step);
          const profile = await min.userProfile.get(step.context, {});
          const args = step.activeDialog.state.options;

          // Transfer to...

          if (args && args.to) {
            // An user from Teams willing to transfer to a WhatsApp user.

            await sec.ensureUser(min, args.to, 'Name', '', 'whatsapp', 'Name', null);

            await sec.assignHumanAgent(min, args.to, profile.userSystemId);
            await min.conversationalService.sendText(
              min,
              step,
              Messages[locale].notify_agent_transfer_done(min.instance.botId)
            );
          } else {
            await min.conversationalService.sendText(min, step, Messages[locale].please_wait_transfering);
            const agentSystemId = await sec.assignHumanAgent(min, from);

            await min.userProfile.set(step.context, profile);

            if (agentSystemId.indexOf('@') !== -1) {

              // Agent is from Teams or Google Chat.

              const agent = await sec.getUserFromSystemId(agentSystemId);
              await min.conversationalService['sendOnConversation'](
                min,
                agent,
                Messages[locale].notify_agent(step.context.activity.from.name)
              );
            } else {
              await min.whatsAppDirectLine.sendToDevice(
                agentSystemId,
                Messages[locale].notify_agent(step.context.activity.from.name)
              );
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
          } else {
            return await step.next(step.options);
          }
        },
        async step => {
          const locale = step.context.activity.locale;

          const sec = new SecService();
          const userSystemId = GBMinService.userMobile(step);
          let user = await sec.getUserFromSystemId(userSystemId);

          if (user.agentMode === 'self') {
            const manualUser = await sec.getUserFromAgentSystemId(userSystemId);

            await min.whatsAppDirectLine.sendToDeviceEx(
              manualUser.userSystemId,
              Messages[locale].notify_end_transfer(min.instance.botId),
              locale,
              step.context.activity.conversation.id
            );

            if (userSystemId.charAt(2) === ':' || userSystemId.indexOf('@') > -1) {
              // Agent is from Teams or Google Chat.
              await min.conversationalService.sendText(
                min,
                step,
                Messages[locale].notify_end_transfer(min.instance.botId)
              );
            } else {
              await min.whatsAppDirectLine.sendToDeviceEx(
                userSystemId,
                Messages[locale].notify_end_transfer(min.instance.botId),
                locale,
                step.context.activity.conversation.id
              );
            }

            await sec.updateHumanAgent(userSystemId, min.instance.instanceId, null);
            await sec.updateHumanAgent(manualUser.userSystemId, min.instance.instanceId, null);


          } else if (user.agentMode === 'human') {
            const agent = await sec.getUserFromSystemId(user.agentSystemId);

            await min.whatsAppDirectLine.sendToDeviceEx(
              user.userSystemId,
              Messages[locale].notify_end_transfer(min.instance.botId),
              locale,
              step.context.activity.conversation.id
            );

            if (user.agentSystemId.indexOf('@') !== -1) {
              // Agent is from Teams or Google Chat.
              await min.conversationalService.sendText(
                min,
                step,
                Messages[locale].notify_end_transfer(min.instance.botId)
              );
            } else {
              await min.whatsAppDirectLine.sendToDeviceEx(
                user.agentSystemId,
                Messages[locale].notify_end_transfer(min.instance.botId),
                locale,
                step.context.activity.conversation.id
              );
            }

            await sec.updateHumanAgent(user.userSystemId, min.instance.instanceId, null);
            await sec.updateHumanAgent(agent.userSystemId, min.instance.instanceId, null);

          } else {
            if (user.userSystemId.charAt(2) === ':' || userSystemId.indexOf('@') > -1) {
              // Agent is from Teams or Google Chat.
              await min.conversationalService.sendText(min, step, 'Nenhum atendimento em andamento.');
            } else {
              await min.whatsAppDirectLine.sendToDeviceEx(
                user.userSystemId,
                'Nenhum atendimento em andamento.',
                locale,
                step.context.activity.conversation.id
              );
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
          } else {
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
          } else {
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
          let rate = 1;

          if (process.env.PRIVACY_STORE_MESSAGES === 'true') {
            // Updates values to perform Bot Analytics.

            const analytics = new AnalyticsService();
            let rate = await analytics.updateConversationSuggestion(
              min.instance.instanceId,
              user.conversation.conversationId,
              step.result,
              user.locale
            );
          }

          if (rate > 0.5) {
            await min.conversationalService.sendText(min, step, Messages[fixedLocale].glad_you_liked);
          } else {
            const message = min.core.getParam<string>(
              min.instance,
              'Feedback Improve Message',
              Messages[fixedLocale].we_will_improve
            );

            await min.conversationalService.sendText(min, step, message);
          }

          return await step.replaceDialog('/ask', { isReturning: true });
        }
      ])
    );
  }
}
