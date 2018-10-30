/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
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

"use strict";

import { IGBDialog } from "botlib";
import { GBMinInstance } from "botlib";
import { BotAdapter } from "botbuilder";
import { Messages } from "../strings";

export class WelcomeDialog extends IGBDialog {
  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  static setup(bot: BotAdapter, min: GBMinInstance) {
    min.dialogs.add("/", [
      async (dc, args) => {
        const user = min.userState.get(dc.context);
        const locale = dc.context.activity.locale;

        if (!user.once) {
          user.once = true;
          var a = new Date();
          const date = a.getHours();
          var msg =
            date < 12
              ? Messages[locale].good_morning
              : date < 18
                ? Messages[locale].good_evening
                : Messages[locale].good_night;

          await dc.context.sendActivity(Messages[locale].hi(msg));
          await dc.replace("/ask", { firstTime: true });

          if (
            dc.context.activity &&
            dc.context.activity.type == "message" &&
            dc.context.activity.text != ""
          ) {
            await dc.replace("/answer", { query: dc.context.activity.text });
          }
        }
      }
    ]);
  }
}
