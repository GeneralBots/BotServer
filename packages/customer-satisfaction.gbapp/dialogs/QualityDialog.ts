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

import { GBMinInstance, IGBDialog } from 'botlib';

import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService';
import { CSService } from '../services/CSService';
import { Messages } from '../strings';

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

    min.dialogs.add(new WaterfallDialog('/quality', [
      async step =>  {
        const locale = step.context.activity.locale;
        const user = await min.userProfile.get(step.context, {});

        const score = step.result;

        setTimeout(
          () => min.conversationalService.sendEvent(min, step, 'stop', undefined),
          400
        );

        if (score === 0) {
          await min.conversationalService.sendText(min, step, Messages[locale].im_sorry_lets_try);
        } else {
          await min.conversationalService.sendText(min, step, Messages[locale].great_thanks);

          await service.insertQuestionAlternate(
            min.instance.instanceId,
            user.lastQuestion,
            user.lastQuestionId
          );

          // Updates values to perform Bot Analytics.

          const analytics = new AnalyticsService();
          analytics.updateConversationSuggestion(
            min.instance.instanceId, user.conversation, step.result, user.systemUser.locale);

          // Goes to the ask loop.

          await step.replaceDialog('/ask', { isReturning: true });
        }

        return await step.next();
      }
    ]));
  }
}
