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

'use strict';

import { GBLog, GBMinInstance } from 'botlib';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { ChartServices } from './ChartServices.js';
import urlJoin from 'url-join';
import { GBServer } from '../../../src/app.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { SecService } from '../../security.gbapp/services/SecService.js';
import { Jimp } from 'jimp';
import jsQR from 'jsqr';
import { SystemKeywords } from './SystemKeywords.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { Messages } from '../strings.js';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService.js';
import fs from 'fs/promises';
import libphonenumber from 'google-libphonenumber';
import * as df from 'date-diff';
import tesseract from 'node-tesseract-ocr';
import path from 'path';
import sgMail from '@sendgrid/mail';
import mammoth from 'mammoth';
import qrcode from 'qrcode';
import { WebAutomationServices } from './WebAutomationServices.js';
import QrScanner from 'qr-scanner';
import pkg from 'whatsapp-web.js';
import { ActivityTypes } from 'botbuilder';
const { List, Buttons } = pkg;
import mime from 'mime-types';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GBUtil } from '../../../src/util.js';
import { GBVMService } from './GBVMService.js';
import { ChatServices } from '../../../packages/llm.gblib/services/ChatServices.js';
import puppeteer from 'puppeteer';


/**
 * Default check interval for user replay
 */
const DEFAULT_HEAR_POLL_INTERVAL = 500;
const POOLING_COUNT = 120;

/**
 * Base services of conversation to be called by BASIC.
 */
export class DialogKeywords {
  public async llmChart({ pid, data, prompt }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    // The prompt for the LLM, including the data.

    const llmPrompt = `
    You are given the following data: ${JSON.stringify(data)}.
    
    Based on this data, generate a configuration for a Billboard.js chart. The output should be valid JSON, following Billboard.js conventions. Ensure the JSON is returned without markdown formatting, explanations, or comments.
    
    The chart should be ${prompt}. Return only the one-line only JSON configuration, nothing else.`;

    // Send the prompt to the LLM and get the response

    const response = await ChatServices.invokeLLM(min, llmPrompt);
    const args = JSON.parse(response.content); // Ensure the LLM generates valid JSON

    // Launch Puppeteer to render the chart

    const browser = await puppeteer.launch();
    const page = await browser.newPage();

    // Load Billboard.js styles and scripts

    await page.addStyleTag({ url: 'https://cdn.jsdelivr.net/npm/billboard.js/dist/theme/datalab.min.css' });
    await page.addScriptTag({ url: 'https://cdn.jsdelivr.net/npm/billboard.js/dist/billboard.pkgd.min.js' });

    // Pass the args to render the chart

    await page.evaluate(`bb.generate(${JSON.stringify(args)});`);

    // Get the chart container and take a screenshot

    const content = await page.$('.bb');
    const gbaiName = GBUtil.getGBAIPath(min.botId);
    const localName = path.join('work', gbaiName, 'cache', `chart${GBAdminService.getRndReadableIdentifier()}.jpg`);
    await content.screenshot({ path: localName, omitBackground: true });
    await browser.close();
    const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));
    GBLogEx.info(min, `Visualization: Chart generated at ${url}.`);

    return { localName, url };
  }
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

    const gbaiName = GBUtil.getGBAIPath(min.botId);
    const localName = path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.jpg`);

    await ChartServices.screenshot(definition, localName);

    const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));

    GBLogEx.info(min, `Visualization: Chart generated at ${url}.`);

    return url;
  }

  /**
   * Returns the OCR of image file.
   *
   */
  public async getOCR({ pid, localFile }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `OCR processing on ${localFile}.`);

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
  public async exit({ }) { }

  /**
   * Get active tasks.
   *
   * @example list = ACTIVE TASKS
   */
  public async getActiveTasks({ pid }) { }

  /**
   * Creates a new deal.
   *
   * @example CREATE DEAL dealname,contato,empresa,amount
   */
  public async createDeal({ pid, dealName, contact, company, amount }) { }

  /**
   * Finds contacts in XRM.
   *
   * @example list = FIND CONTACT "Sandra"
   */
  public async fndContact({ pid, name }) { }

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
    GBLogEx.info(min, `BASIC WEEKDAY contentLocale: ${this.getContentLocaleWithCulture(contentLocale)}`);
    GBLogEx.info(min, `BASIC WEEKDAY date: ${dt}`);
    GBLogEx.info(min, dt.toLocaleString(this.getContentLocaleWithCulture(contentLocale), { weekday: 'short' }));

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
  public async getDateDiff({ pid, date1, date2, mode }) {
    let dt1 = date1;
    let dt2 = date2;
    if (!(dt1 instanceof Date)) {
      dt1 = new Date(dt1);
    }
    if (!(dt2 instanceof Date)) {
      dt2 = new Date(dt2);
    }
    const diff1 = df.default.constructor(date1, date2);
    const diff = Date['diff'](date1, date2);

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

  // https://weblog.west-wind.com/posts/2008/Mar/18/A-simple-formatDate-function-for-JavaScript
  public async format({ pid, value, format }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    if (!(value instanceof Date)) {
      value = SystemKeywords.getDateFromLocaleString(pid, value, contentLocale);
    }
    var date: any = new Date(value); //don't change original date

    if (!format) format = 'MM/dd/yyyy';

    var month = date.getMonth() + 1;
    var year = date.getFullYear();

    format = format.replace('MM', GBUtil.padL(month.toString(), 2, '0'));

    if (format.indexOf('yyyy') > -1) format = format.replace('yyyy', year.toString());
    else if (format.indexOf('yy') > -1) format = format.replace('yy', year.toString().substr(2, 2));

    format = format.replace('dd', GBUtil.padL(date.getDate().toString(), 2, '0'));

    var hours = date.getHours();
    if (format.indexOf('t') > -1) {
      if (hours > 11) format = format.replace('t', 'pm');
      else format = format.replace('t', 'am');
    }
    if (format.indexOf('HH') > -1) format = format.replace('HH', GBUtil.padL(hours.toString(), 2, '0'));
    if (format.indexOf('hh') > -1) {
      if (hours > 12) hours - 12;
      if (hours == 0) hours = 12;
      format = format.replace('hh', hours.toString().padL(2, '0'));
    }
    if (format.indexOf('mm') > -1) format = format.replace('mm', GBUtil.padL(date.getMinutes().toString(), 2, '0'));
    if (format.indexOf('ss') > -1) format = format.replace('ss', GBUtil.padL(date.getSeconds().toString(), 2, '0'));

    return format;
  }

  /**
   * Returns specified date week day in format 'Mon'.
   *
   * @example DATEADD date,"minute",60
   *
   * https://stackoverflow.com/a/1214753/18511
   */
  public async dateAdd({ pid, date, mode, units }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    let dateCopy = date;
    if (!(dateCopy instanceof Date)) {
      dateCopy = SystemKeywords.getDateFromLocaleString(pid, dateCopy, contentLocale);
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
      case 'days':
      case 'd':
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
  public async getToLst({ pid, array, member }) {
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
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    if (!process.env.EMAIL_FROM) {
      return;
    }

    if (!body) {
      body = '';
    }

    // tslint:disable-next-line:no-console

    GBLogEx.info(min, `[E-mail]: to:${to},subject: ${subject},body: ${body}.`);
    const emailToken = process.env.EMAIL_API_KEY;

    // Inline word document used as e-mail body.

    if (typeof body === 'object') {
      const result = await mammoth.convertToHtml({ buffer: body });
      body = result.value;
    }

    if (emailToken) {
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
    } else {
      let { client } = await GBDeployer.internalGetDriveClient(min);

      const data = {
        message: {
          subject: subject,
          body: {
            contentType: 'Text',
            content: body
          },
          toRecipients: [
            {
              emailAddress: {
                address: to
              }
            }
          ],
          from: {
            emailAddress: {
              address: process.env.EMAIL_FROM
            }
          }
        }
      };

      await client.api('/me/sendMail').post(data);

      GBLogEx.info(min, `E-mail ${to} (${subject}) sent.`);
    }
  }

  /**
   * Sends a file to a given mobile.
   *
   * @example SEND FILE TO "+199988887777","image.jpg",caption
   *
   */
  public async sendFileTo({ pid, mobile, filename, caption }) {
    const { min, user, proc } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `SEND FILE TO '${mobile}',filename '${filename}'.`);
    return await this.internalSendFile({ pid, mobile, channel: proc.channel, filename, caption });
  }

  /**
   * Sends a template to a given mobile.
   *
   * @example SEND TEMPLATE TO "+199988887777","image.jpg"
   *
   */
  public async sendTemplateTo({ pid, mobile, filename }) {
    const { min, user, proc } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `SEND TEMPLATE TO '${mobile}',filename '${filename}'.`);
    const service = new GBConversationalService(min.core);

    let text;
    if (filename.endsWith('.docx')) {
      text = await min.kbService.getAnswerTextByMediaName(min.instance.instanceId, filename);
    } else {
      text = filename;
    }

    return await service.fillAndBroadcastTemplate(min, filename, mobile, text);
  }

  /**
   * Sends a file to the current user.
   *
   * @example SEND FILE "image.jpg"
   *
   */
  public async sendFile({ pid, filename, caption }) {
    const { min, user, proc } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `SEND FILE (to: ${user.userSystemId},filename '${filename}'.`);
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
    GBLogEx.info(min, `SET LANGUAGE: ${language}.`);
  }

  /**
   * Defines the current security context for dialogs based on roles.
   *
   * @example ALLOW ROLE "DevOps"
   *
   */
  public async allowRole({ pid, role }) {
    const { min, user, proc } = await DialogKeywords.getProcessInfo(pid);
    const sys = new SystemKeywords();

    if (!role) {
      throw new Error(`Invalid access. NULL role specified.`);
    }

    // Updates current roles allowed from now on this dialog/process.

    proc.roles = role;

    // Checks access.

    const filters = ['People.xlsx', `${role}=x`, `id=${user.userSystemId}`];
    const people = await sys.find({ pid, handle: null, args: filters });

    if (!people) {
      throw new Error(`Invalid access. Check if People sheet has the role ${role} checked.`);
    } else {
      GBLogEx.info(min, `Allowed access for ${user.userSystemId} on ${role}`);
      return people;
    }
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
    GBLogEx.info(min, `${name} = ${value} (botId: ${min.botId})`);
    const sec = new SecService();
    if (user) {
      await sec.setParam(user.userId, name, value);
      return { min, user, params };
    } else {
      min[name] = value;
    }
  }

  public static async getOption({ pid, name, root = false }) {
    if (this.isUserSystemParam(name) && !root) {
      throw new Error(`Not possible to retrieve ${name} system param.`);
    }
    let { min, user, params } = await DialogKeywords.getProcessInfo(pid);

    if (user) {
      const sec = new SecService();
      return await sec.getParam(user, name);
    } else {
      return min[name];
    }
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
   * Define array as output.
   *
   * @example SET OUTPUT ARRAY
   *
   */
  public async setOutput({ pid, value }) {
    await DialogKeywords.setOption({ pid, name: 'output', value: value });
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
   * Returns current if any continuation token for paginated HTTP requests.
   *
   * @example CONTINUATION TOKEN
   *
   */
  public async getContinuationToken({ pid }) {
    let { min, user, params, proc } = await DialogKeywords.getProcessInfo(pid);

    return DialogKeywords.getOption({ pid, name: `${proc.executable}-continuationToken` });
  }

  /**
   * Returns bot param persisted on storage.
   *
   * @example GET CONFIG name
   *
   */
  public async getConfig({ pid, name }) {
    let { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    return min.core.getParam(min.instance, name, null, false);
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
   * Defines page mode for paged GET calls.
   *
   * @example SET PAGE MODE "auto"
   *          data = GET url
   *          FOR EACH item in data
   *              ...
   *          END FOR
   *
   */
  public async setPageMode({ pid, value }) {
    await DialogKeywords.setOption({ pid, name: 'pageMode', value });
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
   *
   * SAVE "file.xlsx", username, now
   *
   */
  public async userName({ pid }) {
    let { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    if (user) return user.userName;
    else return 'unattended';
  }

  /**
   * Returns current mobile number from user in conversation.
   */
  public async userMobile({ pid }) {
    let { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    if (user) return user.userSystemId;
    else return 'unattended';
  }

  /**
   * Shows the subject menu to the user
   *
   * @example MENU
   *
   */
  public async showMenu({ }) {
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

        GBLogEx.info(min, `HEAR with [${args.toString()}] (Asking for input).`);
      } else {
        GBLogEx.info(min, 'HEAR (Asking for input).');
      }

      // Wait for the user to answer.

      min.cbMap[userId] = {};
      min.cbMap[userId]['promise'] = '!GBHEAR';

      while (min.cbMap[userId].promise === '!GBHEAR') {
        await GBUtil.sleep(DEFAULT_HEAR_POLL_INTERVAL);
      }

      const answer = min.cbMap[userId].promise;

      if (!kind) {
        result = answer;
      } else if (kind === 'sheet') {
        // Retrieves the .xlsx file associated with the HEAR var AS file.xlsx.

        let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
        const botId = min.instance.botId;
        const packagePath = GBUtil.getGBAIPath(botId);
        let url = `${baseUrl}/drive/root:/${path}:/children`;

        GBLogEx.info(min, `Loading HEAR AS .xlsx options from Sheet: ${url}`);
        const res = await client.api(url).get();

        // Finds .xlsx specified by arg.

        const document = res.value.filter(m => {
          return m.name === args;
        });
        if (document === undefined || document.length === 0) {
          GBLogEx.info(min, `${args} not found on .gbdata folder, check the package.`);
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
        GBLogEx.info(min, `BASIC (${min.botId}): Upload done for ${answer.filename}.`);
        const handle = WebAutomationServices.cyrb53({ pid, str: min.botId + answer.filename });
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

        result = answer;
      } else if (kind === 'name') {
        const extractEntity = text => {
          return text.match(/[_a-zA-Z][_a-zA-Z0-9]{0,30}/gi);
        };

        const value = extractEntity(answer);

        if (value === null || value.length == 1) {
          await this.talk({ pid, text: 'Por favor, digite um nome válido.' });
          return await this.hear({ pid, kind, args });
        }

        result = answer;
      } else if (kind === 'integer') {
        const extractEntity = text => {
          return text.match(/\d+/gi);
        };

        const value = extractEntity(answer);

        if (value === null || value.length != 1) {
          await this.talk({ pid, text: 'Por favor, digite um número válido.' });
          return await this.hear({ pid, kind, args });
        }

        result = answer;
      } else if (kind === 'date') {
        const parseDate = str => {
          function pad(x) {
            return (('' + x).length == 2 ? '' : '0') + x;
          }
          var m = str.match(/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/),
            d = m ? new Date(m[3], m[2] - 1, m[1]) : null,
            matchesPadded = d && str == [pad(d.getDate()), pad(d.getMonth() + 1), d.getFullYear()].join('/'),
            matchesNonPadded = d && str == [d.getDate(), d.getMonth() + 1, d.getFullYear()].join('/');
          return matchesPadded || matchesNonPadded ? d : null;
        };

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

        result = answer;
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
        let phoneNumber = answer;
        let p = libphonenumber.PhoneNumberUtil.getInstance();
        try {
          // https://github.com/GeneralBots/BotServer/issues/307
          phoneNumber = p.parse(phoneNumber);
        } catch (error) {
          await this.talk({ pid, text: Messages[locale].validation_enter_valid_mobile });

          return await this.hear({ pid, kind, args });
        }
        if (!p.isPossibleNumber(phoneNumber)) {
          await this.talk({ pid, text: 'Por favor, digite um número de telefone válido.' });
          return await this.hear({ pid, kind, args });
        }

        result = phoneNumber;
      } else if (kind === 'qrcode') {
        //https://github.com/GeneralBots/BotServer/issues/171
        GBLogEx.info(min, `BASIC (${min.botId}): QRCode for ${answer.filename}.`);
        const handle = WebAutomationServices.cyrb53({ pid, str: min.botId + answer.filename });
        GBServer.globals.files[handle] = answer;

        // Load the image with Jimp
        const image = await Jimp.read(answer.data);

        // Get the image data
        const imageData = {
          data: new Uint8ClampedArray(image.bitmap.data),
          width: image.bitmap.width,
          height: image.bitmap.height,
        };

        // Use jsQR to decode the QR code
        const decodedQR = jsQR(imageData.data, imageData.width, imageData.height);

        result = decodedQR.data;

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

        result = answer;
      } else if (kind === 'menu') {
        const list = args;
        result = null;
        await CollectionUtil.asyncForEach(list, async item => {
          if (GBConversationalService.kmpSearch(answer, item) != -1) {
            result = item;
          }
        });

        if (result === null) {
          await this.talk({ pid, text: `Escolha por favor um dos itens sugeridos (${args.join(', ')}).` });
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
          user = await sec.ensureUser(min, fromOrDialogName, fromOrDialogName, null, 'whatsapp', 'from', null);
        }
        await sec.updateUserHearOnDialog(user.userId, dialogName);
      }
    } else {
      // https://github.com/GeneralBots/BotServer/issues/308
      // await step.beginDialog(fromOrDialogName);
    }
  }

  public async messageBot({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const { conversation, client } = min['apiConversations'][pid];
    GBLogEx.info(min, `API messaged bot (Conversation Id: ${conversation.conversationId}): ${text} .`);

    await client.apis.Conversations.Conversations_PostActivity({
      conversationId: conversation.conversationId,
      activity: {
        textFormat: 'plain',
        text: text,
        type: 'message',
        pid: pid,
        from: {
          id: user.userSystemId,
          name: user.userName
        }
      }
    });

    min['conversationWelcomed'][conversation.conversationId] = true;
    let messages = [];
    GBLogEx.info(min, `Start API message pooling: ${conversation.conversationId})...`);

    let count = POOLING_COUNT;
    while (count--) {
      await GBUtil.sleep(DEFAULT_HEAR_POLL_INTERVAL);

      try {
        const response = await client.apis.Conversations.Conversations_GetActivities({
          conversationId: conversation.conversationId,
          watermark: conversation.watermark
        });
        conversation.watermark = response.obj.watermark;
        let activities = response.obj.activities;

        if (activities && activities.length) {
          activities = activities.filter(m => m.from.id !== user.userSystemId && m.type === 'message');
          if (activities.length) {
            activities.forEach(activity => {
              messages.push(activity.text);
              GBLogEx.info(min, `MESSAGE BOT answer from bot: ${activity.text}`);
            });
            return messages.join('\n');
          }
        }
      } catch (error) {
        count = 0;
        GBLog.error(`API Message Pooling error: ${GBUtil.toYAML(error)}`);
      }
    }
    return null;
  }

  public async start({ botId, botApiKey, userSystemId, text }) {
    let min: GBMinInstance = GBServer.globals.minInstances.filter(p => p.instance.botId === botId)[0];
    let sec = new SecService();
    let user = await sec.getUserFromSystemId(userSystemId);

    if (!user) {
      user = await sec.ensureUser(min, userSystemId, userSystemId, null, 'api', 'API User', null);
    }

    const pid = GBVMService.createProcessInfo(user, min, 'api', null);
    const conversation = min['apiConversations'][pid];
    const client = await GBUtil.getDirectLineClient(min);
    conversation.client = client;
    const response = await client.apis.Conversations.Conversations_StartConversation();
    conversation.conversationId = response.obj.conversationId;

    return await GBVMService.callVM('start', min, null, pid);
  }

  public static async getProcessInfo(pid: number) {
    const proc = GBServer.globals.processes[pid];
    const step = proc.step;
    const min = GBServer.globals.minInstances.filter(p => p.instance.instanceId == proc.instanceId)[0];
    const sec = new SecService();
    const user = GBServer.globals.users[proc.userId];
    const params = user ? JSON.parse(user.params) : {};
    return {
      min,
      user,
      params,
      proc,
      step
    };
  }

  /**
   * Talks to the user by using the specified text.
   */
  public async talk({ pid, text }) {
    const { min, user, step } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `TALK '${text} step:${step}'.`);

    if (user) {
      // TODO: const translate = user ? user.basicOptions.translatorOn : false;
      text = await min.conversationalService.translate(
        min,
        text,
        user.locale ? user.locale : min.core.getParam(min.instance, 'Locale', GBConfigService.get('LOCALE'))
      );
      GBLog.verbose(`Translated text(playMarkdown): ${text}.`);

      if (step) {
        await min.conversationalService.sendText(min, step, text);
      } else {
        await min.conversationalService['sendOnConversation'](min, user, text);
      }
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
    let nameOnly;
    const gbaiName = GBUtil.getGBAIPath(min.botId);

    if (filename.endsWith('.pdf')) {
      const gbdriveName = GBUtil.getGBAIPath(min.botId, 'gbdrive');
      const pdf = path.join(GBConfigService.get('STORAGE_LIBRARY'), gbdriveName, filename);

      const pngs = await GBUtil.pdfPageAsImage(min, pdf, undefined);

      await CollectionUtil.asyncForEach(pngs, async png => {

        // Prepare a cache to be referenced by Bot Framework.

        url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(png.localName));

        const contentType = mime.lookup(url);
        const reply = { type: ActivityTypes.Message, text: caption };
        reply['attachments'] = [];
        reply['attachments'].push({
          name: nameOnly,
          contentType: contentType,
          contentUrl: url
        });

        if (channel === 'omnichannel' || !user) {
          await min.whatsAppDirectLine.sendFileToDevice(mobile, url, filename, caption);
        } else {
          await min.conversationalService['sendOnConversation'](min, user, reply);
        }
      });          
    }

    // Web automation.

    if (element) {
      const localName = path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.jpg`);
      nameOnly = path.basename(localName);
      await element.screenshot({ path: localName, fullPage: true });

      url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));

      GBLogEx.info(min, `WebAutomation: Sending ${url} to ${mobile} (${channel}).`);
    }

    // GBFILE object.
    else if (filename.url) {
      url = filename.url;
      nameOnly = path.basename(filename.localName);

      GBLogEx.info(min, `Sending the GBFILE ${url} to ${mobile} (${channel}).`);
    }

    // Handles Markdown.
    else if (filename.indexOf('.md') !== -1) {
      GBLogEx.info(min, `Sending the contents of ${filename} markdown to mobile ${mobile}.`);
      const md = await min.kbService.getAnswerTextByMediaName(min.instance.instanceId, filename);
      if (!md) {
        GBLogEx.info(min, `Markdown file ${filename} not found on database for ${min.instance.botId}.`);
      }
      await min.conversationalService['playMarkdown'](min, md, DialogKeywords.getChannel(), null, mobile);

      return;
    }

    // .gbdrive direct sending.
    else {


      if (GBConfigService.get('STORAGE_NAME')) {

        const ext = path.extname(filename);
        const gbaiName = GBUtil.getGBAIPath(min.botId);

        let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
        const fileUrl = urlJoin('/', gbaiName, `${min.botId}.gbdrive`, filename);
        GBLogEx.info(min, `Direct send from .gbdrive: ${fileUrl} to ${mobile}.`);

        const sys = new SystemKeywords();

        const pathOnly = fileUrl.substring(0, fileUrl.lastIndexOf('/'));
        const fileOnly = fileUrl.substring(fileUrl.lastIndexOf('/') + 1);

        let template = await sys.internalGetDocument(client, baseUrl, pathOnly, fileOnly);

        const driveUrl = template['@microsoft.graph.downloadUrl'];
        const res = await fetch(driveUrl);
        let buf: any = Buffer.from(await res.arrayBuffer());
        let localName1 = path.join(
          'work',
          gbaiName,
          'cache',
          `${fileOnly.replace(/\s/gi, '')}-${GBAdminService.getNumberIdentifier()}.${ext}`
        );
        await fs.writeFile(localName1, buf, { encoding: null });

        url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName1));
      }
    }

    if (!url) {
      const ext = path.extname(filename.localName);

      // Prepare a cache to be referenced by Bot Framework.

      const buf = await fs.readFile(filename);
      const gbaiName = GBUtil.getGBAIPath(min.botId);
      const localName = path.join('work', gbaiName, 'cache', `tmp${GBAdminService.getRndReadableIdentifier()}.${ext}`);
      await fs.writeFile(localName, buf, { encoding: null });
      url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));
    }

    const contentType = mime.lookup(url);
    const reply = { type: ActivityTypes.Message, text: caption };
    reply['attachments'] = [];
    reply['attachments'].push({
      name: nameOnly,
      contentType: contentType,
      contentUrl: url
    });

    if (channel === 'omnichannel' || !user) {
      await min.conversationalService.sendFile(min, null, mobile, url, caption);
    } else {
      await min.conversationalService['sendOnConversation'](min, user, reply);
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

    const gbaiName = GBUtil.getGBAIPath(min.botId);
    const localName = path.join('work', gbaiName, 'cache', `qr${GBAdminService.getRndReadableIdentifier()}.png`);
    await fs.writeFile(localName, buf, { encoding: null });
    const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));

    return { data: data, localName: localName, url: url };
  }
}
