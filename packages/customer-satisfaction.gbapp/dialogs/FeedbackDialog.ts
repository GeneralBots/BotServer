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
import { AzureText } from 'pragmatismo-io-framework';
import { CSService } from '../services/CSService';
import { Messages } from '../strings';
import { SecService } from '../../security.gbapp/services/SecService';
import { GBServer } from '../../../src/app';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService';

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

          const locale = step.context.activity.locale;

          let sec = new SecService();
          let from = step.context.activity.from.id;

          await min.conversationalService.sendText(min, step, Messages[locale].please_wait_transfering);
          let agentSystemId = await sec.assignHumanAgent(from, min.instance.instanceId);

          await min.whatsAppDirectLine.sendToDevice(agentSystemId,
            Messages[locale].notify_agent(step.context.activity.from.name));

          return await step.next();
        }
      ])
    );

    min.dialogs.add(
      new WaterfallDialog('/qt', [
        async step => {

          const locale = step.context.activity.locale;

          let sec = new SecService();
          let from = step.context.activity.from.id;

          await sec.updateCurrentAgent(from, min.instance.instanceId, null);
          await min.conversationalService.sendText(min, step, Messages[locale].notify_end_transfer(min.instance.botId));

          return await step.next();
        }
      ])
    );


    min.dialogs.add(
      new WaterfallDialog('/feedbackNumber', [
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
          const locale = step.context.activity.locale;

          await min.conversationalService.sendText(min, step, Messages[locale].about_suggestions);
          step.activeDialog.state.cbId = (step.options as any).id;

          return await min.conversationalService.prompt(min, step, Messages[locale].what_about_service);
        },
        async step => {
          const minBoot = GBServer.globals.minBoot as any;
          const user = await min.userProfile.get(step.context, {});

          const rate = await AzureText.getSentiment(
            minBoot.instance.textAnalyticsKey ? minBoot.instance.textAnalyticsKey : minBoot.instance.textAnalyticsKey,
            minBoot.instance.textAnalyticsEndpoint ? minBoot.instance.textAnalyticsEndpoint : minBoot.instance.textAnalyticsEndpoint,
            user.systemUser.locale,
            step.result
          );

          // Updates values to perform Bot Analytics.

          // const analytics = new AnalyticsService();
          // analytics.updateConversationRate(min.instance.instanceId, user.conversation, rate);

          const fixedLocale = 'en-US';
          if (rate > 0.5) {
            await min.conversationalService.sendText(min, step, Messages[fixedLocale].glad_you_liked);
          } else {

            const message = min.core.getParam<string>(min.instance, "Feedback Improve Message",
              Messages[fixedLocale].we_will_improve); // TODO: Improve to be multi-language.

            await min.conversationalService.sendText(min, step, message);
          }

          return await step.replaceDialog('/ask', { isReturning: true });
        }
      ])
    );
  }
}
