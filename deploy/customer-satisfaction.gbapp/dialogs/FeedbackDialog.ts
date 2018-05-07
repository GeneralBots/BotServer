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

import { UniversalBot, Session, Prompts, ListStyle } from "botbuilder";
import { CSService } from '../services/CSService';
import { AzureText } from "pragmatismo-io-framework";
import { GBMinInstance } from "botlib";
import { IGBDialog } from  "botlib";

export class FeedbackDialog extends IGBDialog {

  static setup(bot: UniversalBot, min: GBMinInstance) {
    
    const service = new CSService();

    bot.dialog("/feedbackNumber", [
      function(session, args) {
        session.sendTyping();
        let msgs = [
          "O que achou do meu atendimento, de 1 a 5?",
          "Qual a nota do meu atendimento?",
          "Como define meu atendimento numa escala de 1 a 5?"
        ];
        Prompts.choice(session, msgs, "1|2|3|4|5", {
          listStyle: ListStyle.button
        });
      },
      function(session, results) {
        let rate = results.response.entity;
        service.updateConversationRate(session.userData.conversation, rate, item => {
          let msgs = ["Obrigado!", "Obrigado por responder."];
          session.send(msgs);
        });
      }
    ]);

    bot.dialog("/feedback", [
      function(session, args) {
        if (args && args.fromMenu) {
          let msgs = [
            "Sugestões melhoram muito minha qualidade...",
            "Obrigado pela sua iniciativa de sugestão."
          ];
          session.send(msgs);
        }
        session.sendTyping();
        let msgs = [
          "O que achou do meu atendimento?",
          "Como foi meu atendimento?",
          "Gostaria de dizer algo sobre meu atendimento?"
        ];
        Prompts.text(session, msgs);
      },
      function(session, results) {
        AzureText.getSentiment(
          min.instance.textAnalyticsKey,
          results.response,
          (err, rate) => {
            if (!err && rate > 0) {
              session.send("Bom saber que você gostou. Conte comigo.");
            } else {
              session.send(
                "Vamos registrar sua questão, obrigado pela sinceridade."
              );
            }
            session.replaceDialog('/ask', {isReturning: true});
          }
        );
      }
    ]);
  }
}
