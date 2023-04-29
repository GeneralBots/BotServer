/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__,\| (˅) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model,this program can be used either      |
| under the terms of the GNU Affero General Public License,version 3,       |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,            |
| but WITHOUT ANY WARRANTY,without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights,title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBLog, GBMinInstance } from 'botlib';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { ChartServices } from './ChartServices.js';
import urlJoin from 'url-join';
import { GBServer } from '../../../src/app.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { SecService } from '../../security.gbapp/services/SecService.js';
import { SystemKeywords } from './SystemKeywords.js';
import * as wpp from 'whatsapp-web.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { Messages } from '../strings.js';
import * as Fs from 'fs';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService.js';
import phoneUtil from 'google-libphonenumber';
import phone from 'phone';
import DateDiff from 'date-diff';
import tesseract from 'node-tesseract-ocr';
import Path from 'path';
import sgMail from '@sendgrid/mail';
import mammoth from 'mammoth';
import qrcode from 'qrcode';
import { WebAutomationServices } from './WebAutomationServices.js';
import urljoin from 'url-join';
import QrScanner from 'qr-scanner';
import pkg from 'whatsapp-web.js';
import { ActivityTypes } from 'botbuilder';
const { List, Buttons } = pkg;
import mime from 'mime';

/**
 * Default check interval for user replay
 */
const DEFAULT_HEAR_POLL_INTERVAL = 500;

/**
 * Base services of conversation to be called by BASIC.
 */
export class DialogKeywords {
  /**
   *
   * Data = [10,20,30]
   * Legends = "Steve;Yui;Carlos"
   * img = CHART "pie",data,legends
   *
   * https://c3js.org/examples.html
   * https://c3js.org/samples/timeseries.html (used here)
   *
   * @param data
   * @param legends
   * @see https://www.npmjs.com/package/plot
   */
  public async chart({ pid, type, data, legends, transpose }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    let table = [[]];

    if (legends) {
      const legends_ = legends.split(';');

      // Columns and data are merged like:
      //     columns: [
      //       ['data1',30,200,100,400,150,250],
      //       ['data2',50,20,10,40,15,25]
      //     ]

      for (let i = 0; i < legends_.length; i++) {
        table[i] = [legends_[i]];
        table[i] = table[i].concat(data);
      }
    } else {
      table = SystemKeywords.JSONAsGBTable(data, false);
      table.shift();
    }

    if (transpose) {
      const transpose = array => {
        return array.reduce((prev, next) => next.map((item, i) => (prev[i] || []).concat(next[i])), []);
      };
      table = transpose(table);
    }

    let definition = {
      size: {
        height: 420,
        width: 680
      },
      data: {
        columns: table,
        type: type
      },
      bar: {
        ratio: 0.5
      }
    };

    if (type === 'timeseries') {
      definition['axis'][table[0]] = {
        type: 'timeseries',
        tick: {
          format: '%Y-%m-%d'
        }
      };
    }

    const gbaiName = DialogKeywords.getGBAIPath(min.botId);
    const localName = Path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.jpg`);

    await ChartServices.screenshot(definition, localName);

    const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));

    GBLog.info(`BASIC: Visualization: Chart generated at ${url}.`);

    return url;
  }

  /**
   * Returns the OCR of image file.
   *
   */
  public async getOCR({ localFile }) {
    GBLog.info(`BASIC: OCR processing on ${localFile}.`);

    const config = {
      lang: 'eng',
      oem: 1,
      psm: 3
    };

    return await tesseract.recognize(localFile, config);
  }

  /**
   * Returns the today data filled in dd/mm/yyyy or mm/dd/yyyy.
   *
   * @example x = TODAY
   */
  public async getToday({ pid }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    let d = new Date(),
      month = '' + (d.getMonth() + 1),
      day = '' + d.getDate(),
      year = d.getFullYear();

    if (month.length < 2) {
      month = '0' + month;
    }
    if (day.length < 2) {
      day = '0' + day;
    }

    const contentLocale = min.core.getParam(
      min.instance,
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
   * Quits the dialog,currently required to get out of VM context.
   *
   * @example EXIT
   */
  public async exit({}) {}

  /**
   * Get active tasks.
   *
   * @example list = ACTIVE TASKS
   */
  public async getActiveTasks({ pid }) {}

  /**
   * Creates a new deal.
   *
   * @example CREATE DEAL dealname,contato,empresa,amount
   */
  public async createDeal({ pid, dealName, contact, company, amount }) {}

  /**
   * Finds contacts in XRM.
   *
   * @example list = FIND CONTACT "Sandra"
   */
  public async fndContact({ pid, name }) {}

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

  public async getCoded({ pid, value }) {
    // Checks if it is a GB FILE object.

    if (value.data && value.filename) {
      value = value.data;
    }

    return Buffer.from(value).toString('base64');
  }

  /**
   * Returns specified date week day in format 'Mon'.
   *
   * @example day = WEEKDAY (date)
   *
   */
  public async getWeekFromDate({ pid, date }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    let dt = SystemKeywords.getDateFromLocaleString(pid, date, contentLocale);
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
   * like years,months or days.
   *
   * @example days = DATEDIFF date1,date2,mode
   *
   */
  public async dateDiff(date1, date2, mode) {
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
      case 'year':
        return diff.years();
      case 'month':
        return diff.months();
      case 'week':
        return diff.weeks();
      case 'day':
        return diff.days();
      case 'hour':
        return diff.hours();
      case 'minute':
        return diff.minutes();
    }
  }

  /**
   * Returns specified date week day in format 'Mon'.
   *
   * @example DATEADD date,"minute",60
   *
   * https://stackoverflow.com/a/1214753/18511
   */
  public dateAdd(date, mode, units) {
    let dateCopy = date;
    if (!(dateCopy instanceof Date)) {
      dateCopy = new Date(dateCopy);
    }
    var ret = new Date(dateCopy); //don't change original date
    var checkRollover = function () {
      if (ret.getDate() != date.getDate()) ret.setDate(0);
    };
    switch (String(mode).toLowerCase()) {
      case 'year':
        ret.setFullYear(ret.getFullYear() + units);
        checkRollover();
        break;
      case 'quarter':
        ret.setMonth(ret.getMonth() + 3 * units);
        checkRollover();
        break;
      case 'month':
        ret.setMonth(ret.getMonth() + units);
        checkRollover();
        break;
      case 'week':
        ret.setDate(ret.getDate() + 7 * units);
        break;
      case 'day':
        ret.setDate(ret.getDate() + units);
        break;
      case 'hour':
        ret.setTime(ret.getTime() + units * 3600000);
        break;
      case 'minute':
        ret.setTime(ret.getTime() + units * 60000);
        break;
      case 'second':
        ret.setTime(ret.getTime() + units * 1000);
        break;
      default:
        ret = undefined;
        break;
    }
    return ret;
  }

  /**
   * Returns specified list member separated by comma.
   *
   * @example TALK TOLIST (array,member)
   *
   */
  public async getToLst(pid, array, member) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    if (!array) {
      return '<Empty>';
    }
    if (array[0] && array[0]['gbarray']) {
      array = array.slice(1);
    }
    array = array.filter((v, i, a) => a.findIndex(t => t[member] === v[member]) === i);
    array = array.filter(function (item, pos) {
      return item != undefined;
    });
    array = array.map(item => {
      return item[member];
    });
    array = array.join(',');

    return array;
  }

  /**
   * Returns the specified time in format hh:dd.
   *
   * @example hour = HOUR (date)
   *
   */
  public async getHourFromDate(pid, date) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    function addZero(i) {
      if (i < 10) {
        i = '0' + i;
      }
      return i;
    }

    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    let dt = SystemKeywords.getDateFromLocaleString(pid, date, contentLocale);

    if (dt) {
      if (!(dt instanceof Date)) {
        dt = new Date(dt);
      }
      return addZero(dt.getHours()) + ':' + addZero(dt.getMinutes());
    }
    return 'NULL';
  }

  /**
   * Returns current time in format hh:mm.
   *
   * @example NOW
   *
   */
  public async getNow({ pid }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    const nowUTC = new Date();
    const now = typeof nowUTC === 'string' ? new Date(nowUTC) : nowUTC;

    const nowText = now.toLocaleString(this.getContentLocaleWithCulture(contentLocale), {
      timeZone: process.env.DEFAULT_TIMEZONE
    });

    return /\b([0-9]|0[0-9]|1?[0-9]|2[0-3]):[0-5]?[0-9]/.exec(nowText)[0];
  }

  /**
   * Sends an e-mail.
   *
   * @example
   *
   * SEND MAIL "email@domain.com","Subject", "Message text."
   *
   */
  public async sendEmail({ pid, to, subject, body }) {
    // tslint:disable-next-line:no-console

    GBLog.info(`[E-mail]: to:${to},subject: ${subject},body: ${body}.`);
    const emailToken = process.env.EMAIL_API_KEY;

    // Inline word document used as e-mail body.

    if (typeof body === 'object') {
      const result = await mammoth.convertToHtml({ buffer: body });
      body = result.value;
    }

    return new Promise<any>((resolve, reject) => {
      sgMail.setApiKey(emailToken);
      const msg = {
        to: to,
        from: process.env.EMAIL_FROM,
        subject: subject,
        text: body,
        html: body
      };
      sgMail.send(msg, false, (err, res) => {
        if (err) {
          reject(err);
        } else {
          resolve(res);
        }
      });
    });
  }

  /**
   * Sends a file to a given mobile.
   *
   * @example SEND FILE TO "+199988887777","image.jpg",caption
   *
   */
  public async sendFileTo({ pid, mobile, filename, caption }) {
    GBLog.info(`BASIC: SEND FILE TO '${mobile}',filename '${filename}'.`);
    return await this.internalSendFile({ pid, mobile, channel: null, filename, caption });
  }

  /**
   * Sends a file to the current user.
   *
   * @example SEND FILE "image.jpg"
   *
   */
  public async sendFile({ pid, filename, caption }) {
    const { min, user, proc } = await DialogKeywords.getProcessInfo(pid);
    GBLog.info(`BASIC: SEND FILE (to: ${user.userSystemId},filename '${filename}'.`);
    const mobile = await this.userMobile({ pid });
    return await this.internalSendFile({ pid, channel: proc.channel, mobile, filename, caption });
  }

  /**
   * Defines the current language of the bot conversation.
   *
   * @example SET LANGUAGE "pt"
   *
   */
  public async setLanguage({ pid, language }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const sec = new SecService();
    await sec.updateUserLocale(user.userId, language);
  }

  /**
   * Defines the id generation policy.
   *
   * @example SET ID NUMBER
   *
   */
  public async setIdGeneration({ mode }) {
    this['idGeneration'] = mode;
    this['id'] = new SystemKeywords().getRandomId();
  }

  public static isUserSystemParam(name: string): Boolean {
    const names = [
      'welcomed',
      'loaded',
      'subjects',
      'cb',
      'welcomed',
      'maxLines',
      'translatorOn',
      'wholeWord',
      'theme',
      'maxColumns'
    ];

    return names.indexOf(name) > -1;
  }

  public static async setOption({ pid, name, value }) {
    // if (this.isUserSystemParam(name)) {
    //   throw new Error(`Not possible to define ${name} as it is a reserved system param name.`);
    // }
    let { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    const sec = new SecService();
    await sec.setParam(user.userId, name  , value);
    GBLog.info(`BASIC: ${name} = ${value} (botId: ${min.botId})`);
    return { min, user, params };
  }

  public static async getOption({ pid, name }) {
    if (this.isUserSystemParam(name)) {
      throw new Error(`Not possible to retrieve ${name} system param.`);
    }
    let { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    const sec = new SecService();
    return await sec.getParam(user, name);
  }

  /**
   * Defines the maximum lines to scan in spreedsheets.
   *
   * @example SET MAX LINES 5000
   *
   */
  public async setMaxLines({ pid, count }) {
    await DialogKeywords.setOption({ pid, name: 'maxLines', value: count });
  }

  /**
   * Defines a custom user param to be persisted to storage.
   *
   * @example SET PARAM name AS value
   *
   */
  public async setUserParam({ pid, name, value }) {
    await DialogKeywords.setOption({ pid, name, value });
  }

  /**
   * Returns a custom user param persisted on storage.
   *
   * @example GET PARAM name
   *
   */
  public async getUserParam({ pid, name }) {
    await DialogKeywords.getOption({ pid, name });
  }

  /**
   * Defines the maximum lines to scan in spreedsheets.
   *
   * @example SET MAX COLUMNS 5000
   *
   */
  public async setMaxColumns({ pid, count }) {
    await DialogKeywords.setOption({ pid, name: 'setMaxColumns', value: count });
  }

  /**
   * Defines a custom user filter for SET calls.
   *
   * @example SET FILTER "ColumnName=33"
   *          SET "file.xlsx", "C", "200000"
   *
   */
  public async setFilter({ pid, value }) {
    await DialogKeywords.setOption({ pid, name: 'filter', value });
  }

  /**
   * Defines the FIND behaviour to consider whole words while searching.
   *
   * @example SET WHOLE WORD ON
   *
   */
  public async setWholeWord({ pid, on }) {
    const value = on.trim() === 'on';
    await DialogKeywords.setOption({ pid, name: 'wholeWord', value: value });
  }

  /**
   * Defines the FIND behaviour to consider whole words while searching.
   *
   * @example SET FILTER TYPE date, string
   *
   */
  public async setFilterTypes({ pid, types }) {
    const value = types;
    await DialogKeywords.setOption({ pid, name: 'filterTypes', value: value });
  }

  /**
   * Defines the theme for assets generation.
   *
   * @example SET THEME "themename"
   *
   */
  public async setTheme({ pid, theme }) {
    const value = theme.trim();
    await DialogKeywords.setOption({ pid, name: 'theme', value: value });
  }

  /**
   * Defines translator behaviour.
   *
   * @example SET TRANSLATOR ON | OFF
   *
   */
  public async setTranslatorOn({ pid, on }) {
    const value = on.trim() === 'on';
    await DialogKeywords.setOption({ pid, name: 'translatorOn', value: value });
  }

  /**
   * Returns the name of the user acquired by WhatsApp API.
   */
  public async userName({ pid }) {
    let { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    return user.userName;
  }

  /**
   * Returns current mobile number from user in conversation.
   */
  public async userMobile({ pid }) {
    let { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    return user.userSystemId;
  }

  /**
   * Shows the subject menu to the user
   *
   * @example MENU
   *
   */
  public async showMenu({}) {
    // https://github.com/GeneralBots/BotServer/issues/237
    // return await beginDialog('/menu');
  }

  /**
   * Performs the transfer of the conversation to a human agent.
   *
   * @example TRANSFER
   *
   */
  public async transferTo({ to }) {
    // https://github.com/GeneralBots/BotServer/issues/150
    // return await beginDialog('/t',{ to: to });
  }

  public static getFileByHandle(hash) {
    return GBServer.globals.files[hash];
  }

  /**
   * Hears something from user and put it in a variable
   *
   * @example HEAR name
   *
   */
  public async hear({ pid, kind, args }) {
    let { min, user, params } = await DialogKeywords.getProcessInfo(pid);

    // Handles first arg as an array of args.

    let args1 = [];
    if (args && args.length) {
      args1 = args;
    }
    args = args1;

    try {
      const isIntentYes = (locale, utterance) => {
        return utterance.toLowerCase().match(Messages[locale].affirmative_sentences);
      };

      const sec = new SecService();

      // If SET HEAR ON is defined an impersonated context is created
      // containing the specified user other than the actual user
      // TODO: Store hrOn in processInfo.

      if (params.hrOn) {
        user = await sec.getUserFromAgentSystemId(params.hrOn);
      }

      const userId = user.userId;
      let result;

      const locale = user.locale ? user.locale : 'en-US';
      // https://github.com/GeneralBots/BotServer/issues/266

      if (args && args.length > 1) {
        let i = 0;

        if (args.length > 3) {
          let section = { title: '', rows: [] };
          await CollectionUtil.asyncForEach(args, async arg => {
            i++;
            section.rows.push({ title: arg, id: `button${i}` });
          });
          const list = new List('Select:', '', [section], '', '');
          await this.talk({ pid: pid, text: list });
        } else {
          let buttons = [];
          await CollectionUtil.asyncForEach(args, async arg => {
            i++;
            buttons.push({ body: arg, id: `button${i}` });
          });
          let button = new Buttons('Select:', buttons, '', 'General Bots');
          await this.talk({ pid: pid, text: button });
        }

        GBLog.info(`BASIC: HEAR with [${args.toString()}] (Asking for input).`);
      } else {
        GBLog.info('BASIC: HEAR (Asking for input).');
      }

      // Wait for the user to answer.

      let sleep = ms => {
        return new Promise(resolve => {
          setTimeout(resolve, ms);
        });
      };
      min.cbMap[userId] = {};
      min.cbMap[userId]['promise'] = '!GBHEAR';

      while (min.cbMap[userId].promise === '!GBHEAR') {
        await sleep(DEFAULT_HEAR_POLL_INTERVAL);
      }

      const answer = min.cbMap[userId].promise;

      if (!kind) {
        result = answer;
      } else if (kind === 'sheet') {
        // Retrieves the .xlsx file associated with the HEAR var AS file.xlsx.

        let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
        const botId = min.instance.botId;
        const path = DialogKeywords.getGBAIPath(botId);
        let url = `${baseUrl}/drive/root:/${path}:/children`;

        GBLog.info(`Loading HEAR AS .xlsx options from Sheet: ${url}`);
        const res = await client.api(url).get();

        // Finds .xlsx specified by arg.

        const document = res.value.filter(m => {
          return m.name === args;
        });
        if (document === undefined || document.length === 0) {
          GBLog.info(`${args} not found on .gbdata folder, check the package.`);
          return null;
        }

        // Reads all rows to be used as menu items in HEAR validation.

        let sheets = await client.api(`${baseUrl}/drive/items/${document[0].id}/workbook/worksheets`).get();
        const results = await client
          .api(
            `${baseUrl}/drive/items/${document[0].id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A1:A256')`
          )
          .get();

        // Builds an array of items found in sheet file.

        let index = 0;
        let list = [];
        for (; index < results.text.length; index++) {
          if (results.text[index][0] !== '') {
            list.push(results.text[index][0]);
          } else {
            break;
          }
        }

        // Search the answer in one of valid list items loaded from sheeet.

        result = null;
        await CollectionUtil.asyncForEach(list, async item => {
          if (GBConversationalService.kmpSearch(answer, item) != -1) {
            result = item;
          }
        });

        // In case of unmatch, asks the person to try again.

        if (result === null) {
          await this.talk({ pid, text: `Escolha por favor um dos itens sugeridos.` });
          return await this.hear({ pid, kind, args });
        }
      } else if (kind === 'file') {
        GBLog.info(`BASIC (${min.botId}): Upload done for ${answer.filename}.`);
        const handle = WebAutomationServices.cyrb53(min.botId + answer.filename);
        GBServer.globals.files[handle] = answer;
        result = handle;
      } else if (kind === 'boolean') {
        if (isIntentYes('pt-BR', answer)) {
          result = true;
        } else {
          result = false;
        }
      } else if (kind === 'email') {
        const extractEntity = text => {
          return text.match(/([a-zA-Z0-9._-]+@[a-zA-Z0-9._-]+\.[a-zA-Z0-9_-]+)/gi);
        };

        const value = extractEntity(answer);

        if (value === null) {
          await this.talk({ pid, text: 'Por favor, digite um e-mail válido.' });
          return await this.hear({ pid, kind, args });
        }

        result = value;
      } else if (kind === 'name') {
        const extractEntity = text => {
          return text.match(/[_a-zA-Z][_a-zA-Z0-9]{0,16}/gi);
        };

        const value = extractEntity(answer);

        if (value === null || value.length != 1) {
          await this.talk({ pid, text: 'Por favor, digite um nome válido.' });
          return await this.hear({ pid, kind, args });
        }

        result = value;
      } else if (kind === 'integer') {
        const extractEntity = text => {
          return text.match(/\d+/gi);
        };

        const value = extractEntity(answer);

        if (value === null || value.length != 1) {
          await this.talk({ pid, text: 'Por favor, digite um número válido.' });
          return await this.hear({ pid, kind, args });
        }

        result = value;
      } else if (kind === 'date') {
        const extractEntity = text => {
          return text.match(
            /(^(((0[1-9]|1[0-9]|2[0-8])[\/](0[1-9]|1[012]))|((29|30|31)[\/](0[13578]|1[02]))|((29|30)[\/](0[4,6,9]|11)))[\/](19|[2-9][0-9])\d\d$)|(^29[\/]02[\/](19|[2-9][0-9])(00|04|08|12|16|20|24|28|32|36|40|44|48|52|56|60|64|68|72|76|80|84|88|92|96)$)/gi
          );
        };

        const parseDate = str => {
          function pad(x){return (((''+x).length==2) ? '' : '0') + x; }
          var m = str.match(/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/)
            , d = (m) ? new Date(m[3], m[2]-1, m[1]) : null
            , matchesPadded = (d&&(str==[pad(d.getDate()),pad(d.getMonth()+1),d.getFullYear()].join('/')))
            , matchesNonPadded = (d&&(str==[d.getDate(),d.getMonth()+1,d.getFullYear()].join('/')));
          return (matchesPadded || matchesNonPadded) ? d : null;
        }

        let value = parseDate(answer);

        if (value === null) {
          await this.talk({ pid, text: 'Por favor, digite uma data no formato 12/12/2020.' });
          return await this.hear({ pid, kind, args });
        }
        value = new Date(value);
        result = value.toLocaleString('pt-BR', { day: '2-digit', month: '2-digit', year: 'numeric' });
      } else if (kind === 'hour') {
        const extractEntity = (text: string) => {
          return text.match(/^([0-1]?[0-9]|2[0-4]):([0-5][0-9])(:[0-5][0-9])?$/gi);
        };

        const value = extractEntity(answer);

        if (value === null || value.length != 1) {
          await this.talk({ pid, text: 'Por favor, digite um horário no formato hh:ss.' });
          return await this.hear({ pid, kind, args });
        }

        result = value;
      } else if (kind === 'money') {
        const extractEntity = (text: string) => {
          // https://github.com/GeneralBots/BotServer/issues/307
          if (user.locale === 'en') {
            return text.match(/(?:\d{1,3},)*\d{1,3}(?:\.\d+)?/gi);
          } else {
            return text.match(/(?:\d{1,3}.)*\d{1,3}(?:\,\d+)?/gi);
          }
          return [];
        };

        const value = extractEntity(answer);

        if (value === null || value.length != 1) {
          await this.talk({ pid, text: 'Por favor, digite um valor monetário.' });
          return await this.hear({ pid, kind, args });
        }

        result = value;
      } else if (kind === 'mobile') {
        let phoneNumber;
        try {
          // https://github.com/GeneralBots/BotServer/issues/307
          phoneNumber = phone(answer, { country: 'BRA' })[0];
          phoneNumber = phoneUtil.parse(phoneNumber);
        } catch (error) {
          await this.talk({ pid, text: Messages[locale].validation_enter_valid_mobile });

          return await this.hear({ pid, kind, args });
        }
        if (!phoneUtil.isPossibleNumber(phoneNumber)) {
          await this.talk({ pid, text: 'Por favor, digite um número de telefone válido.' });
          return await this.hear({ pid, kind, args });
        }

        result = phoneNumber;
      } else if (kind === 'qr-scanner') {
        //https://github.com/GeneralBots/BotServer/issues/171
        GBLog.info(`BASIC (${min.botId}): Upload done for ${answer.filename}.`);
        const handle = WebAutomationServices.cyrb53(min.botId + answer.filename);
        GBServer.globals.files[handle] = answer;
        QrScanner.scanImage(GBServer.globals.files[handle])
          .then(result => console.log(result))
          .catch(error => console.log(error || 'no QR code found.'));
      } else if (kind === 'zipcode') {
        const extractEntity = (text: string) => {
          text = text.replace(/\-/gi, '');

          if (user.locale === 'en') {
            // https://github.com/GeneralBots/BotServer/issues/307
            return text.match(/\d{8}/gi);
          } else {
            return text.match(/(?:\d{1,3}.)*\d{1,3}(?:\,\d+)?/gi);
          }
        };

        const value = extractEntity(answer);

        if (value === null || value.length != 1) {
          await this.talk({ pid, text: 'Por favor, digite um CEP válido.' });
          return await this.hear({ pid, kind, args });
        }

        result = value[0];
      } else if (kind === 'menu') {
        const list = args;
        result = null;
        await CollectionUtil.asyncForEach(list, async item => {
          if (GBConversationalService.kmpSearch(answer, item) != -1) {
            result = item;
          }
        });

        if (result === null) {
          await this.talk({ pid, text: `Escolha por favor um dos itens sugeridos.` });
          return await this.hear({ pid, kind, args });
        }
      } else if (kind === 'language') {
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

        await CollectionUtil.asyncForEach(list, async item => {
          if (
            GBConversationalService.kmpSearch(answer.toLowerCase(), item.name.toLowerCase()) != -1 ||
            GBConversationalService.kmpSearch(answer.toLowerCase(), item.code.toLowerCase()) != -1
          ) {
            result = item.code;
          }
        });

        if (result === null) {
          await this.talk({ pid, text: `Escolha por favor um dos itens sugeridos.` });
          return await this.hear({ pid, kind, args });
        }
      }
      return result;
    } catch (error) {
      GBLog.error(`BASIC RUNTIME ERR HEAR ${error.message ? error.message : error}\n Stack:${error.stack}`);
    }
  }
  static getGBAIPath(botId, packageType = null, packageName = null) {
    let gbai = `${botId}.gbai`;
    if (!packageType && !packageName) {
      return GBConfigService.get('DEV_GBAI') ? GBConfigService.get('DEV_GBAI') : gbai;
    }

    if (GBConfigService.get('DEV_GBAI')) {
      gbai = GBConfigService.get('DEV_GBAI');
      botId = gbai.replace(/\.[^/.]+$/, '');
      return urljoin(GBConfigService.get('DEV_GBAI'), packageName ? packageName : `${botId}.${packageType}`);
    } else {
      return urljoin(gbai, packageName ? packageName : `${botId}.${packageType}`);
    }
  }

  /**
   * Prepares the next dialog to be shown to the specified user.
   */
  public async gotoDialog({ pid, fromOrDialogName, dialogName }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    if (dialogName) {
      if (dialogName.charAt(0) === '/') {
        // https://github.com/GeneralBots/BotServer/issues/308
        // await step.beginDialog(fromOrDialogName);
      } else {
        let sec = new SecService();
        let user = await sec.getUserFromSystemId(fromOrDialogName);
        if (!user) {
          user = await sec.ensureUser(
            min.instance.instanceId,
            fromOrDialogName,
            fromOrDialogName,
            null,
            'whatsapp',
            'from',
            null
          );
        }
        await sec.updateUserHearOnDialog(user.userId, dialogName);
      }
    } else {
      // https://github.com/GeneralBots/BotServer/issues/308
      // await step.beginDialog(fromOrDialogName);
    }
  }

  public static async getProcessInfo(pid: number) {
    const proc = GBServer.globals.processes[pid];

    const min = GBServer.globals.minInstances.filter(p => p.instance.instanceId == proc.instanceId)[0];
    const sec = new SecService();
    const user = await sec.getUserFromId(min.instance.instanceId, proc.userId);
    const params = JSON.parse(user.params);
    return {
      min,
      user,
      params,
      proc
    };
  }

  /**
   * Talks to the user by using the specified text.
   */
  public async talk({ pid, text }) {
    GBLog.info(`BASIC: TALK '${text}'.`);
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    if (user) {
      // TODO: const translate = user ? user.basicOptions.translatorOn : false;

      await min.conversationalService['sendOnConversation'](min, user, text);
    }
    return { status: 0 };
  }

  private static getChannel(): string {
    return 'whatsapp';
    // https://github.com/GeneralBots/BotServer/issues/309
  }

  /**
   * Processes the sending of the file.
   */
  private async internalSendFile({ pid, channel, mobile, filename, caption }) {
    // Handles SEND FILE TO mobile,element in Web Automation.

    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const element = filename._page ? filename._page : filename.screenshot ? filename : null;
    let url;

    if (element) {
      const gbaiName = DialogKeywords.getGBAIPath(min.botId);
      const localName = Path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.jpg`);
      await element.screenshot({ path: localName, fullPage: true });

      url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));

      GBLog.info(`BASIC: WebAutomation: Sending the file ${url} to mobile ${mobile}.`);
    } else if (filename.url) {
      url = filename.url;
    }

    // Handles Markdown.
    else if (filename.indexOf('.md') > -1) {
      GBLog.info(`BASIC: Sending the contents of ${filename} markdown to mobile ${mobile}.`);
      const md = await min.kbService.getAnswerTextByMediaName(min.instance.instanceId, filename);
      if (!md) {
        GBLog.info(`BASIC: Markdown file ${filename} not found on database for ${min.instance.botId}.`);
      }

      await min.conversationalService['playMarkdown'](min, md, DialogKeywords.getChannel(), mobile);
    } else {
      const gbaiName = DialogKeywords.getGBAIPath(min.botId, `gbkb`);

      GBLog.info(`BASIC: Sending the file ${filename} to mobile ${mobile}.`);

      if (!filename.startsWith('https://')) {
        url = urlJoin(GBServer.globals.publicAddress, 'kb', gbaiName, 'assets', filename);
      } else {
        url = filename
      }
    }

    if (url) {
      const reply = { type: ActivityTypes.Message, text: caption };

      const imageData = await (await fetch(url)).arrayBuffer();
      const base64Image = Buffer.from(imageData).toString('base64');
      const contentType = mime.getType(url);  
      const ext = mime.getExtension(contentType);
      reply['attachments'] = [];
      reply['attachments'].push({
        name: filename,
        contentType: ext,
        contentUrl: `data:${contentType};base64,${base64Image}`
      });

      if (channel === 'omnichannel') {
        await min.conversationalService.sendFile(min, null, mobile, url, caption);
      } else {
        await min.conversationalService['sendOnConversation'](min, user, reply);
      }
    }
  }
  /**
   * Generates a new QRCode.
   *
   * file = QRCODE "data"
   *
   */
  public async getQRCode({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const img = await qrcode.toDataURL(text);
    const data = img.replace(/^data:image\/\w+;base64,/, '');
    const buf = Buffer.from(data, 'base64');

    const gbaiName = DialogKeywords.getGBAIPath(min.botId);
    const localName = Path.join('work', gbaiName, 'cache', `qr${GBAdminService.getRndReadableIdentifier()}.png`);
    Fs.writeFileSync(localName, buf, { encoding: null });
    const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));

    return { data: data, localName: localName, url: url };
  }
}
