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

'use strict';

import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBLog } from 'botlib';
import * as fs from 'fs';
import { CollectionUtil } from 'pragmatismo-io-framework';
import * as request from 'request-promise-native';
import { Messages } from '../strings';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService';
const Path = require('path');
const phoneUtil = require('google-libphonenumber').PhoneNumberUtil.getInstance();
const phone = require('phone');


//tslint:disable-next-line:no-submodule-imports
/**
 * HEAR Bot Framework support.
 */
export class HearDialog {

  private static async downloadAttachmentAndWrite(attachment) {

    // Retrieve the attachment via the attachment's contentUrl.
    const url = attachment.contentUrl;

    // Local file path for the bot to save the attachment.
    const localFileName = Path.join(__dirname, attachment.name);

    try {
      // arraybuffer is necessary for images
      const options = {
        url: url,
        method: 'GET',
        encoding: 'binary',
      };
      let response = await request.get(options);

  
      fs.writeFile(localFileName, response, (fsError) => {
        if (fsError) {
          throw fsError;
        }
      });
    } catch (error) {
      console.error(error);
      return undefined;
    }
    // If no error was thrown while writing to disk, return the attachment's name
    // and localFilePath for the response back to the user.
    return {
      fileName: attachment.name,
      localPath: localFileName
    };
  }

  public static addHearDialog(min) {
    min.dialogs.add(
      new WaterfallDialog('/hear', [
        async step => {
          step.activeDialog.state.options = step.options;
          step.activeDialog.state.options.id = (step.options as any).id;
          step.activeDialog.state.options.previousResolve = (step.options as any).previousResolve;

          if (step.options['args']) {

            GBLog.info(`BASIC: Asking for input (HEAR with ${step.options['args'][0]}).`);
          }
          else {

            GBLog.info('BASIC: Asking for input (HEAR).');
          }

          step.activeDialog.state.options = step.options;
          if (step.activeDialog.state.options['kind'] === "login") {
            if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
              GBLog.info('BASIC: Authenticating beforing running General Bots BASIC code.');
              return await step.beginDialog('/auth');
            }
          }
          return await step.next(step.options);
        },
        async step => {
          if (step.activeDialog.state.options['kind'] === "login") {
            return await step.next(step.options);
          } else {

            if (step.activeDialog.state.options['kind'] === "file") {
            return await step.prompt('attachmentPrompt', {});
            }
            else{
              return await min.conversationalService.prompt(min, step, null);
            }
          }

        },
        async step => {

          const isIntentYes = (locale, utterance) => {
            return utterance.toLowerCase().match(Messages[locale].affirmative_sentences);
          }

          let result = step.context.activity['originalText'];
          if (step.activeDialog.state.options['kind'] === "file") {

            // Prepare Promises to download each attachment and then execute each Promise.
            const promises = step.context.activity.attachments.map(HearDialog.downloadAttachmentAndWrite);
            const successfulSaves = await Promise.all(promises);

            async function replyForReceivedAttachments(localAttachmentData) {
              if (localAttachmentData) {
                // Because the TurnContext was bound to this function, the bot can call
                // `TurnContext.sendActivity` via `this.sendActivity`;
                await this.sendActivity(`Attachment "${localAttachmentData.fileName}" ` +
                  `has been received and saved to "${localAttachmentData.localPath}".`);
              } else {
                await this.sendActivity('Attachment was not successfully saved to disk.');
              }
            }

            // Prepare Promises to reply to the user with information about saved attachments.
            // The current TurnContext is bound so `replyForReceivedAttachments` can also send replies.
            const replyPromises = successfulSaves.map(replyForReceivedAttachments.bind(step.context));
            await Promise.all(replyPromises);

          }
          else if (step.activeDialog.state.options['kind'] === "boolean") {
            if (isIntentYes('pt-BR', step.result)) {
              result = true;
            }
            else {
              result = false;
            }
          }
          else if (step.activeDialog.state.options['kind'] === "email") {

            const extractEntity = (text) => {
              return text.match(/([a-zA-Z0-9._-]+@[a-zA-Z0-9._-]+\.[a-zA-Z0-9_-]+)/gi);
            }

            const value = extractEntity(step.result);

            if (value === null) {
              await step.context.sendActivity("Por favor, digite um e-mail válido.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;

          }
          else if (step.activeDialog.state.options['kind'] === "name") {
            const extractEntity = text => {
              return text.match(/[_a-zA-Z][_a-zA-Z0-9]{0,16}/gi);
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um nome válido.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;

          }
          else if (step.activeDialog.state.options['kind'] === "integer") {
            const extractEntity = text => {
              return text.match(/\d+/gi);
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um número válido.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;
          }
          else if (step.activeDialog.state.options['kind'] === "date") {
            const extractEntity = text => {
              return text.match(/(^(((0[1-9]|1[0-9]|2[0-8])[\/](0[1-9]|1[012]))|((29|30|31)[\/](0[13578]|1[02]))|((29|30)[\/](0[4,6,9]|11)))[\/](19|[2-9][0-9])\d\d$)|(^29[\/]02[\/](19|[2-9][0-9])(00|04|08|12|16|20|24|28|32|36|40|44|48|52|56|60|64|68|72|76|80|84|88|92|96)$)/gi);
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite uma data no formato 12/12/2020.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;
          }
          else if (step.activeDialog.state.options['kind'] === "hour") {

            const extractEntity = text => {
              return text.match(/^([0-1]?[0-9]|2[0-4]):([0-5][0-9])(:[0-5][0-9])?$/gi);
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um horário no formato hh:ss.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;
          }
          else if (step.activeDialog.state.options['kind'] === "money") {
            const extractEntity = text => {

              if (step.context.locale === 'en') { // TODO: Change to user.
                return text.match(/(?:\d{1,3},)*\d{1,3}(?:\.\d+)?/gi);
              }
              else {
                return text.match(/(?:\d{1,3}.)*\d{1,3}(?:\,\d+)?/gi);
              }
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um valor monetário.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;
          }
          else if (step.activeDialog.state.options['kind'] === "mobile") {
            const locale = step.context.activity.locale;
            let phoneNumber;
            try {
              phoneNumber = phone(step.result, 'BRA')[0]; // TODO: Use accordingly to the person.
              phoneNumber = phoneUtil.parse(phoneNumber);
            } catch (error) {
              await step.context.sendActivity(Messages[locale].validation_enter_valid_mobile);

              return await step.replaceDialog('/profile_mobile', step.activeDialog.state.options);
            }
            if (!phoneUtil.isPossibleNumber(phoneNumber)) {
              await step.context.sendActivity("Por favor, digite um número de telefone válido.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = phoneNumber;

          }
          else if (step.activeDialog.state.options['kind'] === "zipcode") {
            const extractEntity = text => {

              text = text.replace(/\-/gi, '');

              if (step.context.locale === 'en') { // TODO: Change to user.
                return text.match(/\d{8}/gi);
              }
              else {
                return text.match(/(?:\d{1,3}.)*\d{1,3}(?:\,\d+)?/gi);

              }
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um valor monetário.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value[0];

          }
          else if (step.activeDialog.state.options['kind'] === "menu") {

            const list = step.activeDialog.state.options['args'];
            result = null;
            await CollectionUtil.asyncForEach(list, async item => {
              if (GBConversationalService.kmpSearch(step.result, item) != -1) {
                result = item;
              }
            });

            if (result === null) {
              await step.context.sendActivity(`Escolha por favor um dos itens sugeridos.`);
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }
          }
          else if (step.activeDialog.state.options['kind'] === "language") {

            result = null;

            const list = [
              { name: 'english', code: 'en' },
              { name: 'inglês', code: 'en' },
              { name: 'portuguese', code: 'pt' },
              { name: 'português', code: 'pt' },
              { name: 'français', code: 'fr' },
              { name: 'francês', code: 'fr' },
              { name: 'french', code: 'fr' },
              { name: 'spanish', code: 'es' },
              { name: 'espanõl', code: 'es' },
              { name: 'espanhol', code: 'es' },
              { name: 'german', code: 'de' },
              { name: 'deutsch', code: 'de' },
              { name: 'alemão', code: 'de' }
            ];

            const text = step.context.activity['originalText'];

            await CollectionUtil.asyncForEach(list, async item => {
              if (GBConversationalService.kmpSearch(text.toLowerCase(), item.name.toLowerCase()) != -1 ||
                GBConversationalService.kmpSearch(text.toLowerCase(), item.code.toLowerCase()) != -1) {
                result = item.code;
              }
            });

            if (result === null) {
              await min.conversationalService.sendText(min, step, `Escolha por favor um dos idiomas sugeridos.`);
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

          }

          const id = step.activeDialog.state.options.id;
          if (min.cbMap[id]) {
            const promise = min.cbMap[id].promise;
            delete min.cbMap[id];
            try {
              const opts = await promise(step, result);
              if (opts) {
                return await step.replaceDialog('/hear', opts);
              }
            } catch (error) {
              GBLog.error(`Error in BASIC code: ${error}`);
              const locale = step.context.activity.locale;
              await min.conversationalService.sendText(min, step, Messages[locale].very_sorry_about_error);
            }
          }
          return await step.endDialog();
        }
      ])
    );
  }
}