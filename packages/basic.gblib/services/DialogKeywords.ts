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

import { TurnContext, BotAdapter } from 'botbuilder';
import { WaterfallStepContext, WaterfallDialog } from 'botbuilder-dialogs';
import { GBLog, GBMinInstance } from 'botlib';
import urlJoin = require('url-join');
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { Messages } from '../strings';
import { GBServer } from '../../../src/app';
import { SecService } from '../../security.gbapp/services/SecService';
import { SystemKeywords } from './SystemKeywords';

/**
 * Base services of conversation to be called by BASIC.
 */
export class DialogKeywords {
  public min: GBMinInstance;
  public context: TurnContext;
  public step: WaterfallStepContext;
  public internalSys: SystemKeywords;

  constructor(min: GBMinInstance, deployer: GBDeployer) {
    this.min = min;
    this.internalSys = new SystemKeywords(min, deployer);
  }

  public sys(): SystemKeywords {
    return this.internalSys;
  }

  public async getToday(step) {
    var d = new Date(),
      month = '' + (d.getMonth() + 1),
      day = '' + d.getDate(),
      year = d.getFullYear();

    if (month.length < 2) month = '0' + month;
    if (day.length < 2) day = '0' + day;

    const locale = step.context.activity.locale;
    switch (locale) {
      case 'pt-BR':
        return [day, month, year].join('/');

      case 'en-US':
        return [month, day, year].join('/');

      default:
        return [year, month, day].join('/');
    }
  }

  public async isAffirmative(text) {
    return text.toLowerCase().match(Messages['pt-BR'].affirmative_sentences); // TODO: Dynamitize.
  }

  public async exit(step) {
    await step.endDialog();
  }

  public async getNow() {
    const nowUTC = new Date();
    const now = new Date((typeof nowUTC === "string" ?
      new Date(nowUTC) :
      nowUTC).toLocaleString("en-US", { timeZone: process.env.DEFAULT_TIMEZONE }));

    return now.getHours() + ':' + now.getMinutes();
  }

  public async sendFileTo(mobile, filename, caption) {
    return await this.internalSendFile(null, mobile, filename, caption);
  }

  public async sendFile(step, filename, caption) {
    return await this.internalSendFile(step, null, filename, caption);
  }

  private async internalSendFile(step, mobile, filename, caption) {
    if (filename.indexOf('.md') > -1) {
      GBLog.info(`BASIC: Sending the contents of ${filename} markdown to mobile.`);
      let md = await this.min.kbService.getAnswerTextByMediaName(this.min.instance.instanceId, filename);
      await this.min.conversationalService.sendMarkdownToMobile(this.min, step, mobile, md);
    } else {
      GBLog.info(`BASIC: Sending the file ${filename} to mobile.`);
      let url = urlJoin(
        GBServer.globals.publicAddress,
        'kb',
        `${this.min.botId}.gbai`,
        `${this.min.botId}.gbkb`,
        'assets',
        filename
      );

      await this.min.conversationalService.sendFile(this.min, step, mobile, url, caption);
    }
  }

  public async setLanguage(step, language) {
    const user = await this.min.userProfile.get(step.context, {});

    let sec = new SecService();
    user.systemUser = await sec.updateUserLocale(user.systemUser.userId, language);

    await this.min.userProfile.set(step.context, user);
  }

  public async from(step) {
    return step.context.activity.from.id;
  }

  public async userName(step) {
    return step.context.activity.from.name;
  }

  public async userMobile(step) {
    if (isNaN(step.context.activity.from.id)) {
      return 'No mobile available.';
    } else {
      return step.context.activity.from.id;
    }
  }

  public async showMenu(step) {
    return await step.beginDialog('/menu');
  }

  public async transfer(step) {
    return await step.beginDialog('/t');
  }

  public async hear(step, promise, previousResolve, kind, ...args) {
    function random(low, high) {
      return Math.random() * (high - low) + low;
    }
    const idPromise = random(0, 120000000);
    this.min.cbMap[idPromise] = {};
    this.min.cbMap[idPromise].promise = promise;

    const opts = { id: idPromise, previousResolve: previousResolve, kind: kind, args };
    if (previousResolve !== undefined) {
      previousResolve(opts);
    } else {
      await step.beginDialog('/hear', opts);
    }
  }

  public async talk(step, text: string) {
    return await this.min.conversationalService.sendText(this.min, step, text);
  }
}
