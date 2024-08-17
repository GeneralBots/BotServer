/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
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
import { Messages } from '../strings.js';
import { SecService } from '../../security.gbapp/services/SecService.js';
import { GBServer } from '../../../src/app.js';
import { GBConversationalService } from '../services/GBConversationalService.js';
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
  public static setup (bot: BotAdapter, min: GBMinInstance) {
    min.dialogs.add(
      new WaterfallDialog('/language', [
        async step => {
          if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
            return await step.beginDialog('/auth');
          } else {
            return await step.next(step.options);
          }
        },

        async step => {
          const locale = step.context.activity.locale;

          return await min.conversationalService.prompt(min, step, Messages[locale].which_language);
        },
        async step => {
          const locale = step.context.activity.locale;          

          const list = [
            { name: 'english', code: 'en' },
            { name: 'inglês', code: 'en' },
            { name: 'portuguese', code: 'pt' },
            { name: 'português', code: 'pt' },
            { name: 'français', code: 'fr' },
            { name: 'francês', code: 'fr' },
            { name: 'french', code: 'fr' },
            { name: 'português', code: 'pt' },
            { name: 'spanish', code: 'es' },
            { name: 'espanõl', code: 'es' },
            { name: 'espanhol', code: 'es' },
            { name: 'german', code: 'de' },
            { name: 'deutsch', code: 'de' },
            { name: 'alemão', code: 'de' }
          ];
          let translatorLocale = null;
          const text = step.context.activity['originalText'];

          await CollectionUtil.asyncForEach(list, async item => {
            if (
              GBConversationalService.kmpSearch(text.toLowerCase(), item.name.toLowerCase()) != -1 ||
              GBConversationalService.kmpSearch(text.toLowerCase(), item.code.toLowerCase()) != -1
            ) {
              translatorLocale = item.code;
            }
          });

          let sec = new SecService();
          let user = await  sec.getUserFromSystemId(step.context.activity.from.id);
          user = await sec.updateUserLocale(user.userId, translatorLocale);

          await min.conversationalService.sendText(min, step, Messages[locale].language_chosen);

          await step.replaceDialog('/ask', { firstTime: true });
        }
      ])
    );
  }
}
