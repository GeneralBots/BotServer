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
      new WaterfallDialog('/feedbackNumber', [
        async step => {
          const locale = step.context.activity.locale;

          return await step.next();
        },
        async step => {
          const locale = step.context.activity.locale;
          const rate = step.result.entity;
          const user = await min.userProfile.get(context, {});
          await service.updateConversationRate(user.conversation, rate);
          await step.context.sendActivity(Messages[locale].thanks);

          return await step.next();
        }
      ])
    );

    min.dialogs.add(
      new WaterfallDialog('/feedback', [
        async step => {
          const locale = step.context.activity.locale;

          await step.context.sendActivity(Messages[locale].about_suggestions);
          step.activeDialog.state.cbId = (step.options as any).id;

          return await step.prompt('textPrompt', Messages[locale].what_about_service);
        },
        async step => {
          const locale = step.context.activity.locale;
          const rate = await AzureText.getSentiment(
            min.instance.textAnalyticsKey,
            min.instance.textAnalyticsEndpoint,
            min.conversationalService.getCurrentLanguage(step),
            step.result
          );

          if (rate > 0.5) {
            await step.context.sendActivity(Messages[locale].glad_you_liked);
          } else {
            await step.context.sendActivity(Messages[locale].we_will_improve);
        }

          return await step.replaceDialog('/ask', { isReturning: true });
        }
      ])
    );
  }
}
