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
import {WaterfallDialog } from 'botbuilder-dialogs';
import { GBMinInstance, IGBDialog } from 'botlib';
import { Messages } from '../strings';
import { GBServer } from '../../../src/app';

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
  public static setup(bot: BotAdapter, min: GBMinInstance) {

    min.dialogs.add(new WaterfallDialog('/', [
      async step => {

        if (GBServer.globals.entryPointDialog !== null)
        {
          return step.replaceDialog(GBServer.globals.entryPointDialog);
        }

        const user = await min.userProfile.get(step.context, {});
        const locale = step.context.activity.locale;

        if (!user.once) {
          user.once = true;
          await min.userProfile.set(step.context, user);
          const a = new Date();
          const date = a.getHours();
          const msg =
            date < 12
              ? Messages[locale].good_morning
              : date < 18
                ? Messages[locale].good_evening
                : Messages[locale].good_night;

          await step.context.sendActivity(Messages[locale].hi(msg));
          await step.replaceDialog('/ask', { firstTime: true });

          if (
            step.context.activity !== undefined &&
            step.context.activity.type === 'message' &&
            step.context.activity.text !== ''
          ) {
            await step.replaceDialog('/answer', { query: step.context.activity.text });
          }
        }

        return await step.next();
      }
    ]));
  }
}
