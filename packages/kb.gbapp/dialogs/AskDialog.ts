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

import { GBServer } from '../../../src/app';
import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBLog, GBMinInstance, IGBDialog, IGBPackage } from 'botlib';
import { Messages } from '../strings';
import { KBService } from './../services/KBService';
import { GuaribasAnswer } from '../models';
import { SecService } from '../../security.gbapp/services/SecService';
import { CollectionUtil, AzureText } from 'pragmatismo-io-framework';
import { GBVMService } from '../../basic.gblib/services/GBVMService';
import { GBImporter } from '../../core.gbapp/services/GBImporterService';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';

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
  }

  private static getAskDialog(min: GBMinInstance) {
    return [
      async step => {
        const locale = step.context.activity.locale;
        const user = await min.userProfile.get(step.context, {});
        user.isAsking = true;
        if (!user.subjects) {
          user.subjects = [];
        }
        let text;
        // Three forms of asking.
        if (step.options && step.options.firstTime) {
          text = Messages[locale].ask_first_time;
        } else if (step.options && step.options.isReturning) {
          text = Messages[locale].anything_else;
        } else if (step.options && step.options.emptyPrompt) {
          text = "";
        } else if (user.subjects.length > 0) {
          text = Messages[locale].which_question;
        } else {
          throw new Error('Invalid use of /ask');
        }
        if (text.length > 0) {
          return await min.conversationalService.prompt(min, step, text);
        }

        return await step.next();
      },
      async step => {
        if (step.result) {
          let text = step.result;
          text = text.replace(/<([^>]+?)([^>]*?)>(.*?)<\/\1>/gi, '');

          let sec = new SecService();
          const member = step.context.activity.from;
          const user = await sec.ensureUser(min.instance.instanceId, member.id, member.name, '', 'web', member.name);

          let handled = false;
          let nextDialog = null;

          await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
            if (
              nextDialog = await e.onExchangeData(min, 'handleAnswer', {
                query: text,
                step: step,
                message: text,
                user: user ? user['dataValues'] : null
              })
            ) {
              handled = true;
            }
          });

          await step.beginDialog(nextDialog ? nextDialog : '/answer', {
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
        let answer: GuaribasAnswer = null;
        const user = await min.userProfile.get(step.context, {});

        const minBoot = GBServer.globals.minBoot as any;

        let text = step.options.query;
        text = text.replace(/<([^>]+?)([^>]*?)>(.*?)<\/\1>/gi, '');

        // When no text is typed, the start dialog is invoked again
        // when people type just the @botName in MSTEAMS for example.

        if (!text) {
          const startDialog =
            min.core.getParam(min.instance, 'Start Dialog', null);
          await GBVMService.callVM(startDialog.toLowerCase(), min, step, this.deployer);

          return step.endDialog();
        }


        const locale = step.context.activity.locale;

        // Stops any content on projector.

        await min.conversationalService.sendEvent(min, step, 'stop', undefined);

        // Handle extra text from FAQ.

        if (step.options && step.options.query) {
          text = step.options.query;
        } else if (step.options && step.options.fromFaq) {
          await min.conversationalService.sendText(min, step, Messages[locale].going_answer);
        }

        // Searches KB for the first time.

        const searchScore = min.core.getParam(min.instance, 'Search Score',
          min.instance.searchScore ? min.instance.searchScore : minBoot.instance.searchScore);

        user.lastQuestion = text;
        await min.userProfile.set(step.context, user);

        const resultsA = await service.ask(min.instance, text, searchScore, user.subjects);

        // If there is some result, answer immediately.

        if (resultsA !== undefined && resultsA.answer !== undefined) {
          // Saves some context info.

          user.isAsking = false;
          user.lastQuestionId = resultsA.questionId;
          await min.userProfile.set(step.context, user);

          // Sends the answer to all outputs, including projector.

          answer = resultsA.answer;

          // If this search was restricted to some subjects...
        } else if (user.subjects && user.subjects.length > 0) {
          // ...second time running Search, now with no filter.

          const resultsB = await service.ask(min.instance, text, searchScore, undefined);

          // If there is some result, answer immediately.

          if (resultsB !== undefined && resultsB.answer !== undefined) {
            // Saves some context info.

            const user2 = await min.userProfile.get(step.context, {});
            user2.isAsking = false;
            user2.lastQuestionId = resultsB.questionId;
            await min.userProfile.set(step.context, user2);

            // Informs user that a broader search will be used.

            if (user2.subjects.length > 0) {
              await min.conversationalService.sendText(min, step, Messages[locale].wider_answer);
            }

            answer = resultsB.answer;
          }
        }

        // Answers using Search or NLP responses.

        if (answer) {
          return await AskDialog.handleAnswer(service, min, step, answer);
        } else if (!(await min.conversationalService.routeNLP(step, min, text))) {
          const message = min.core.getParam<string>(min.instance, 'Not Found Message',
            Messages[locale].did_not_find);

          await min.conversationalService.sendText(min, step, message);
          return await step.replaceDialog('/ask', { isReturning: true });
        }
      }
    ];
  }

  private static async handleAnswer(service: KBService, min: GBMinInstance, step: any, answer: GuaribasAnswer) {
    const text = answer.content;
    if (text.endsWith('.docx')) {
      const mainName = GBVMService.getMethodNameFromVBSFilename(text);
      return await GBVMService.callVM(mainName, min, step, this.deployer);
    } else {
      await service.sendAnswer(min, AskDialog.getChannel(step), step, answer);
      return await step.replaceDialog('/ask', { isReturning: true });
    }
  }

  private static getChannel(step): string {
    return !isNaN(step.context.activity.from.id) ? 'whatsapp' : step.context.activity.channelId;
  }

  private static getAnswerEventDialog(service: KBService, min: GBMinInstance) {
    return [
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
}
