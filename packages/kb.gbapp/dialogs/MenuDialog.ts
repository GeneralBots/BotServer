/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
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
 * @fileoverview Main dialog for kb.gbapp
 */

'use strict';

import urlJoin = require('url-join');

import { BotAdapter, CardFactory, MessageFactory } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBMinInstance, IGBDialog } from 'botlib';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService';
import { GuaribasSubject } from '../models';
import { KBService } from '../services/KBService';
import { Messages } from '../strings';

/**
 * Dialog arguments.
 */
export class MenuDialogArgs {
  public to: string;
  public subjectId: string;
}

/**
 * Dialogs for handling Menu control.
 */
export class MenuDialog extends IGBDialog {
  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  public static setup(bot: BotAdapter, min: GBMinInstance) {
    const service = new KBService(min.core.sequelize);

    min.dialogs.add(new WaterfallDialog('/menu', MenuDialog.getMenuDialog(min, service)));
  }

  private static getMenuDialog(min: GBMinInstance, service: KBService) {
    return [
      async step => {
        if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
          return await step.beginDialog('/auth');
        }
        else{
          return await step.next(step.options);
        }
      },

      async step => {
        const locale = step.context.activity.locale;
        const user = await min.userProfile.get(step.context, {});
        const args: MenuDialogArgs = step.options;

        // tslint:disable-next-line: no-null-keyword
        let rootSubjectId = null;

        if (Object.keys(args).length > 0) {
          // If there is a shortcut specified as subject destination, go there.
          if (args.to !== null) {
            const dialog = args.to.split(':')[1];

            return await step.replaceDialog(`/${dialog}`);
          }

          user.subjects.push(args);
          // tslint:disable-next-line: no-null-keyword
          rootSubjectId = args.subjectId === undefined ? null : args.subjectId;

          // Whenever a subject is selected, shows a faq about it.
          if (user.subjects.length > 0) {
            const list = await service.getFaqBySubjectArray(min.instance.instanceId,
               'menu', user.subjects);
            await min.conversationalService.sendEvent(min, step, 'play', {
              playerType: 'bullet',
              data: list.slice(0, 10)
            });
          }
        } else {
          user.subjects = [];
          await min.conversationalService.sendText(min, step, Messages[locale].here_is_subjects);
          user.isAsking = false;
        }
        const msg = MessageFactory.text('');
        const attachments = [];
        const data = await service.getSubjectItems(min.instance.instanceId, rootSubjectId);
        msg.attachmentLayout = 'carousel';
        data.forEach((item: GuaribasSubject) => {
          const subject = item;
          const card = CardFactory.heroCard(
            subject.title,
            subject.description,
            CardFactory.images([urlJoin('/kb', min.instance.kb, 'subjects', 'subject.png')]),
            CardFactory.actions([
              {
                type: 'postBack',
                title: Messages[locale].menu_select,
                value: JSON.stringify({
                  title: subject.title,
                  description: subject.description,
                  subjectId: subject.subjectId,
                  internalId: subject.internalId,
                  to: subject.to
                })
              }
            ])
          );
          attachments.push(card);
        });
        if (attachments.length === 0) {

          if (user.subjects && user.subjects.length > 0) {
            await min.conversationalService.sendText(min, step,
                                                     Messages[locale].lets_search(KBService.getFormattedSubjectItems(user.subjects))
            );
          }
        } else {
          msg.attachments = attachments;
          await step.context.sendActivity(msg);
        }
        user.isAsking = true;

        return await step.next();
      }
    ];
  }
}
