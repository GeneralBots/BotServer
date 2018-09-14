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

import { CSService } from "../services/CSService";
import { AzureText } from "pragmatismo-io-framework";
import { GBMinInstance } from "botlib";
import { IGBDialog } from "botlib";
import { BotAdapter } from "botbuilder";
import { Messages } from "../strings";

export class FeedbackDialog extends IGBDialog {
  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  static setup(bot: BotAdapter, min: GBMinInstance) {
    const service = new CSService();

    min.dialogs.add("/feedbackNumber", [
      async dc => {
        let locale = dc.context.activity.locale;
        await dc.prompt("choicePrompt", Messages[locale].what_about_me, [
          "1",
          "2",
          "3",
          "4",
          "5"
        ]);
      },
      async (dc, value) => {
        let locale = dc.context.activity.locale;
        let rate = value.entity;
        const user = min.userState.get(dc.context);
        await service.updateConversationRate(user.conversation, rate);
        await dc.context.sendActivity(Messages[locale].thanks);
      }
    ]);
    
    min.dialogs.add("/feedback", [
      async (dc, args) => {
        let locale = dc.context.activity.locale;
        if (args && args.fromMenu) {
          await dc.context.sendActivity(Messages[locale].about_suggestions);
        }

        await dc.prompt("textPrompt", Messages[locale].what_about_service);
      },
      async (dc, value) => {
        let locale = dc.context.activity.locale;
        let rate = await AzureText.getSentiment(
          min.instance.textAnalyticsKey,
          min.instance.textAnalyticsServerUrl,
          min.conversationalService.getCurrentLanguage(dc),
          value
        );

        if (rate > 0.5) {
          await dc.context.sendActivity(Messages[locale].glad_you_liked);
        } else {
          await dc.context.sendActivity(Messages[locale].we_will_improve);

          // TODO: Record.
        }
        await dc.replace("/ask", { isReturning: true });
      }
    ]);
  }
}