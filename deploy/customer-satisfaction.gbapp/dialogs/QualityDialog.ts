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
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
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

import { IGBDialog } from  "botlib";
import { UniversalBot, Session, Prompts, ListStyle } from "botbuilder";
import { GBMinInstance } from "botlib";
import { CSService } from "../services/CSService";
const logger = require("../../../src/logger");

export class QualityDialog extends IGBDialog {

  static setup(bot: UniversalBot, min: GBMinInstance) {

    const service = new CSService();

    bot.dialog("/quality", [
      (session, args) => {
        var score = args.score;

        setTimeout(
          () => min.conversationalService.sendEvent(session, "stop", null),
          400
        );

        if (score == 0) {
          let msg = [
            "Desculpe-me, vamos tentar novamente.",
            "Lamento... Vamos tentar novamente!",
            "Desculpe-me. Por favor, tente escrever de outra forma?"
          ];
          session.send(msg);
        } else {
          let msg = [
            "Ótimo, obrigado por contribuir com sua resposta.",
            "Certo, obrigado pela informação.",
            "Obrigado pela contribuição."
          ];
          session.send(msg);

          service.insertQuestionAlternate(
            min.instance.instanceId,
            session.userData.lastQuestion,
            session.userData.lastQuestionId,
            (data, err) => {
              logger.trace("QuestionAlternate inserted.");
            }
          );

          session.replaceDialog('/ask', {isReturning: true});
        }
      }
    ]);
  }
}
