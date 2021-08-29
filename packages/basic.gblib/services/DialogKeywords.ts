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

import { GBDialogStep, GBLog, GBMinInstance } from 'botlib';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService';
import urlJoin = require('url-join');
import { GBServer } from '../../../src/app';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { SecService } from '../../security.gbapp/services/SecService';
import { SystemKeywords } from './SystemKeywords';
var DateDiff = require('date-diff');


/**
 * Base services of conversation to be called by BASIC which
 * requries step variable to work.
 */
export class DialogKeywords {

  /**
  * Reference to minimal bot instance.
  */
  public min: GBMinInstance;

  /**
   * Reference to the base system keywords functions to be called.
   */
  public internalSys: SystemKeywords;

  /**
   * Current user object to get BASIC properties read.
   */
  public user;

  /**
   * When creating this keyword facade, a bot instance is
   * specified among the deployer service.
   */
  constructor(min: GBMinInstance, deployer: GBDeployer, step: GBDialogStep, user) {
    this.min = min;
    this.user = user;
    this.internalSys = new SystemKeywords(min, deployer, this);
  }

  /**
   * Base reference of system keyword facade, called directly
   * by the script.
   */
  public sys(): SystemKeywords {
    return this.internalSys;
  }

  /**
   * Returns the today data filled in dd/mm/yyyy or mm/dd/yyyy.
   *
   * @example x = TODAY
   */
  public async getToday(step) {
    let d = new Date(),
      month = '' + (d.getMonth() + 1),
      day = '' + d.getDate(),
      year = d.getFullYear();

    if (month.length < 2) { month = '0' + month; }
    if (day.length < 2) { day = '0' + day; }

    const contentLocale = this.min.core.getParam<string>(
      this.min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    switch (contentLocale) {
      case 'pt':
        return [day, month, year].join('/');

      case 'en':
        return [month, day, year].join('/');

      default:
        return [year, month, day].join('/');
    }
  }

  /**
   * Quits the dialog, currently required to get out of VM context.
   *
   * @example EXIT
   */
  public async exit(step) {
    await step.endDialog();
  }

  public getContentLocaleWithCulture(contentLocale) {
    switch (contentLocale) {
      case 'pt':
        return 'pt-BR';

      case 'en':
        return 'en-US';

      default:
        return 'en-us';
    }

  }

  /**
   * Returns specified date week day in format 'Mon'.
   *
   * @example day = WEEKDAY (date) 
   *
   */
  public getWeekFromDate(date) {

    const contentLocale = this.min.core.getParam<string>(
      this.min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    let dt = SystemKeywords.getDateFromLocaleString(date, contentLocale);
    GBLog.info(`BASIC WEEKDAY contentLocale: ${this.getContentLocaleWithCulture(contentLocale)}`);
    GBLog.info(`BASIC WEEKDAY date: ${dt}`);
    GBLog.info(dt.toLocaleString(this.getContentLocaleWithCulture(contentLocale), { weekday: 'short' }));

    if (dt) {
      if (!(dt instanceof Date)) {
        dt = new Date(dt);
      }
      let week = dt.toLocaleString(this.getContentLocaleWithCulture(contentLocale), { weekday: 'short' });
      return week.substr(0, 3);
    }
    return 'NULL';
  }

  /**
   * Returns an object ready to get information about difference in several ways
   * like years, months or days.
   *
   * @example days = DATEDIFF date1, date2, mode
   *
   */
  public dateDiff(date1, date2, mode) {
    let dt1 = date1;
    let dt2 = date2;
    if (!(dt1 instanceof Date)) {
      dt1 = new Date(dt1);
    }
    if (!(dt2 instanceof Date)) {
      dt2 = new Date(dt2);
    }
    const diff = new DateDiff(date1, date2);
    switch (mode) {
      case 'year': return diff.years();
      case 'month': return diff.months();
      case 'week': return diff.weeks();
      case 'day': return diff.days();
      case 'hour': return diff.hours();
      case 'minute': return diff.minutes();
    }
  }

  /**
   * Returns specified date week day in format 'Mon'.
   *
   * @example DATEADD date, "minute", 60 
   * 
   * https://stackoverflow.com/a/1214753/18511
   */
  public dateAdd(date, mode, units) {
    let dateCopy = date;
    if (!(dateCopy instanceof Date)) {
      dateCopy = new Date(dateCopy);
    }
    var ret = new Date(dateCopy); //don't change original date
    var checkRollover = function () { if (ret.getDate() != date.getDate()) ret.setDate(0); };
    switch (String(mode).toLowerCase()) {
      case 'year': ret.setFullYear(ret.getFullYear() + units); checkRollover(); break;
      case 'quarter': ret.setMonth(ret.getMonth() + 3 * units); checkRollover(); break;
      case 'month': ret.setMonth(ret.getMonth() + units); checkRollover(); break;
      case 'week': ret.setDate(ret.getDate() + 7 * units); break;
      case 'day': ret.setDate(ret.getDate() + units); break;
      case 'hour': ret.setTime(ret.getTime() + units * 3600000); break;
      case 'minute': ret.setTime(ret.getTime() + units * 60000); break;
      case 'second': ret.setTime(ret.getTime() + units * 1000); break;
      default: ret = undefined; break;
    }
    return ret;
  }



  /**
   * Returns specified list member separated by comma.
   *
   * @example TALK TOLIST (array, member) 
   *
   */
  public getToLst(array, member) {
    if (!array) {
      return "<Empty>"
    }
    if (array[0] && array[0]['gbarray']) {
      array = array.slice(1);
    }
    array = array.filter((v, i, a) => a.findIndex(t => (t[member] === v[member])) === i);
    array = array.filter(function (item, pos) { return item != undefined; });
    array = array.map((item) => { return item[member]; })
    array = array.join(", ");

    return array;
  }

  /**
   * Returns the specified time in format hh:dd.
   *
   * @example hour = HOUR (date)
   *
   */
  public getHourFromDate(date) {
    function addZero(i) {
      if (i < 10) {
        i = "0" + i;
      }
      return i;
    }

    const contentLocale = this.min.core.getParam<string>(
      this.min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    let dt = SystemKeywords.getDateFromLocaleString(date, contentLocale);

    if (dt) {
      if (!(dt instanceof Date)) {
        dt = new Date(dt);
      }
      return addZero(dt.getHours()) + ':' + addZero(dt.getMinutes());
    }
    return 'NULL';
  }

  /**
   * Returns current time in format hh:dd.
   *
   * @example SAVE "file.xlsx", name, email, NOW
   *
   */
  public async getNow() {
    const contentLocale = this.min.core.getParam<string>(
      this.min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    const nowUTC = new Date();
    const now = new Date((typeof nowUTC === 'string' ?
      new Date(nowUTC) :
      nowUTC).toLocaleString(this.getContentLocaleWithCulture(contentLocale),
        { timeZone: process.env.DEFAULT_TIMEZONE }));

    return now.getHours().toString().padStart(2, "0") + ':' + now.getMinutes().toString().padStart(2, "0");
  }

  /**
   * Sends a file to a given mobile.
   *
   * @example SEND FILE TO "+199988887777", "image.jpg", caption
   *
   */
  public async sendFileTo(step, mobile, filename, caption) {
    GBLog.info(`BASIC: SEND FILE TO '${mobile}', filename '${filename}'.`);
    return await this.internalSendFile(null, mobile, filename, caption);
  }

  /**
   * Sends a file to the current user.
   *
   * @example SEND FILE "image.jpg"
   *
   */
  public async sendFile(step, filename, caption) {
    return await this.internalSendFile(step, null, filename, caption);
  }

  /**
   * Defines the current language of the bot conversation.
   *
   * @example SET LANGUAGE "pt"
   *
   */
  public async setLanguage(step, language) {
    const user = await this.min.userProfile.get(step.context, {});

    const sec = new SecService();
    user.systemUser = await sec.updateUserLocale(user.systemUser.userId, language);

    await this.min.userProfile.set(step.context, user);
    this.user = user;
  }

  /**
   * Defines the maximum lines to scan in spreedsheets.
   *
   * @example SET MAX LINES 5000
   *
   */
  public async setMaxLines(step, count) {
    const user = await this.min.userProfile.get(step.context, {});
    user.basicOptions.maxLines = count;
    await this.min.userProfile.set(step.context, user);
    this.user = user;
  }

  /**
   * Defines the FIND behaviour to consider whole words while searching.
   *
   * @example SET WHOLE WORD ON
   *
   */
  public async setWholeWord(step, on) {
    const user = await this.min.userProfile.get(step.context, {});
    user.basicOptions.wholeWord = (on.trim() === "on");
    await this.min.userProfile.set(step.context, user);
    this.user = user;
  }

  /**
   * Defines translator behaviour.
   *
   * @example SET TRANSLATOR ON | OFF
   *
   */
  public async setTranslatorOn(step, on) {
    const user = await this.min.userProfile.get(step.context, {});
    user.basicOptions.translatorOn = (on.trim() === "on");
    await this.min.userProfile.set(step.context, user);
    this.user = user;
  }

  /**
   * Returns the name of the user acquired by WhatsApp API.
   */
  public async userName(step) {
    return step ? step.context.activity.from.name : 'N/A';
  }

  /**
   * OBSOLETE. 
   */
  public async getFrom(step) {
    return step ? await this.userMobile(step) : 'N/A';
  }


  /**
   * Returns current mobile number from user in conversation.
   *
   * @example SAVE "file.xlsx", name, email, MOBILE
   *
   */
  public async userMobile(step) {
    if (!step) {
      return 'N/A';
    }
    if (isNaN(step.context.activity['mobile'])) {
      if (step.context.activity.from && !isNaN(step.context.activity.from.id)) {
        return step.context.activity.from.id;
      }
      return 'No mobile available.';
    } else {
      return step.context.activity['mobile'];
    }
  }

  /**
   * Shows the subject menu to the user
   *
   * @example MENU
   *
   */
  public async showMenu(step) {
    return await step.beginDialog('/menu');
  }

  /**
   * Performs the transfer of the conversation to a human agent.
   *
   * @example TRANSFER
   *
   */
  public async transfer(step) {
    return await step.beginDialog('/t');
  }

  /**
   * Hears something from user and put it in a variable
   *
   * @example HEAR name
   *
   */
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

  /**
   * Talks to the user by using the specified text.
   */
  public async talk(step, text: string) {
    await this.min.conversationalService['sendTextWithOptions'](this.min, step, text,
      this.user.basicOptions.translatorOn, null);
  }

  private static getChannel(step): string {
    if (!isNaN(step.context.activity['mobile'])) {
      return 'webchat';
    } else {
      if (step.context.activity.from && !isNaN(step.context.activity.from.id)) {
        return 'whatsapp';
      }
      return 'webchat';
    }
  }


  /**
   * Processes the sending of the file.
   */
  private async internalSendFile(step, mobile, filename, caption) {
    if (filename.indexOf('.md') > -1) {
      GBLog.info(`BASIC: Sending the contents of ${filename} markdown to mobile ${mobile}.`);
      const md = await this.min.kbService.getAnswerTextByMediaName(this.min.instance.instanceId, filename);
      if (!md) {
        GBLog.info(`BASIC: Markdown file ${filename} not found on database for ${this.min.instance.botId}.`);
      }

      await this.min.conversationalService['playMarkdown'](this.min, md,
        DialogKeywords.getChannel(step), step);
    } else {
      GBLog.info(`BASIC: Sending the file ${filename} to mobile ${mobile}.`);
      const url = urlJoin(
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
}
