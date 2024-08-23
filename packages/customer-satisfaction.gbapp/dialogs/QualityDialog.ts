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

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import { GBMinInstance, IGBDialog } from 'botlib';

import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService.js';
import { CSService } from '../services/CSService.js';
import { Messages } from '../strings.js';

/**
 * Dialog for collecting quality of answer.
 */
export class QualityDialog extends IGBDialog {
  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  public static setup(bot: BotAdapter, min: GBMinInstance) {
    const service = new CSService();

    min.dialogs.add(
      new WaterfallDialog('/check', [
        async step => {
          const locale = step.context.activity.locale;
          await min.conversationalService.sendText(min, step, Messages[locale].check_whatsapp_ok);
          return await step.replaceDialog('/ask', { isReturning: true });
        }
      ])
    );

    min.dialogs.add(
      new WaterfallDialog('/quality', [
        async step => {
          const locale = step.context.activity.locale;
          const user = await min.userProfile.get(step.context, {});

          const score = step.result;

          if (score === 0) {
            await min.conversationalService.sendText(min, step, Messages[locale].im_sorry_lets_try);

            return await step.next();
          } else {
            await min.conversationalService.sendText(min, step, Messages[locale].great_thanks);
            await min.conversationalService.sendEvent(min, step, 'play', {
              playerType: 'markdown',
              data: {
                content: Messages[locale].great_thanks
              }
            });
            await service.insertQuestionAlternate(min.instance.instanceId, user.lastQuestion, user.lastQuestionId);

            // Updates values to perform Bot Analytics.
            if (process.env.PRIVACY_STORE_MESSAGES === 'true') {
              const analytics = new AnalyticsService();
              analytics.updateConversationSuggestion(
                min.instance.instanceId,
                user.conversation,
                step.result,
                user.locale
              );
            }
            // Goes to the ask loop.

            return await step.replaceDialog('/ask', { emptyPrompt: true });
          }
        }
      ])
    );
  }
}
