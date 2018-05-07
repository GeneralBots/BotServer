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
 
import { Length } from "sequelize-typescript";
import {
  UniversalBot,
  Session,
  Message,
  AttachmentLayout,
  CardAction,
  HeroCard,
  CardImage
} from "botbuilder";
import { IGBDialog } from  "botlib";
import { GBMinInstance } from "botlib";
import { AzureText } from "pragmatismo-io-framework";
import { GuaribasSubject } from '../models';
import { KBService } from "../services/KBService";

const UrlJoin = require("url-join");
const WaitUntil = require("wait-until");

export class MenuDialog extends IGBDialog {

  static setup(bot: UniversalBot, min: GBMinInstance) {

    var service = new KBService();

    bot
      .dialog("/menu", [
        (session, args) => {
          var rootSubjectId = null;
          var botId = min.botId;

          var msg = session.message;
          if (msg.attachments && msg.attachments.length > 0) {
            var attachment = msg.attachments[0];         
          }

          if (args && args.data) {
            var subject = JSON.parse(args.data); // ?

            if (subject.to) {
              let dialog = subject.to.split(":")[1];
              session.replaceDialog("/" + dialog);
              session.endDialog();
              return;
            }

            session.userData.subjects.push(subject);
            rootSubjectId = subject.subjectId;

            if (session.userData.subjects.length > 0) {
              
              service.getFaqBySubjectArray(
                "menu",
                session.userData.subjects,
                (data, err) => {
                  min.conversationalService.sendEvent(session, "play", {
                    playerType: "bullet",
                    data: data.slice(0, 6)
                  });
                }
              );
            }
          } else {
            session.userData.subjects = [];
            session.sendTyping();
            WaitUntil()
              .interval(2000)
              .times(1)
              .condition(function(cb) {
                return false;
              })
              .done(function(result) {
                let msgs = [
                  "Aqui estão algumas categorias de assuntos...",
                  "Selecionando o assunto você pode me ajudar a encontrar a resposta certa...",
                  "Você pode selecionar algum dos assuntos abaixo e perguntar algo..."
                ];
                session.send(msgs);
              });

              session.userData.isAsking = false;
          }

          service.getSubjectItems(
            min.instance.instanceId,
            rootSubjectId,
            data => {
              var msg = new Message(session);
              msg.attachmentLayout(AttachmentLayout.carousel);
              var attachments = [];

              data.forEach(function(item: GuaribasSubject) {
                var subject = item;
                var button = CardAction.dialogAction(
                  session,
                  "menuAction",
                  JSON.stringify({
                    title: subject.title,
                    subjectId: subject.subjectId,
                    to: subject.to
                  }),
                  "Selecionar"
                );
                var card = new HeroCard(session)
                  .title(subject.title)
                  .text(subject.description)
                  .images([
                    CardImage.create(
                      session,
                      UrlJoin(
                        "/kb",
                        min.instance.kb,
                        "subjects",
                        subject.internalId + ".png" // TODO: or fallback to subject.png
                      )
                    )
                  ]) // Using public dir of ui.
                  .buttons([button]);
                attachments.push(card);
              });

              if (attachments.length == 0) {
                if (session.userData.subjects && session.userData.subjects.length > 0) {
                  session.send(
                    `Vamos pesquisar sobre ${KBService.getFormattedSubjectItems(
                      session.userData.subjects
                    )}?`
                  );
                }

                session.replaceDialog("/ask", {});
              } else {
                msg.attachments(attachments);
                session.send(msg);
              }
            }
          );

          session.userData.isAsking = true;
        },
        function(session, results) {
          var text = results.response;
          if (AzureText.isIntentNo(text)) {
            session.replaceDialog("/feedback");
          } else {
            session.replaceDialog("/ask");
          }
        }
      ])
      .triggerAction({
        matches: /^(menu)/i
      });

    bot.beginDialogAction("menuAction", "/menu");
  }
}
