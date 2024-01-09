/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.             |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.                 |
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
import { GBLog, GBMinInstance, IGBDialog } from 'botlib';
import { GBServer } from '../../../src/app.js';
import { GBConversationalService } from '../services/GBConversationalService.js';
import { Messages } from '../strings.js';

/**
 *  Dialog for Welcoming people.
 */
export class WelcomeDialog extends IGBDialog {
  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  public static setup (bot: BotAdapter, min: GBMinInstance) {
    min.dialogs.add(
      new WaterfallDialog('/', [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          } else {
            return await step.next(step.options);
          }
        },
        async step => {
          if (
            GBServer.globals.entryPointDialog !== null &&
            min.instance.botId === process.env.BOT_ID &&
            step.context.activity.channelId === 'webchat'
          ) {
            return step.replaceDialog(GBServer.globals.entryPointDialog);
          }

          const locale = step.context.activity.locale;

          if (
            //  TODO: https://github.com/GeneralBots/BotServer/issues/9            !user.once &&
            step.context.activity.channelId === 'webchat' &&
            min.core.getParam<boolean>(min.instance, 'HelloGoodX', true) === 'true'
          ) {
            // user.once = true;
            const a = new Date();
            const date = a.getHours();
            const msg =
              date < 12
                ? Messages[locale].good_morning
                : date < 18
                ? Messages[locale].good_evening
                : Messages[locale].good_night;

            await min.conversationalService.sendText(min, step, Messages[locale].hi(msg));

            await step.replaceDialog('/ask', { firstTime: true });

            if (
              step.context.activity !== undefined &&
              step.context.activity.type === 'message' &&
              step.context.activity.text !== ''
            ) {
              GBLog.info(`/answer being called from WelcomeDialog.`);
              await step.replaceDialog('/answer', { query: step.context.activity.text });
            }
          }

          return await step.next();
        }
      ])
    );
  }
}
