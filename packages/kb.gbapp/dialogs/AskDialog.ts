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

import { GBServer } from '../../../src/app.js';
import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { ChatGPTAPIBrowser } from 'chatgpt';
import { GBLog, GBMinInstance, IGBDialog, IGBPackage } from 'botlib';
import { Messages } from '../strings.js';
import { KBService } from './../services/KBService.js';
import { GuaribasAnswer } from '../models/index.js';
import { SecService } from '../../security.gbapp/services/SecService.js';
import { CollectionUtil, AzureText } from 'pragmatismo-io-framework';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import { GBImporter } from '../../core.gbapp/services/GBImporterService.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import Fs from 'fs';
import urlJoin from 'url-join';
import { SystemKeywords } from '../../basic.gblib/services/SystemKeywords.js';

/**
 * Dialog arguments.
 */
export class AskDialogArgs {
  public questionId: number;
  public fromFaq: boolean;
}

/**
 * Handle the ask loop on knowledge base data or delegate to other services.
 */
export class AskDialog extends IGBDialog {
  static deployer: any;
  /**
   * Setup dialogs flows and define services call.
   *
   * @param bot The bot adapter.
   * @param min The minimal bot instance data.
   */
  public static setup(bot: BotAdapter, min: GBMinInstance) {
    const service = new KBService(min.core.sequelize);
    const importer = new GBImporter(min.core);
    this.deployer = new GBDeployer(min.core, importer);

    min.dialogs.add(new WaterfallDialog('/answerEvent', AskDialog.getAnswerEventDialog(service, min)));
    min.dialogs.add(new WaterfallDialog('/answer', AskDialog.getAnswerDialog(min, service)));
    min.dialogs.add(new WaterfallDialog('/ask', AskDialog.getAskDialog(min)));
    min.dialogs.add(new WaterfallDialog('/dialog', AskDialog.getLLVMDialog(min, service)));
  }

  private static getAskDialog(min: GBMinInstance) {
    return [
      async step => {
        if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
          return await step.beginDialog('/auth');
        } else {
          return await step.next(step.options);
        }
      },
      async step => {
        const locale = step.context.activity.locale;
        const user = await min.userProfile.get(step.context, {});
        user.isAsking = true;
        if (!user.subjects) {
          user.subjects = [];
        }
        let text: string;

        // Three forms of asking.

        if (step.options && step.options.firstTime) {
          text = Messages[locale].ask_first_time;
        }
        else if (step.options && step.options.isReturning && !step.context.activity.group) {
          text = Messages[locale].anything_else;
        }
        else if (step.context.activity.group || (step.options && step.options.emptyPrompt)) {
          text = '';
        } else if (user.subjects.length > 0) {
          text = Messages[locale].which_question;
        } else {
          throw new Error('Invalid use of /ask');
        }

        return await min.conversationalService.prompt(min, step, text);
      },
      async step => {
        if (step.result) {
          let text = step.result;
          text = text.replace(/<([^>]+?)([^>]*?)>(.*?)<\/\1>/gi, '');

          let sec = new SecService();
          const member = step.context.activity.from;
          const user = await sec.ensureUser(
            min.instance.instanceId,
            member.id,
            member.name,
            '',
            'web',
            member.name,
            null
          );

          let handled = false;
          let nextDialog = null;

          let data = {
            query: text,
            step: step,
            message: text,
            user: user ? user['dataValues'] : null
          };
          await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
            if ((nextDialog = await e.onExchangeData(min, 'handleAnswer', data))) {
              handled = true;
            }
          });
          data.step = null;
          GBLog.info(`/answer being called from getAskDialog.`);
          await step.beginDialog(nextDialog ? nextDialog : '/answer', {
            data: data,
            query: text,
            user: user ? user['dataValues'] : null,
            message: text
          });
        } else {
          return await step.next();
        }
      }
    ];
  }

  private static getAnswerDialog(min: GBMinInstance, service: KBService) {
    return [
      async step => {
        if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
          return await step.beginDialog('/auth');
        } else {
          return await step.next(step.options);
        }
      },
      async step => {
        let answer: GuaribasAnswer = null;
        const member = step.context.activity.from;
        const sec = new SecService();
        let user = await sec.ensureUser(min.instance.instanceId, member.id, member.name, '', 'web', member.name, null);

        const minBoot = GBServer.globals.minBoot as any;

        let text = step.options.query;
        text = text.replace(/<([^>]+?)([^>]*?)>(.*?)<\/\1>/gi, '');

        // When no text is typed, the start dialog is invoked again
        // when people type just the @botName in MSTEAMS for example.

        if (!text) {
          const startDialog = min.core.getParam(min.instance, 'Start Dialog', null);
          if (startDialog) {
            await GBVMService.callVM(startDialog.toLowerCase().trim(), min, step, user, this.deployer, false);
          }

          return step.endDialog();
        }

        const locale = step.context.activity.locale;

        // Stops any content on projector.
        if (step.context.activity.channelId !== 'msteams') {
          await min.conversationalService.sendEvent(min, step, 'stop', undefined);
        }
        // Handle extra text from FAQ.

        if (step.options && step.options.query) {
          text = step.options.query;
        } else if (step.options && step.options.fromFaq) {
          await min.conversationalService.sendText(min, step, Messages[locale].going_answer);
        }

        // Searches KB for the first time.

        const searchScore = min.core.getParam(
          min.instance,
          'Search Score',
          min.instance.searchScore ? min.instance.searchScore : minBoot.instance.searchScore
        );

        // TODO: https://github.com/GeneralBots/BotServer/issues/9 user.lastQuestion = text;

        const resultsA = await service.ask(min.instance, text, searchScore, null /* user.subjects */);

        // If there is some result, answer immediately.

        if (resultsA !== undefined && resultsA.answer !== undefined) {
          // Saves some context info.

          // user.isAsking = false;
          // user.lastQuestionId = resultsA.questionId;

          // Sends the answer to all outputs, including projector.

          answer = resultsA.answer;

          // If this search was restricted to some subjects...
        }
        // TODO: https://github.com/GeneralBots/BotServer/issues/9
        // else if (user.subjects && user.subjects.length > 0) {
        //   // ..second time running Search, now with no filter.

        //   const resultsB = await service.ask(min.instance, text, searchScore, undefined);

        //   // If there is some result, answer immediately.

        //   if (resultsB !== undefined && resultsB.answer !== undefined) {
        //     // Saves some context info.

        //     // user2.isAsking = false;
        //     // user2.lastQuestionId = resultsB.questionId;

        //     // Informs user that a broader search will be used.

        //     if (user2.subjects.length > 0) {
        //       await min.conversationalService.sendText(min, step, Messages[locale].wider_answer);
        //     }

        //     answer = resultsB.answer;
        //   }
        // }

        // Try to answer by search.

        if (answer) {
          return await AskDialog.handleAnswer(service, min, step, user, answer);
        }

        // Tries to answer by NLP.

        let nextDialog = await min.conversationalService.routeNLP(step, min, text);
        if (nextDialog) {
          return nextDialog;
        }

        // Tries to answer by Reading Comprehension.

        if (process.env.GBMODELS_SERVER) {
          text = await min.conversationalService.translate(min, text, 'en');
          let answered = false;

          const docs = await min.kbService['getDocs'](min.instance.instanceId);

          await CollectionUtil.asyncForEach(docs, async (doc: GuaribasAnswer) => {
            if (!answered) {
              const answerText = await min.kbService['readComprehension'](min.instance.instanceId, doc.content, text);
              answered = true;
              text = await min.conversationalService.translate(
                min,
                text,
                user.locale
                  ? user.locale
                  : min.core.getParam<string>(min.instance, 'Locale', GBConfigService.get('LOCALE'))
              );
              await min.conversationalService.sendText(min, step, answerText);
              await min.conversationalService.sendEvent(min, step, 'stop', undefined);
            }
          });
          return await step.replaceDialog('/ask', { isReturning: true });
        }
        if (process.env.OPENAI_EMAIL) {
          if (!GBServer.globals.chatGPT) {
            GBServer.globals.chatGPT = new ChatGPTAPIBrowser({
              email: process.env.OPENAI_EMAIL,
              password: process.env.OPENAI_PASSWORD
            });
            await GBServer.globals.chatGPT.init();
          }
          const CHATGPT_TIMEOUT = 60 * 1000;
          GBLog.info(`ChatGPT being used...`);

          const response = await GBServer.globals.chatGPT.sendMessage(text, { timeoutMs: CHATGPT_TIMEOUT });

          if (!response) {
            GBLog.info(`SEARCH called but NO answer could be found (zero results).`);
          } else {
            await min.conversationalService.sendText(min, step, response);
          }
          return await step.replaceDialog('/ask', { isReturning: true });
        }

        // Not found.

        const message = min.core.getParam<string>(min.instance, 'Not Found Message', Messages[locale].did_not_find);

        await min.conversationalService.sendText(min, step, message);
        return await step.replaceDialog('/ask', { isReturning: true });
      }
    ];
  }

  private static async handleAnswer(service: KBService, min: GBMinInstance, step: any, user, answer: GuaribasAnswer) {
    const text = answer.content;
    if (text.endsWith('.docx')) {
      const mainName = GBVMService.getMethodNameFromVBSFilename(text);
      await step.endDialog();
      return await GBVMService.callVM(mainName, min, step, user, this.deployer, false);
    } else {
      await service.sendAnswer(min, AskDialog.getChannel(step), step, answer);
      return await step.replaceDialog('/ask', { isReturning: true });
    }
  }

  private static getChannel(step): string {
    return !isNaN(step.context.activity['mobile']) ? 'whatsapp' : step.context.activity.channelId;
  }

  private static getAnswerEventDialog(service: KBService, min: GBMinInstance) {
    return [
      async step => {
        if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
          return await step.beginDialog('/auth');
        } else {
          return await step.next(step.options);
        }
      },

      async step => {
        const data = step.options as AskDialogArgs;
        if (data !== undefined && data.questionId !== undefined) {
          const question = await service.getQuestionById(min.instance.instanceId, data.questionId);
          const answer = await service.getAnswerById(min.instance.instanceId, question.answerId);
          // Sends the answer to all outputs, including projector.
          await service.sendAnswer(min, AskDialog.getChannel(step), step, answer);
          await step.replaceDialog('/ask', { isReturning: true });
        }

        return await step.next();
      }
    ];
  }

  private static getLLVMDialog(min: GBMinInstance, service: KBService) {
    return [
      async step => {
        if (step.context.activity.channelId !== 'msteams' && process.env.ENABLE_AUTH) {
          return await step.beginDialog('/auth');
        } else {
          return await step.next(step.options);
        }
      },
      async step => {
        return await min.conversationalService.prompt(min, step, 'Please, describe the dialog scene.');
      },
      async step => {
        step.options.dialog = step.result;
        return await min.conversationalService.prompt(min, step, 'How would you call this?');
      },
      async step => {
        if (GBServer.globals.chatGPT) {
          let input = `Write a BASIC program that ${step.options.dialog.toLowerCase()}. And does not explain.`;

          await min.conversationalService.sendText(min, step, 'Thank you. The dialog is being written right now...');

          const CHATGPT_TIMEOUT = 3 * 60 * 1000;
          GBLog.info(`ChatGPT Code: ${input}`);
          let response = await GBServer.globals.chatGPT.sendMessage(input, {
            timeoutMs: CHATGPT_TIMEOUT
          });

          // Removes instructions, just code.

          response = response.replace(/Copy code/gim, '\n');
          let lines = response.split('\n')
          let filteredLines = lines.filter(line => /\s*\d+\s*.*/.test(line))
          response = filteredLines.join('\n');

          // Gets dialog name and file handling

          let dialogName = step.result.replace('.', '');
          const docx = urlJoin(`${min.botId}.gbdialog`, `${dialogName}.docx`);
          const sys = new SystemKeywords();
          const document = await sys.internalCreateDocument(min, docx, response);
          await service.addQA(min, dialogName, dialogName);

          let message = `Waiting for publishing...`;
          await min.conversationalService.sendText(min, step, message);

          await step.replaceDialog('/publish', { confirm: true });

          message = `Dialog is ready! Let's run:`;
          await min.conversationalService.sendText(min, step, message);


          let sec = new SecService();
          const member = step.context.activity.from;
          const user = await sec.ensureUser(
            min.instance.instanceId,
            member.id,
            member.name,
            '',
            'web',
            member.name,
            null
          );

          await step.endDialog();

          await GBVMService.callVM(dialogName.toLowerCase(),
            min, step, user, this.deployer, false);
        }
      }
    ];
  }
}
