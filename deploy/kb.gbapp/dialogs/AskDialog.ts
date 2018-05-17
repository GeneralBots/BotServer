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

import { Prompts, UniversalBot, Session, ListStyle } from "botbuilder";
import { IGBDialog } from "botlib";
import { AzureText } from "pragmatismo-io-framework";
import { GBMinInstance } from "botlib";
import { KBService } from './../services/KBService';

const logger = require("../../../src/logger");

export class AskDialog extends IGBDialog {
  static setup(bot: UniversalBot, min: GBMinInstance) {

    const service = new KBService();

    bot.dialog("/answer", [
      (session, args) => {
        let text = "";

        if (args && args.query) {
          text = args.query;
        } else if (args && args.fromFaq) {
          let msgs = [
            `Ótima escolha, procurando resposta para sua questão...`,
            `Pesquisando sobre o termo...`,
            `Aguarde, por favor, enquanto acho sua resposta...`
          ];
          session.sendTyping();
          session.send(msgs);
        }

        if (text === "") {
          session.replaceDialog("/ask");
        } else {
          AzureText.getSpelledText(
            min.instance.spellcheckerKey,
            text,
            (data, err) => {
              if (data != text) {
                logger.trace("Spelled Text: " + data);
                text = data;
              }
              session.userData.lastQuestion = data;

              service.ask(
                min.instance,
                text,
                min.instance.searchScore,
                session.userData.subjects,
                resultsA => {
                  min.conversationalService.sendEvent(session, "stop", null);

                  if (resultsA && resultsA.answer) {
                    session.userData.isAsking = false;
                    service.sendAnswer(min.conversationalService,
                      session,
                      resultsA.answer
                    );
                    session.userData.lastQuestionId = resultsA.questionId;

                    session.replaceDialog("/ask", { isReturning: true });
                  } else {
                    //if (min.isAsking) {
                    // Second time with no filter.

                    service.ask(
                      min.instance,
                      text,
                      min.instance.searchScore,
                      null,
                      resultsB => {
                        if (resultsB && resultsB.answer) {
                          session.userData.isAsking = false;

                          if (session.userData.subjects.length > 0) {
                            let subjectText =
                              `${KBService.getSubjectItemsSeparatedBySpaces(
                                session.userData.subjects
                              )}`;

                            let msgs = [
                              `Respondendo nao apenas sobre ${subjectText}... `,
                              `Respondendo de modo mais abrangente...`,
                              `Vou te responder de modo mais abrangente... 
                                Não apenas sobre ${subjectText}`
                            ];
                            session.send(msgs);
                          }
                          session.userData.isAsking = false;
                          service.sendAnswer(min.conversationalService,
                            session,
                            resultsB.answer
                          );
                          session.replaceDialog("/ask", { isReturning: true });

                          session.userData.lastQuestionId = resultsB.questionId;
                        } else {

                          min.conversationalService.runNLP(
                            session,
                            min,
                            text,
                            (data, error) => {

                              if (!data) {
                                let msgs = [
                                  "Desculpe-me, não encontrei nada a respeito.",
                                  "Lamento... Não encontrei nada sobre isso. Vamos tentar novamente?",
                                  "Desculpe-me, não achei nada parecido. Poderia tentar escrever de outra forma?"
                                ];

                                session.send(msgs);
                                session.replaceDialog("/ask", { isReturning: true });
                              }
                            }
                          );
                        }
                      }
                    );
                  }
                }
              );
            }
          );
        }
      }
    ]);

    bot
      .dialog("/ask", [
        (session, args) => {
          session.userData.isAsking = true;
          let text = [];
          if (session.userData.subjects.length > 0) {
            text = [
              `Faça sua pergunta...`,
              `Pode perguntar sobre o assunto em questão... `,
              `Qual a pergunta?`
            ];
          }

          if (args && args.isReturning) {
            text = [
              "Sobre o que mais posso ajudar?",
              "Então, posso ajudar em algo a mais?",
              "Deseja fazer outra pergunta?"
            ];
          }
          if (text.length > 0) {
            Prompts.text(session, text);
          }
        },
        (session, results) => {
          session.replaceDialog("/answer", { query: results.response });
        }
      ])
      .triggerAction({
        matches: /^(bing|google)/i
      });
    bot.beginDialogAction("ask", "/ask");
  }
}
