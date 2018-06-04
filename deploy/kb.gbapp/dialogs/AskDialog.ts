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

import { IGBDialog } from "botlib";
import { AzureText } from "pragmatismo-io-framework";
import { GBMinInstance } from "botlib";
import { KBService } from './../services/KBService';
import { BotAdapter } from "botbuilder";
import { LuisRecognizer } from "botbuilder-ai";

const logger = require("../../../src/logger");

export class AskDialog extends IGBDialog {
  static setup(bot: BotAdapter, min: GBMinInstance) {

    const service = new KBService();

    const model = new LuisRecognizer({
      appId: min.instance.nlpAppId,
      subscriptionKey: min.instance.nlpSubscriptionKey,
      serviceEndpoint: min.instance.nlpServerUrl
  });


    min.dialogs.add("/answer", [
      async (dc, args) => {
        const user = min.userState.get(dc.context);

        let text = "";

        if (args && args.query) {
          text = args.query;
        } else if (args && args.fromFaq) {
          let messages = [
            `Ótima escolha, procurando resposta para sua questão...`,
            `Pesquisando sobre o termo...`,
            `Aguarde, por favor, enquanto acho sua resposta...`
          ];

          dc.context.sendActivity(messages[0]); // TODO: Handle rnd.
        }

      await model.recognize(dc.context).then(res => {
        console.log(res);
      }).catch(err => {
        console.log(err);
      });

        // await min.conversationalService.runNLP(
        //   dc,
        //   min,
        //   text,
        //   (data, error) => {

        //     if (!data) {
        //       let messages = [
        //         "Desculpe-me, não encontrei nada a respeito.",
        //         "Lamento... Não encontrei nada sobre isso. Vamos tentar novamente?",
        //         "Desculpe-me, não achei nada parecido. Poderia tentar escrever de outra forma?"
        //       ];

        //       dc.context.sendActivity(messages[0]); // TODO: Handle rnd.
        //       dc.replace("/ask", { isReturning: true });
        //     }
        //   }
        // );


        // if (text === "") {
        //   dc.replace("/ask");
        // } else {
        //   // AzureText.getSpelledText(
        //   //   min.instance.spellcheckerKey,
        //   //   text,
        //   //   async (data, err) => {
        //   var data = text;
        //   if (data != text) {
        //     logger.trace("Spelled Text: " + data);
        //     text = data;
        //   }
        //   user.lastQuestion = data;

        //   service.ask(
        //     min.instance,
        //     text,
        //     min.instance.searchScore,
        //     user.subjects,
        //     async resultsA => {
        //       min.conversationalService.sendEvent(dc, "stop", null);

        //       if (resultsA && resultsA.answer) {
        //         user.isAsking = false;
        //         service.sendAnswer(min.conversationalService,
        //           dc,
        //           resultsA.answer
        //         );
        //         user.lastQuestionId = resultsA.questionId;

        //         dc.replace("/ask", { isReturning: true });
        //       } else {
        //         //if (min.isAsking) {
        //         // Second time with no filter.

        //         service.ask(
        //           min.instance,
        //           text,
        //           min.instance.searchScore,
        //           null,
        //           async resultsB => {
        //             if (resultsB && resultsB.answer) {
        //               const user = min.userState.get(dc.context);

        //               user.isAsking = false;

        //               if (user.subjects.length > 0) {
        //                 let subjectText =
        //                   `${KBService.getSubjectItemsSeparatedBySpaces(
        //                     user.subjects
        //                   )}`;

        //                 let messages = [
        //                   `Respondendo nao apenas sobre ${subjectText}... `,
        //                   `Respondendo de modo mais abrangente...`,
        //                   `Vou te responder de modo mais abrangente... 
        //                         Não apenas sobre ${subjectText}`
        //                 ];
        //                 dc.context.sendActivity(messages[0]); // TODO: Handle rnd.
        //               }
        //               user.isAsking = false;
        //               service.sendAnswer(min.conversationalService,
        //                 dc,
        //                 resultsB.answer
        //               );
        //               dc.replace("/ask", { isReturning: true });

        //               user.lastQuestionId = resultsB.questionId;
        //             } else {

        //               await min.conversationalService.runNLP(
        //                 dc,
        //                 min,
        //                 text,
        //                 (data, error) => {

        //                   if (!data) {
        //                     let messages = [
        //                       "Desculpe-me, não encontrei nada a respeito.",
        //                       "Lamento... Não encontrei nada sobre isso. Vamos tentar novamente?",
        //                       "Desculpe-me, não achei nada parecido. Poderia tentar escrever de outra forma?"
        //                     ];

        //                     dc.context.sendActivity(messages[0]); // TODO: Handle rnd.
        //                     dc.replace("/ask", { isReturning: true });
        //                   }
        //                 }
        //               );
        //             }
        //           }
        //         );
        //       }
        //     }
        //   );
        // }
        //);        }
      }
    ]);

    bot
    min.dialogs.add("/ask", [
      async (dc, args) => {
        const user = min.userState.get(dc.context);
        user.isAsking = true;
        if (!user.subjects) {
          user.subjects = [];
        }
        let text = [];
        if (user.subjects.length > 0) {
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
          await dc.prompt('textPrompt', text[0]);
        }
      },
      async (dc, value) => {
        dc.endAll();
        dc.begin("/answer", { query: value });
      }
    ]);
  }
}
