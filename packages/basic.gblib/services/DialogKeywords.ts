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
const urlJoin = require('url-join');
import { GBServer } from '../../../src/app';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { SecService } from '../../security.gbapp/services/SecService';
import { SystemKeywords } from './SystemKeywords';
import { GBMinService } from '../../core.gbapp/services/GBMinService';
import { HubSpotServices } from '../../hubspot.gblib/services/HubSpotServices';
import { WhatsappDirectLine } from '../../whatsapp.gblib/services/WhatsappDirectLine';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import * as fs from 'fs';
const DateDiff = require('date-diff');
const puppeteer = require('puppeteer');
const Path = require('path');

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
   * HTML browser for conversation over page interaction.
   */
  browser: any;

  /**
   * The number used in this execution for HEAR calls (useful for SET SCHEDULE).
   */
  hrOn: string;

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
   * Returns the page object.
   *
   * @example x = GET PAGE
   */
  public async getPage(step, url) {

    if (!this.browser) {
      this.browser = await puppeteer.launch({
        args: [
          '--ignore-certificate-errors',
          '--no-sandbox',
          '--disable-setuid-sandbox',
          '--window-size=1920,1080',
          "--disable-accelerated-2d-canvas",
          "--disable-gpu"],
        ignoreHTTPSErrors: true,
        headless: false,
      });
    }
    const page = await this.browser.newPage();
    await page.goto(url);
    return page;
  }

  /**
   * Find element on page DOM.
   *
   * @example GET page, "elementName", "text"
   */
  public async getBySelector(page, elementName) {
    await page.waitForSelector(elementName)
    let element = await page.$(elementName);
    return element;
  }

  /**
   * Find element on page DOM.
   *
   * @example GET page, "frameSelector, "elementSelector"
   */
  public async getByFrame(page, frame, selector) {
    await page.waitForSelector(frame)
    let frameHandle = await page.$(frame);
    const f = await frameHandle.contentFrame();
    await f.waitForSelector(selector);
    const element = await f.$(selector);
    return element;
  }

  /**
   * Returns the today data filled in dd/mm/yyyy or mm/dd/yyyy. 
   *
   * @example x = TODAY
   */
  public async click(step, page, idOrName) {
    const e = await this.getBySelector(page, idOrName);

    await Promise.all([
      page.waitForNavigation(),
      page.click(e.name)
    ]);
  }


  /**
   * Returns the screenshot of page or element
   *
   * @example file = SCREENSHOT page
   */
  public async screenshot(step, page, idOrName, localName) {
    const e = await this.getBySelector(page, idOrName);
    await e.screenshot({ path: localName });
  }

  /**
   * Performs the download to the .gbdrive Download folder.
   *
   * @example file = DOWNLOAD page, "tableName", row
   */
  public async download(step, page, idOrName, localName) {

    const e = await this.getBySelector(page, idOrName);
    const context = await this.browser.newContext({ acceptDownloads: true });

    var cells = e.rows[0].cells;

    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.click(cells[0])
    ]);

    const path = await download.path();

    console.log(path);
  }

  /**
   * Types the text into the text field.
   *
   * @example TYPE page, "elementName", "text"
   */
  public async type(step, page, idOrName, text) {
    const e = await this.getBySelector(page, idOrName);
    await e.type(text);
  }

  /**
   * Returns the today data filled in dd/mm/yyyy or mm/dd/yyyy.
   *
   * @example x = TODAY
   */
  public async getOCR(step, localFile) {
    const tesseract = require("node-tesseract-ocr")

    const config = {
      lang: "eng",
      oem: 1,
      psm: 3,
    }

    return await tesseract.recognize(localFile, config);
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
    if (this.browser) {
      await this.browser.close();
    }
    await step.endDialog();
  }

  /**
   * Get active tasks.
   *
   * @example list = ACTIVE TASKS
   */
  public async getActiveTasks() {
    let s = new HubSpotServices(null, null, process.env.HUBSPOT_KEY);
    return await s.getActiveTasks();
  }

  /**
   * Creates a new deal.
   *
   * @example CREATE DEAL dealname, contato, empresa, amount
   */
  public async createDeal(dealName, contact, company, amount) {
    let s = new HubSpotServices(null, null, process.env.HUBSPOT_KEY);
    let deal = await s.createDeal(dealName, contact, company, amount);
    return deal;
  }

  /**
   * Finds contacts in XRM.
   *
   * @example list = FIND CONTACT "Sandra"
   */
  public async fndContact(name) {
    let s = new HubSpotServices(null, null, process.env.HUBSPOT_KEY);
    return await s.searchContact(name);
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
    const now = typeof nowUTC === 'string' ?
      new Date(nowUTC) :
      nowUTC;

    const nowText = now.toLocaleString(this.getContentLocaleWithCulture(contentLocale),
      { timeZone: process.env.DEFAULT_TIMEZONE });

    return /\b([0-9]|0[0-9]|1?[0-9]|2[0-3]):[0-5]?[0-9]/.exec(nowText)[0];
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
    const mobile = await this.userMobile(step);
    GBLog.info(`BASIC: SEND FILE (current: ${mobile}, filename '${filename}'.`);
    return await this.internalSendFile(step, mobile, filename, caption);
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
    return step ? WhatsappDirectLine.usernames[await this.userMobile(step)] : 'N/A';
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
    return GBMinService.userMobile(step);
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
  public async transferTo(step, to: string = null) {
    return await step.beginDialog('/t', { to: to });
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

    let opts = { id: idPromise, previousResolve: previousResolve, kind: kind, args };

    if (this.hrOn) {

      let sleep = ms => {
        return new Promise(resolve => {
          setTimeout(resolve, ms);
        });
      };

      // Waits for next message in HEAR delegated context.

      const mobile = await this.userMobile(step);
      while (true) {
        if (WhatsappDirectLine.state[mobile] === 3) {
          break;
        }
        sleep(5000);
      }
      const result = WhatsappDirectLine.lastMessage[mobile];
      opts = await promise(step, result);

      if (previousResolve !== undefined) {
        previousResolve(opts);
      }
    }
    else {

      if (previousResolve !== undefined) {
        previousResolve(opts);
      } else {
        await step.beginDialog('/hear', opts);
      }
    }
  }

  /**
   * Prepares the next dialog to be shown to the specified user.
   */
  public async gotoDialog(step, fromOrDialogName: string, dialogName: string) {
    if (dialogName) {
      if (dialogName.charAt(0) === '/') {
        await step.beginDialog(fromOrDialogName);
      } else {
        let sec = new SecService();
        let user = await sec.getUserFromSystemId(fromOrDialogName);
        if (!user) {
          user = await sec.ensureUser(this.min.instance.instanceId, fromOrDialogName,
            fromOrDialogName, null, 'whatsapp', 'from', null);
        }
        await sec.updateUserHearOnDialog(user.userId, dialogName);
      }
    }
    else {
      await step.beginDialog(fromOrDialogName);
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
    if (!step) return 'whatsapp';
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

    // Handles SEND FILE TO mobile, element in Web Automation.

    const page = filename._page;
    if (page) {
      const gbaiName = `${this.min.botId}.gbai`;
      const localName = Path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.jpg`);
      await filename.screenshot({ path: localName });

      const url = urlJoin(
        GBServer.globals.publicAddress,
        this.min.botId,
        'cache',
        Path.basename(localName)
      );

      GBLog.info(`BASIC: WebAutomation: Sending the file ${url} to mobile ${mobile}.`);
      await this.min.conversationalService.sendFile(this.min, step, mobile, url, caption);
    }

    // Handles Markdown.

    else if (filename.indexOf('.md') > -1) {
      GBLog.info(`BASIC: Sending the contents of ${filename} markdown to mobile ${mobile}.`);
      const md = await this.min.kbService.getAnswerTextByMediaName(this.min.instance.instanceId, filename);
      if (!md) {
        GBLog.info(`BASIC: Markdown file ${filename} not found on database for ${this.min.instance.botId}.`);
      }

      await this.min.conversationalService['playMarkdown'](this.min, md,
        DialogKeywords.getChannel(step), step, mobile);

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
