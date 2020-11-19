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
 * @fileoverview General Bots server core.
 */

'use strict';

import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBMinInstance, IGBDialog } from 'botlib';
import { Messages } from '../strings';
import { SecService } from '../../security.gbapp/services/SecService';
import { GBServer } from '../../../src/app';
import { GBConversationalService } from '../services/GBConversationalService';
import { CollectionUtil } from 'pragmatismo-io-framework';
/**
 * Dialog for the bot explains about itself.
 */
export class LanguageDialog extends IGBDialog {
  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  public static setup(bot: BotAdapter, min: GBMinInstance) {
    min.dialogs.add(new WaterfallDialog('/language', [

      async step => {
        const locale = step.context.activity.locale;

        return await min.conversationalService.prompt(min, step,
          Messages[locale].which_language);
      },
      async step => {

        const locale = step.context.activity.locale;
        const user = await min.userProfile.get(step.context, {});

        const list = [
          { name: 'english', code: 'en' },
          { name: 'inglês', code: 'en' },
          { name: 'portuguese', code: 'pt' },
          { name: 'português', code: 'pt' },
          { name: 'spanish', code: 'es' },
          { name: 'espanõl', code: 'es' },
          { name: 'german', code: 'de' },
          { name: 'deutsch', code: 'de' }
        ];
        let translatorLocale = null;
        const text = step.context.activity['originalText'];

        await CollectionUtil.asyncForEach(list, async item => {
          if (GBConversationalService.kmpSearch(text, item.name) != -1) {
            translatorLocale = item.code;
          }
        });

        let sec = new SecService();
        user.systemUser = await sec.updateUserLocale(user.systemUser.userId, translatorLocale);

        await min.userProfile.set(step.context, user);
        await min.conversationalService.sendText(min, step,
          Messages[locale].language_chosen);

        await step.replaceDialog('/ask', { firstTime: true });
      }
    ]));
  }
}
