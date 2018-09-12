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

"use strict"

const UrlJoin = require("url-join")

import { BotAdapter, CardFactory, MessageFactory } from "botbuilder"
import { IGBDialog } from "botlib"
import { GBMinInstance } from "botlib"
import { GuaribasSubject } from '../models'
import { KBService } from "../services/KBService"

export class MenuDialog extends IGBDialog {

  /**
   * Setup dialogs flows and define services call.
   * 
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  static setup(bot: BotAdapter, min: GBMinInstance) {

    var service = new KBService(min.core.sequelize)

    bot
    min.dialogs.add("/menu", [
      async (dc, args) => {
        var rootSubjectId = null

        // var msg = dc.message TODO: message from Where in V4?
        // if (msg.attachments && msg.attachments.length > 0) {
        //   var attachment = msg.attachments[0]         
        // }

        if (args && args.data) {
          var subject = args.data

          // If there is a shortcut specified as subject destination, go there.

          if (subject.to) {
            let dialog = subject.to.split(":")[1]
            await dc.replace("/" + dialog)
            await dc.end()
            return
          }
          const user = min.userState.get(dc.context)
          user.subjects.push(subject)
          rootSubjectId = subject.subjectId

          if (user.subjects.length > 0) {
            let data = await service.getFaqBySubjectArray("menu", user.subjects)
            await min.conversationalService.sendEvent(dc, "play", {
              playerType: "bullet",
              data: data.slice(0, 6)
            })

          }
        } else {
          const user = min.userState.get(dc.context)
          user.subjects = []

          let messages = [
            "Aqui estão algumas categorias de assuntos...",
            "Selecionando o assunto você pode me ajudar a encontrar a resposta certa...",
            "Você pode selecionar algum dos assuntos abaixo e perguntar algo..."
          ]
          await dc.context.sendActivity(messages[0]) // TODO: Handle rnd.
          user.isAsking = false
        }

        const msg = MessageFactory.text('')
        var attachments = []

        let data = await service.getSubjectItems(
          min.instance.instanceId,
          rootSubjectId)

        msg.attachmentLayout = 'carousel'

        data.forEach(function (item: GuaribasSubject) {

          var subject = item
          var card = CardFactory.heroCard(
            subject.title,
            CardFactory.images([UrlJoin(
              "/kb",
              min.instance.kb,
              "subjects",
              subject.internalId + ".png" // TODO: or fallback to subject.png
            )]),
            CardFactory.actions([
              {
                type: 'postBack',
                title: 'Selecionar',
                value: JSON.stringify({
                  title: subject.title,
                  subjectId: subject.subjectId,
                  to: subject.to
                })
              }]))

          attachments.push(card)

        })

        if (attachments.length == 0) {
          const user = min.userState.get(dc.context)
          if (user.subjects && user.subjects.length > 0) {
            await dc.context.sendActivity(
              `Vamos pesquisar sobre ${KBService.getFormattedSubjectItems(
                user.subjects
              )}?`
            )
          }

          await dc.replace("/ask", {})
        } else {
          msg.attachments = attachments
          await dc.context.sendActivity(msg)
        }

        const user = min.userState.get(dc.context)
        user.isAsking = true
      },
      async (dc, value) => {
        var text = value
        if (text === "no" || text === "n") { // TODO: Migrate to a common.
          await dc.replace("/feedback")
        } else {
          await dc.replace("/ask")
        }
      }
    ])
  }
}
