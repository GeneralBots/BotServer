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
import { GBLog, GBMinInstance } from 'botlib';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { DialogKeywords } from './DialogKeywords.js';
import { GBServer } from '../../../src/app.js';
import { GBVMService } from './GBVMService.js';
import Fs from 'fs';
import { GBSSR }from '../../core.gbapp/services/GBSSR.js';
import urlJoin from 'url-join';
import Excel from 'exceljs';
import { TwitterApi } from 'twitter-api-v2';
import Path from 'path';
import ComputerVisionClient from '@azure/cognitiveservices-computervision';
import ApiKeyCredentials from '@azure/ms-rest-js';
import alasql from 'alasql';
import PizZip from 'pizzip';
import Docxtemplater from 'docxtemplater';
import pptxTemplaterModule from 'pptxtemplater';
import _ from 'lodash';
import { DocxImager } from 'docximager';
import { pdfToPng, PngPageOutput } from 'pdf-to-png-converter';
import sharp from 'sharp';
import apply from 'async/apply';
import ImageModule from 'open-docxtemplater-image-module';


/**
 * @fileoverview General Bots server core.
 */

/**
 * BASIC system class for extra manipulation of bot behaviour.
 */
export class SystemKeywords {
  /**
   * Reference to minimal bot instance.
   */
  public min: GBMinInstance;

  /**
   * Reference to the deployer service.
   */
  private readonly deployer: GBDeployer;

  dk: DialogKeywords;
  wa;

  /**
   * When creating this keyword facade, a bot instance is
   * specified among the deployer service.
   */
  constructor(min: GBMinInstance, deployer: GBDeployer, dk: DialogKeywords, wa) {
    this.min = min;
    this.wa = wa;
    this.deployer = deployer;
    this.dk = dk;
  }

  public async callVM({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const step = null;
    const deployer = null;

    return await GBVMService.callVM(text, min, step, user, deployer, false);
  }

  public async append({ pid, args }) {
    let array = [].concat(...args);
    return array.filter(function (item, pos) {
      return item;
    });
  }

  /**
   *
   * @example SEE CAPTION OF url AS variable
   *
   */
  public async seeCaption({ pid, url }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const computerVisionClient = new ComputerVisionClient.ComputerVisionClient(
      new ApiKeyCredentials.ApiKeyCredentials({ inHeader: { 'Ocp-Apim-Subscription-Key': process.env.VISION_KEY } }),
      process.env.VISION_ENDPOINT
    );

    let caption = (await computerVisionClient.describeImage(url)).captions[0];

    const contentLocale = this.min.core.getParam<string>(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );
    GBLog.info(`GBVision (caption): '${caption.text}' (Confidence: ${caption.confidence.toFixed(2)})`);

    return await min.conversationalService.translate(min, caption.text, contentLocale);
  }

  /**
   *
   * @example SEE TEXT OF url AS variable
   *
   */
  public async seeText({ pid, url }) {
    const computerVisionClient = new ComputerVisionClient.ComputerVisionClient(
      new ApiKeyCredentials.ApiKeyCredentials({ inHeader: { 'Ocp-Apim-Subscription-Key': process.env.VISION_KEY } }),
      process.env.VISION_ENDPOINT
    );

    const result = await computerVisionClient.recognizePrintedText(true, url);
    const text = result.regions[0].lines[0].words[0].text;
    let final = '';

    for (let i = 0; i < result.regions.length; i++) {
      const region = result.regions[i];

      for (let j = 0; j < region.lines.length; j++) {
        const line = region.lines[j];

        for (let k = 0; k < line.words.length; k++) {
          final += `${line.words[k].text} `;
        }
      }
    }

    GBLog.info(`GBVision (text): '${final}'`);
    return final;
  }

  public async sortBy({ pid, array, memberName }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    memberName = memberName.trim();
    const contentLocale = this.min.core.getParam<string>(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    // Detects data type from the first element of array.

    let dt = array[0] ? array[0][memberName] : null;
    let date = SystemKeywords.getDateFromLocaleString(pid, dt, contentLocale);
    if (date) {
      return array
        ? array.sort((a, b) => {
            const c = new Date(a[memberName]);
            const d = new Date(b[memberName]);
            return c.getTime() - d.getTime();
          })
        : null;
    } else {
      return array
        ? array.sort((a, b) => {
            if (a[memberName] < b[memberName]) {
              return -1;
            }
            if (a[memberName] > b[memberName]) {
              return 1;
            }
            return 0;
          })
        : array;
    }
  }

  public static JSONAsGBTable(data, headers) {
    try {
      let output = [];
      let isObject = false;

      if (Array.isArray(data)) {
        isObject = Object.keys(data[1]) !== null;
      }

      if (isObject || JSON.parse(data) !== null) {
        let keys = Object.keys(data[1]);

        if (headers) {
          output[0] = [];
          // Copies headers as the first element.

          for (let i = 0; i < keys.length; i++) {
            output[0][i] = keys[i];
          }
        } else {
          output.push({ gbarray: '0' });
        }

        // Copies data from JSON format into simple array.

        for (let i = 0; i < data.length; i++) {
          output[i + 1] = [];
          for (let j = 0; j < keys.length; j++) {
            output[i + 1][j] = data[i][keys[j]];
          }
        }

        return output;
      }
    } catch (error) {
      GBLog.error(error);
      return data;
    }
  }

  /**
   *
   * @param data
   * @param renderPDF
   * @param renderImage
   * @returns
   *
   * @see http://tabulator.info/examples/5.2
   */
  private async renderTable(pid, data, renderPDF, renderImage) {
    if (!data[1]) {
      return null;
    }

    data = SystemKeywords.JSONAsGBTable(data, true);

    // Detects if it is a collection with repeated
    // headers.

    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const gbaiName = `${min.botId}.gbai`;
    const browser = await GBSSR.createBrowser(null);
    const page = await browser.newPage();

    // Includes the associated CSS related to current theme.

    const theme = this.dk.user.basicOptions.theme;
    switch (theme) {
      case 'white':
        await page.addStyleTag({ path: 'node_modules/tabulator-tables/dist/css/tabulator_simple.min.css' });
        break;
      case 'dark':
        await page.addStyleTag({ path: 'node_modules/tabulator-tables/dist/css/tabulator_midnight.min.css' });
        break;
      case 'blue':
        await page.addStyleTag({ path: 'node_modules/tabulator-tables/dist/css/tabulator_modern.min.css' });
        break;
      default:
        break;
    }

    await page.addScriptTag({ path: 'node_modules/tabulator-tables/dist/js/tabulator.min.js' });

    // Removes internal hidden element used to hold one-based index arrays.

    data.shift();

    // Guess fields from data variable into Tabulator fields collection.

    let fields = [];
    let keys = Object.keys(data[0]);
    for (let i = 0; i < keys.length; i++) {
      fields.push({ field: keys[i], title: keys[i] });
    }

    // Adds DIV for Tabulator.

    await page.evaluate(() => {
      const el = document.createElement('div');
      el.setAttribute('id', 'table');
      document.body.appendChild(el);
    });

    const code = `
        var table = new Tabulator("#table", {
        height:"311px",
        layout:"fitColumns",
        data: ${JSON.stringify(data)},
        columns: ${JSON.stringify(fields)}
    });
    `;
    await page.evaluate(code);
    await page.waitForSelector('#table');

    // Handles image generation.

    let url;
    let localName;
    if (renderImage) {
      localName = Path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.png`);
      await page.screenshot({ path: localName, fullPage: true });
      url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));
      GBLog.info(`BASIC: Table image generated at ${url} .`);
    }

    // Handles PDF generation.

    if (renderPDF) {
      localName = Path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.pdf`);
      url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));
      let pdf = await page.pdf({ format: 'A4' });
      GBLog.info(`BASIC: Table PDF generated at ${url} .`);
    }

    await browser.close();
    return [url, localName];
  }

  public async asPDF({ pid, data }) {
    let file = await this.renderTable(pid, data, true, false);
    return file[0];
  }

  public async asImage({ pid, data }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    // Checks if it is a GBFILE.

    if (data.data) {  
      const gbfile = data.data;

      let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
      const botId = this.min.instance.botId;
      const gbaiName = `${this.min.botId}.gbai`;
      const tmpDocx = urlJoin(gbaiName, `${botId}.gbdrive`, `tmp${GBAdminService.getRndReadableIdentifier()}.docx`);

      // Performs the conversion operation.

      await client.api(`${baseUrl}/drive/root:/${tmpDocx}:/content`).put(data.data);
      const res = await client.api(`${baseUrl}/drive/root:/${tmpDocx}:/content?format=pdf`).get();
      await client.api(`${baseUrl}/drive/root:/${tmpDocx}:/content`).delete();

      const streamToBuffer = stream => {
        const chunks = [];
        return new Promise((resolve, reject) => {
          stream.on('data', chunk => chunks.push(chunk));
          stream.on('error', reject);
          stream.on('end', () => resolve(Buffer.concat(chunks)));
        });
      };

      gbfile.data = await streamToBuffer(res);

      // Converts the PDF to PNG.

      const pngPages: PngPageOutput[] = await pdfToPng(gbfile.data, {
        disableFontFace: false,
        useSystemFonts: false,
        viewportScale: 2.0,
        pagesToProcess: [1],
        strictPagesToProcess: false,
        verbosityLevel: 0
      });

      // Prepare an image on cache and return the GBFILE information.

      const localName = Path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.png`);
      if (pngPages.length > 0) {
        const buffer = pngPages[0].content;
        const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));

        Fs.writeFileSync(localName, buffer, { encoding: null });

        return { localName: localName, url: url, data: buffer };
      }
    } else {
      let file = await this.renderTable(pid, data, false, true);
      return file[0];
    }
  }

  public async executeSQL({ pid, data, sql, tableName }) {
    let objectMode = false;
    if (Object.keys(data[0])) {
      objectMode = true;
    }

    let first;
    if (!objectMode) {
      first = data.shift();
    }
    data = alasql(sql, [data]);
    if (!objectMode) {
      data.unshift(first);
    }
    return data;
  }

  /**
   * Retrives the content of a given URL.
   */
  public async getFileContents({ pid, url, headers }) {
    const options = {
      method: 'GET',
      encoding: 'binary',
      headers: headers
    };
    return await fetch(url, options);
  }

  /**
   * Retrives a random id with a length of five, every time it is called.
   */
  public getRandomId() {
    const idGeneration = this.dk['idGeneration'];
    if (idGeneration && idGeneration.trim().toLowerCase() === 'number') {
      return GBAdminService.getNumberIdentifier();
    } else {
      return GBAdminService.getRndReadableIdentifier().substr(5);
    }
  }

  /**
   * Retrives stock inforation for a given symbol.
   */
  public async getStock({ pid, symbol }) {
    const url = `http://live-nse.herokuapp.com/?symbol=${symbol}`;
    let data = await fetch(url);
    return data;
  }

  /**
   * Holds script execution for the number of seconds specified.
   *
   * @example WAIT 5 ' This will wait five seconds.
   *
   */
  public async wait({ pid, seconds }) {
    // tslint:disable-next-line no-string-based-set-timeout
    GBLog.info(`BASIC: WAIT for ${seconds} second(s).`);
    const timeout = async (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
    await timeout(seconds * 1000);
  }

  /**
   * Sends a text message to the mobile number specified.
   *
   * @example TALK TO "+199988887777", "Message text here"
   *
   */
  public async talkTo({ pid, mobile, message }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLog.info(`BASIC: Talking '${message}' to a specific user (${mobile}) (TALK TO). `);
    await min.conversationalService.sendMarkdownToMobile(min, null, mobile, message);
  }

  /**
   * Sends a SMS message to the mobile number specified.
   *
   * @example SEND SMS TO "+199988887777", "Message text here"
   *
   */
  public async sendSmsTo({ pid, mobile, message }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLog.info(`BASIC: SEND SMS TO '${mobile}', message '${message}'.`);
    await min.conversationalService.sendSms(min, mobile, message);
  }

  /**
   * 1. Defines a cell value in the tabular file.
   * 2. Defines an element text on HTML page.
   *
   * @example SET "file.xlsx", "A2", 4500
   *
   * @example SET page, "elementHTMLSelector", "text"
   *
   */
  public async set({ pid, file, address, value }): Promise<any> {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    // Handles calls for HTML stuff

    if (file._javascriptEnabled) {
      const page = file;
      GBLog.info(`BASIC: Web automation setting ${page}' to '${value}' (SET). `);
      await this.wa.setElementText({ page, selector: address, text: value });

      return;
    }

    // Handles calls for BASIC persistence on sheet files.

    GBLog.info(`BASIC: Defining '${address}' in '${file}' to '${value}' (SET). `);

    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

    const botId = min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbdata`;

    address = address.indexOf(':') !== -1 ? address : address + ':' + address;

    let document = await this.internalGetDocument(client, baseUrl, path, file);

    let body = { values: [[]] };
    body.values[0][0] = value;

    let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();

    await client
      .api(
        `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${address}')`
      )
      .patch(body);
  }

  /**
   * Retrives a document from the drive, given a path and filename.
   */
  private async internalGetDocument(client: any, baseUrl: any, path: string, file: string) {
    let res = await client.api(`${baseUrl}/drive/root:${path}:/children`).get();

    let documents = res.value.filter(m => {
      return m.name.toLowerCase() === file.toLowerCase();
    });

    if (!documents || documents.length === 0) {
      throw `File '${file}' specified on GBasic command not found. Check the .gbdata or the .gbdialog associated.`;
    }

    return documents[0];
  }

  /**
   * Saves the content of variable into the file in .gbdata default folder.
   *
   * @exaple SAVE variable as "my.txt"
   *
   */
  public async saveFile({ pid, file, data }): Promise<any> {
    GBLog.info(`BASIC: Saving '${file}' (SAVE file).`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
    const botId = this.min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbdrive`;

    // Checks if it is a GB FILE object.

    if (data.data && data.filename) {
      data = data.data;
    }

    try {
      data = GBServer.globals.files[data].data;
      await client.api(`${baseUrl}/drive/root:/${path}/${file}:/content`).put(data);
    } catch (error) {
      if (error.code === 'itemNotFound') {
        GBLog.info(`BASIC: BASIC source file not found: ${file}.`);
      } else if (error.code === 'nameAlreadyExists') {
        GBLog.info(`BASIC: BASIC destination file already exists: ${file}.`);
      }
      throw error;
    }
  }

  /**
   * Saves the content of several variables to a new row in a tabular file.
   *
   * @exaple SAVE "customers.xlsx", name, email, phone, address, city, state, country
   *
   */
  public async save({ pid, args }): Promise<any> {
    const file = args[0];
    args.shift();
    GBLog.info(`BASIC: Saving '${file}' (SAVE). Args: ${args.join(',')}.`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
    const botId = this.min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbdata`;

    let document = await this.internalGetDocument(client, baseUrl, path, file);
    let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();

    await client
      .api(
        `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A2:DX2')/insert`
      )
      .post({});

    if (args.length > 128) {
      throw `File '${file}' has a SAVE call with more than 128 arguments. Check the .gbdialog associated.`;
    }

    let body = { values: [[]] };

    const address = `A2:${this.numberToLetters(args.length - 1)}2`;
    for (let index = 0; index < args.length; index++) {
      let value = args[index];
      if (value && this.isValidDate(value)) {
        value = `'${value}`;
      }
      body.values[0][index] = value;
    }

    await client
      .api(
        `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${address}')`
      )
      .patch(body);
  }

  /**
   * Retrives the content of a cell in a tabular file.
   *
   * @example value = GET "file.xlsx", "A2"
   *
   */
  public async get({ pid, file, addressOrHeaders, httpUsername, httpPs, qs, streaming }): Promise<any> {
    if (file.startsWith('http')) {
      return await this.getByHttp({
        pid,
        url: file,
        headers: addressOrHeaders,
        username: httpUsername,
        ps: httpPs,
        qs
      });
    } else {
      GBLog.info(`BASIC: GET '${addressOrHeaders}' in '${file}'.`);
      let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
      const botId = this.min.instance.botId;
      const path = `/${botId}.gbai/${botId}.gbdata`;

      let document = await this.internalGetDocument(client, baseUrl, path, file);

      // Creates workbook session that will be discarded.

      let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();

      let results = await client
        .api(
          `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${addressOrHeaders}')`
        )
        .get();

      let val = results.text[0][0];
      GBLog.info(`BASIC: Getting '${file}' (GET). Value= ${val}.`);
      return val;
    }
  }

  public isValidDate({ pid, dt }) {
    const contentLocale = this.min.core.getParam<string>(
      this.min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    let date = SystemKeywords.getDateFromLocaleString(pid, dt, contentLocale);
    if (!date) {
      return false;
    }

    if (!(date instanceof Date)) {
      date = new Date(date);
    }

    return !isNaN(date.valueOf());
  }

  public isValidNumber({ pid, number }) {
    if (number === '') {
      return false;
    }
    return !isNaN(number);
  }

  public isValidHour({ pid, value }) {
    return /^([01]?[0-9]|2[0-3]):[0-5][0-9]$/.test(value);
  }

  /**
   * Finds a value or multi-value results in a tabular file.
   *
   * @example
   *
   *  rows = FIND "file.xlsx", "A2=active", "A2 < 12/06/2010 15:00"
   *  i = 1
   *  do while i < ubound(row)
   *    row = rows[i]
   *    send sms to "+" + row.mobile, "Hello " + row.name + "! "
   *  loop
   * @see NPM package data-forge
   *
   */
  public async find({ pid, args }): Promise<any> {
    const file = args[0];
    args.shift();

    const botId = this.min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbdata`;

    // MAX LINES property.

    let maxLines;
    if (this.dk.user && this.dk.user.basicOptions && this.dk.user.basicOptions.maxLines) {
      if (this.dk.user.basicOptions.maxLines.toString().toLowerCase() !== 'default') {
        maxLines = Number.parseInt(this.dk.user.basicOptions.maxLines).valueOf();
      }
    } else {
      maxLines = this.dk.maxLines;
    }
    GBLog.info(`BASIC: FIND running on ${file} (maxLines: ${maxLines}) and args: ${JSON.stringify(args)}...`);

    // Choose data sources based on file type (HTML Table, data variable or sheet file)

    let results;
    let header, rows;

    if (file['$eval']) {
      const container = file['frame'] ? file['frame'] : file['_page'];
      const originalSelector = file['originalSelector'];

      // Transforms table

      const resultH = await container.evaluate(originalSelector => {
        const rows = document.querySelectorAll(`${originalSelector} tr`);
        return Array.from(rows, row => {
          const columns = row.querySelectorAll('th');
          return Array.from(columns, column => column.innerText);
        });
      }, originalSelector);

      const result = await container.evaluate(originalSelector => {
        const rows = document.querySelectorAll(`${originalSelector} tr`);
        return Array.from(rows, row => {
          const columns = row.querySelectorAll('td');
          return Array.from(columns, column => column.innerText);
        });
      }, originalSelector);

      header = [];
      for (let i = 0; i < resultH[0].length; i++) {
        header[i] = resultH[0][i];
      }

      rows = [];
      rows[0] = header;
      for (let i = 1; i < result.length; i++) {
        rows[i] = result[i];
      }
    } else if (file['cTag']) {
      const gbaiName = `${this.min.botId}.gbai`;
      const localName = Path.join('work', gbaiName, 'cache', `csv${GBAdminService.getRndReadableIdentifier()}.csv`);
      const url = file['@microsoft.graph.downloadUrl'];
      const response = await fetch(url);
      Fs.writeFileSync(localName, Buffer.from(await response.arrayBuffer()), { encoding: null });

      var workbook = new Excel.Workbook();
      const worksheet = await workbook.csv.readFile(localName);
      header = [];
      rows = [];

      for (let i = 0; i < worksheet.rowCount; i++) {
        const r = worksheet.getRow(i + 1);
        let outRow = [];
        for (let j = 0; j < r.cellCount; j++) {
          outRow.push(r.getCell(j + 1).text);
        }

        if (i == 0) {
          header = outRow;
        } else {
          rows.push(outRow);
        }
      }
    } else {
      let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);

      let document;
      document = await this.internalGetDocument(client, baseUrl, path, file);

      // Creates workbook session that will be discarded.

      let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();

      results = await client
        .api(
          `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A1:CZ${maxLines}')`
        )
        .get();

      header = results.text[0];
      rows = results.text;
    }

    let getFilter = async text => {
      let filter;
      const operators = [/\<\=/, /\>\=/, /\</, /\>/, /\bnot in\b/, /\bin\b/, /\=/];
      let done = false;
      await CollectionUtil.asyncForEach(operators, async op => {
        var re = new RegExp(op, 'gi');
        const parts = text.split(re);

        if (parts.length === 2 && !done) {
          filter = {
            columnName: parts[0].trim(),
            operator: op.toString().replace(/\\b/g, '').replace(/\//g, '').replace(/\\/g, '').replace(/\b/g, ''),
            value: parts[1].trim()
          };

          // Swaps values and names in case of IN operators.

          if (filter.operator === 'not in' || filter.operator === 'in') {
            const columnName = filter.columnName;
            filter.columnName = filter.value;
            filter.value = columnName;
          }

          done = true;
        }
      });

      return filter;
    };

    const contentLocale = this.min.core.getParam<string>(
      this.min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    // Increments columnIndex by looping until find a column match.

    const filters = [];

    await CollectionUtil.asyncForEach(args, async arg => {
      const filter = await getFilter(arg);
      if (!filter) {
        throw new Error(`BASIC: FIND filter has an error: ${arg} check this and publish .gbdialog again.`);
      }

      let columnIndex = 0;
      for (; columnIndex < header.length; columnIndex++) {
        if (header[columnIndex].toLowerCase() === filter.columnName.toLowerCase()) {
          break;
        }
      }
      filter.columnIndex = columnIndex;

      if (this.isValidHour(filter.value)) {
        filter.dataType = 'hourInterval';
      } else if (this.isValidDate(filter.value)) {
        filter.value = SystemKeywords.getDateFromLocaleString(pid, filter.value, contentLocale);
        filter.dataType = 'date';
      } else if (this.isValidNumber(filter.value)) {
        filter.value = Number.parseInt(filter.value);
        filter.dataType = 'number';
      } else {
        filter.value = filter.value;
        filter.dataType = 'string';
      }
      filters.push(filter);
    });

    // As BASIC uses arrays starting with 1 (one) as index,
    // a ghost element is added at 0 (zero) position.

    let table = [];
    table.push({ gbarray: '0' });
    let foundIndex = 1;

    // Fills the row variable.

    let rowCount = 0;
    for (; foundIndex < rows.length; foundIndex++) {
      let filterAcceptCount = 0;
      await CollectionUtil.asyncForEach(filters, async filter => {
        let result = rows[foundIndex][filter.columnIndex];
        let wholeWord = true;
        if (this.dk.user && this.dk.user.basicOptions && this.dk.user.basicOptions.wholeWord) {
          wholeWord = this.dk.user.basicOptions.wholeWord;
        }

        switch (filter.dataType) {
          case 'string':
            switch (filter.operator) {
              case '=':
                if (wholeWord) {
                  if (result && result.toLowerCase().trim() === filter.value.toLowerCase().trim()) {
                    filterAcceptCount++;
                  }
                } else {
                  if (result && result.toLowerCase().trim().indexOf(filter.value.toLowerCase().trim()) > -1) {
                    filterAcceptCount++;
                  }
                }
                break;
              case 'not in':
                if (wholeWord) {
                  if (result && result.toLowerCase().trim() !== filter.value.toLowerCase().trim()) {
                    filterAcceptCount++;
                  }
                } else {
                  if (result && result.toLowerCase().trim().indexOf(filter.value.toLowerCase().trim()) === -1) {
                    filterAcceptCount++;
                  }
                }
                break;
              case 'in':
                if (wholeWord) {
                  if (result && result.toLowerCase().trim() === filter.value.toLowerCase().trim()) {
                    filterAcceptCount++;
                  }
                } else {
                  if (result && result.toLowerCase().trim().indexOf(filter.value.toLowerCase().trim()) > -1) {
                    filterAcceptCount++;
                  }
                }
                break;
            }
            break;
          case 'number':
            switch (filter.operator) {
              case '=':
                if (Number.parseInt(result) === filter.value) {
                  filterAcceptCount++;
                }
                break;
            }
            break;

          case 'hourInterval':
            switch (filter.operator) {
              case '=':
                if (result && result.toLowerCase().trim() === filter.value.toLowerCase().trim()) {
                  filterAcceptCount++;
                }
                break;
              case 'in':
                const e = result.split(';');
                const hr = Number.parseInt(filter.value.split(':')[0]);
                let lastHour = Number.parseInt(e[0]);
                let found = false;
                await CollectionUtil.asyncForEach(e, async hour => {
                  if (!found && lastHour <= hr && hr <= hour) {
                    filterAcceptCount++;
                    found = true;
                  }
                  lastHour = hour;
                });
                break;
            }
            break;

          case 'date':
            if (result.charAt(0) === "'") {
              result = result.substr(1);
            }
            const resultDate = SystemKeywords.getDateFromLocaleString(pid, result, contentLocale);
            if (filter.value['dateOnly']) {
              resultDate.setHours(0, 0, 0, 0);
            }
            if (resultDate) {
              switch (filter.operator) {
                case '=':
                  if (resultDate.getTime() == filter.value.getTime()) filterAcceptCount++;
                  break;
                case '<':
                  if (resultDate.getTime() < filter.value.getTime()) filterAcceptCount++;
                  break;
                case '>':
                  if (resultDate.getTime() > filter.value.getTime()) filterAcceptCount++;
                  break;
                case '<=':
                  if (resultDate.getTime() <= filter.value.getTime()) filterAcceptCount++;
                  break;
                case '>=':
                  if (resultDate.getTime() >= filter.value.getTime()) filterAcceptCount++;
                  break;
              }
              break;
            }
        }
      });

      if (filterAcceptCount === filters.length) {
        rowCount++;
        let row = {};
        const xlRow = rows[foundIndex];
        for (let colIndex = 0; colIndex < xlRow.length; colIndex++) {
          const propertyName = header[colIndex];
          let value = xlRow[colIndex];
          if (value && value.charAt(0) === "'") {
            if (this.isValidDate(value.substr(1))) {
              value = value.substr(1);
            }
          }
          row[propertyName] = value;
        }
        row['ordinal'] = rowCount;
        row['line'] = foundIndex + 1;
        table.push(row);
      }
    }

    if (table.length === 1) {
      GBLog.info(`BASIC: FIND returned no results (zero rows).`);
      return null;
    } else if (table.length === 2) {
      GBLog.info(`BASIC: FIND returned single result: ${table[0]}.`);
      return table[1];
    } else {
      GBLog.info(`BASIC: FIND returned multiple results (Count): ${table.length - 1}.`);
      return table;
    }
  }

  public static getDateFromLocaleString(pid, date: any, contentLocale: any) {
    let ret = null;
    let parts = /^([0-3]?[0-9]).([0-3]?[0-9]).((?:[0-9]{2})?[0-9]{2})\s*(10|11|12|0?[1-9]):([0-5][0-9])/gi.exec(date);
    if (parts && parts[5]) {
      switch (contentLocale) {
        case 'pt':
          ret = new Date(
            Number.parseInt(parts[3]),
            Number.parseInt(parts[2]) - 1,
            Number.parseInt(parts[1]),
            Number.parseInt(parts[4]),
            Number.parseInt(parts[5]),
            0,
            0
          );
          break;
        case 'en':
          ret = new Date(
            Number.parseInt(parts[3]),
            Number.parseInt(parts[1]) - 1,
            Number.parseInt(parts[2]),
            Number.parseInt(parts[4]),
            Number.parseInt(parts[5]),
            0,
            0
          );
          break;
      }

      ret['dateOnly'] = false;
    }

    parts = /^([0-3]?[0-9]).([0-3]?[0-9]).((?:[0-9]{2})?[0-9]{2})$/gi.exec(date);
    if (parts && parts[3]) {
      switch (contentLocale) {
        case 'pt':
          ret = new Date(
            Number.parseInt(parts[3]),
            Number.parseInt(parts[2]) - 1,
            Number.parseInt(parts[1]),
            0,
            0,
            0,
            0
          );
          break;
        case 'en':
          ret = new Date(
            Number.parseInt(parts[3]),
            Number.parseInt(parts[1]) - 1,
            Number.parseInt(parts[2]),
            0,
            0,
            0,
            0
          );
          break;
      }

      ret['dateOnly'] = true;
    }
    return ret;
  }

  /**
   * Creates a folder in the bot instance drive.
   *
   * @example folder = CREATE FOLDER "notes\01"
   *
   */
  public async createFolder({ pid, name }) {
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
    const botId = this.min.instance.botId;
    let path = `/${botId}.gbai/${botId}.gbdrive`;

    // Extracts each part of path to call create folder to each
    // one of them.

    name = name.replace(/\\/gi, '/');
    const parts = name.split('/');
    let lastFolder = null;

    // Creates each subfolder.

    await CollectionUtil.asyncForEach(parts, async item => {
      // Calls drive API.

      const body = {
        name: item,
        folder: {},
        '@microsoft.graph.conflictBehavior': 'fail'
      };

      try {
        lastFolder = await client.api(`${baseUrl}/drive/root:/${path}:/children`).post(body);
      } catch (error) {
        if (error.code !== 'nameAlreadyExists') {
          throw error;
        } else {
          lastFolder = await client.api(`${baseUrl}/drive/root:/${urlJoin(path, item)}`).get();
        }
      }

      // Increments path to the next child be created.

      path = urlJoin(path, item);
    });
    return lastFolder;
  }

  /**
   * Shares a folder from the drive to a e-mail recipient.
   *
   * @example
   *
   * folder = CREATE FOLDER "notes\10"
   * SHARE FOLDER folder, "nome@domain.com", "E-mail message"
   *
   */
  public async shareFolder({ pid, folder, email, message }) {
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
    const root = urlJoin(`/${this.min.botId}.gbai/${this.min.botId}.gbdrive`, folder);

    const src = await client.api(`${baseUrl}/drive/root:/${root}`).get();

    const driveId = src.parentReference.driveId;
    const itemId = src.id;
    const body = {
      recipients: [{ email: email }],
      message: message,
      requireSignIn: true,
      sendInvitation: true,
      roles: ['write']
    };

    await client.api(`https://graph.microsoft.com/v1.0/drives/${driveId}/items/${itemId}/invite`).post(body);
  }

  /**
   * Copies a drive file from a place to another .
   *
   * @example
   *
   * COPY "template.xlsx", "reports\" + customerName + "\final.xlsx"
   *
   */
  public async copyFile({ pid, src, dest }) {
    GBLog.info(`BASIC: BEGINING COPY '${src}' to '${dest}'`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
    const botId = this.min.instance.botId;

    // Normalizes all slashes.

    src = src.replace(/\\/gi, '/');
    dest = dest.replace(/\\/gi, '/');

    // Determines full path at source and destination.

    const root = urlJoin(`/${botId}.gbai/${botId}.gbdata`);
    const srcPath = urlJoin(root, src);
    const dstPath = urlJoin(`/${botId}.gbai/${botId}.gbdata`, dest);

    // Checks if the destination contains subfolders that
    // need to be created.

    let folder;
    if (dest.indexOf('/') !== -1) {
      const pathOnly = Path.dirname(dest);
      folder = await this.createFolder({ pid, name: pathOnly });
    } else {
      folder = await client.api(`${baseUrl}/drive/root:/${root}`).get();
    }

    // Performs the copy operation getting a reference
    // to the source and calling /copy on drive API.

    try {
      const srcFile = await client.api(`${baseUrl}/drive/root:/${srcPath}`).get();
      const destFile = {
        parentReference: { driveId: folder.parentReference.driveId, id: folder.id },
        name: `${Path.basename(dest)}`
      };

      return await client.api(`${baseUrl}/drive/items/${srcFile.id}/copy`).post(destFile);
    } catch (error) {
      if (error.code === 'itemNotFound') {
        GBLog.info(`BASIC: COPY source file not found: ${srcPath}.`);
      } else if (error.code === 'nameAlreadyExists') {
        GBLog.info(`BASIC: COPY destination file already exists: ${dstPath}.`);
      }
      throw error;
    }
    GBLog.info(`BASIC: FINISHED COPY '${src}' to '${dest}'`);
  }

  /**
   * Converts a drive file from a place to another .
   *
   * Supported sources csv, doc, docx, odp, ods, odt, pot, potm, potx, pps,
   * ppsx, ppsxm, ppt, pptm, pptx, rtf, xls, xlsx
   *
   * @example
   *
   * CONVERT "customers.xlsx" TO "reports\" + today + ".pdf"
   *
   */
  public async convert({ pid, src, dest }) {
    GBLog.info(`BASIC: CONVERT '${src}' to '${dest}'`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
    const botId = this.min.instance.botId;

    // Normalizes all slashes.

    src = src.replace(/\\/gi, '/');
    dest = dest.replace(/\\/gi, '/');

    // Determines full path at source and destination.

    const root = urlJoin(`/${botId}.gbai/${botId}.gbdrive`);
    const srcPath = urlJoin(root, src);
    const dstPath = urlJoin(`/${botId}.gbai/${botId}.gbdrive`, dest);

    // Checks if the destination contains subfolders that
    // need to be created.

    let folder;
    if (dest.indexOf('/') !== -1) {
      const pathOnly = Path.dirname(dest);
      folder = await this.createFolder({ pid, name: pathOnly });
    } else {
      folder = await client.api(`${baseUrl}/drive/root:/${root}`).get();
    }

    // Performs the conversion operation getting a reference
    // to the source and calling /content on drive API.

    try {
      const res = await client.api(`${baseUrl}/drive/root:/${srcPath}:/content?format=pdf`).get();

      const streamToBuffer = stream => {
        const chunks = [];
        return new Promise((resolve, reject) => {
          stream.on('data', chunk => chunks.push(chunk));
          stream.on('error', reject);
          stream.on('end', () => resolve(Buffer.concat(chunks)));
        });
      };

      const result = await streamToBuffer(res);

      await client.api(`${baseUrl}/drive/root:/${dstPath}:/content`).put(result);
    } catch (error) {
      if (error.code === 'itemNotFound') {
        GBLog.info(`BASIC: CONVERT source file not found: ${srcPath}.`);
      } else if (error.code === 'nameAlreadyExists') {
        GBLog.info(`BASIC: CONVERT destination file already exists: ${dstPath}.`);
      }
      throw error;
    }
  }

  /**
   * Generate a secure and unique password.
   *
   * @example pass = PASSWORD
   *
   */
  public generatePassword(pid) {
    return GBAdminService.getRndPassword();
  }

  /**
   * Calls any REST API by using GET HTTP method.
   *
   * @example user = get "http://server/users/1"
   *
   */
  public async getByHttp({ pid, url, headers, username, ps, qs }) {
    let options = {};
    if (headers) {
      options['headers'] = headers;
    }
    if (username) {
      options['auth'] = {
        user: username,
        pass: ps
      };
    }
    if (qs) {
      options['qs'] = qs;
    }

    const result = await fetch(url, options);

    try {
      return JSON.parse(await result.text());
    } catch (error) {
      GBLog.info(`[GET]: OK.`);

      return result;
    }
  }

  /**
   * Calls any REST API by using POST HTTP method.
   *
   * @example
   *
   * user = put "http://server/path", "data"
   * talk "The updated user area is" + user.area
   *
   */
  public async putByHttp({ pid, url, data, headers }) {
    const options = {
      json: data,
      headers: headers
    };

    let result = await fetch(url, options);
    GBLog.info(`[PUT]: ${url} (${data}): ${result}`);
    return typeof result === 'object' ? result : JSON.parse(result);
  }

  /**
   * Calls any REST API by using POST HTTP method.
   *
   * @example
   *
   * user = post "http://server/path", "data"
   * talk "The updated user area is" + user.area
   *
   */
  public async postByHttp({ pid, url, data, headers }) {
    const options = {
      json: data,
      headers: headers
    };

    let result = await fetch(url, options);
    GBLog.info(`[POST]: ${url} (${data}): ${result}`);

    return result ? (typeof result === 'object' ? result : JSON.parse(result)) : true;
  }

  public async numberOnly({ pid, text }) {
    return text.replace(/\D/gi, '');
  }

  /**
   *
   * Fills a .docx or .pptx with template data.
   *
   * doc = FILL "templates/template.docx", data
   *
   */
  public async fill({ pid, templateName, data }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const botId = this.min.instance.botId;
    const gbaiName = `${botId}.gbai`;
    let localName;

    // Downloads template from .gbdrive.

    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
    let path = '/' + urlJoin(gbaiName, `${botId}.gbdrive`);
    let template = await this.internalGetDocument(client, baseUrl, path, templateName);
    let url = template['@microsoft.graph.downloadUrl'];
    const res = await fetch(url);
    let buf: any = Buffer.from(await res.arrayBuffer());
    localName = Path.join('work', gbaiName, 'cache', `tmp${GBAdminService.getRndReadableIdentifier()}.docx`);
    Fs.writeFileSync(localName, buf, { encoding: null });

    // Loads the file as binary content.

    let zip = new PizZip(buf);

    // Replace image path on all elements of data.

    const images = [];
    let index = 0;
    path = Path.join(gbaiName, 'cache', `tmp${GBAdminService.getRndReadableIdentifier()}.docx`);
    url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));

    const traverseDataToInjectImageUrl = async o => {
      for (var i in o) {
        let value = o[i];

        if (value && value.gbarray){
          o.shift();
          value = o[i];
        }

        for (const kind of ['png', 'jpg', 'jpeg']) {
          if (value.endsWith && value.endsWith(`.${kind}`)) {

            const { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);

            path = urlJoin(gbaiName, `${botId}.gbdrive`);
            if (value.indexOf('/') !== -1) {
              path = '/' + urlJoin(path, Path.dirname(value));
              value = Path.basename(value);
            }
    
            const ref = await this.internalGetDocument(client, baseUrl, path, value);
            let url = ref['@microsoft.graph.downloadUrl'];
            const imageName = Path.join(
              'work',
              gbaiName,
              'cache',
              `tmp${GBAdminService.getRndReadableIdentifier()}-${value}.png`
            );
            const response = await fetch(url);
            const buf = Buffer.from(await response.arrayBuffer());
            Fs.writeFileSync(imageName, buf, { encoding: null });
    
            const getNormalSize = ({ width, height, orientation }) => {
              return (orientation || 0) >= 5 ? [  height, width ] : [ width, height];
            };
    
            const size = getNormalSize(await sharp(buf).metadata());
            url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(imageName));
            images[index++] = {url: url, size:size, buf: buf} ;

          }
        }
        if (o[i] !== null && typeof o[i] == 'object') {
          await traverseDataToInjectImageUrl(o[i]);
        }
      }
    };

    let indexImage = 0;
    var opts = {
      fileType: 'docx',
      centered: false,
      getImage: (tagValue, tagName) => {
        return images[indexImage].buf;

      },
      getSize: (img, tagValue, tagName) => {
        return images[indexImage++].size;
      }
    };

    let doc = new Docxtemplater();
    doc.setOptions({ paragraphLoop: true, linebreaks: true });
    doc.loadZip(zip);
    if (localName.endsWith('.pptx')) {
      doc.attachModule(pptxTemplaterModule);
    }
    doc.attachModule(new ImageModule(opts));

    await traverseDataToInjectImageUrl(data);
    doc
        .setData(data)
        .render();
    
    buf = doc.getZip().generate({ type: 'nodebuffer', compression: 'DEFLATE' });
    Fs.writeFileSync(localName, buf, { encoding: null });

    return { localName: localName, url: url, data: buf };
  }

  public screenCapture(pid) {
    // scrcpy Disabled
    // function captureImage({ x, y, w, h }) {
    //   const pic = robot.screen.capture(x, y, w, h)
    //   const width = pic.byteWidth / pic.bytesPerPixel // pic.width is sometimes wrong!
    //   const height = pic.height
    //   const image = new Jimp(width, height)
    //   let red, green, blue
    //   pic.image.forEach((byte, i) => {
    //     switch (i % 4) {
    //       case 0: return blue = byte
    //       case 1: return green = byte
    //       case 2: return red = byte
    //       case 3:
    //         image.bitmap.data[i - 3] = red
    //         image.bitmap.data[i - 2] = green
    //         image.bitmap.data[i - 1] = blue
    //         image.bitmap.data[i] = 255
    //     }
    //   })
    //   return image
    // }
    // let file = 'out.png';
    // captureImage({ x: 60, y: 263, w: 250, h: 83 }).write(file)
    // const config = {
    //   lang: "eng",
    //   oem: 1,
    //   psm: 3,
    // }
    // tesseract.recognize(file, config).then(value => {
    //   console.log(value);
    // });
  }

  private numberToLetters(num) {
    let letters = '';
    while (num >= 0) {
      letters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'[num % 26] + letters;
      num = Math.floor(num / 26) - 1;
    }
    return letters;
  }

  /**
   * Merges a multi-value with a tabular file using BY field as key.
   *
   * @example
   *
   *  data = FIND first.xlsx
   *  MERGE "second.xlsx" WITH data BY customer_id
   *
   */
  public async merge({ pid, file, data, key1, key2 }): Promise<any> {
    GBLog.info(`BASIC: MERGE running on ${file} and key1: ${key1}, key2: ${key2}...`);

    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const botId = min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbdata`;

    // MAX LINES property.

    let maxLines = 1000;
    if (this.dk.user && this.dk.user.basicOptions && this.dk.user.basicOptions.maxLines) {
      if (this.dk.user.basicOptions.maxLines.toString().toLowerCase() !== 'default') {
        maxLines = Number.parseInt(this.dk.user.basicOptions.maxLines).valueOf();
      }
    }

    // Choose data sources based on file type (HTML Table, data variable or sheet file)

    let results;
    let header, rows;
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

    let document;
    document = await this.internalGetDocument(client, baseUrl, path, file);

    // Creates workbook session that will be discarded.

    let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();

    results = await client
      .api(
        `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A1:CZ${maxLines}')`
      )
      .get();

    header = results.text[0];
    rows = results.text;

    // As BASIC uses arrays starting with 1 (one) as index,
    // a ghost element is added at 0 (zero) position.

    let table = [];
    table.push({ gbarray: '0' });
    let foundIndex = 1;

    // Fills the row variable.

    for (; foundIndex < rows.length; foundIndex++) {
      let row = {};
      const xlRow = rows[foundIndex];
      for (let colIndex = 0; colIndex < xlRow.length; colIndex++) {
        const propertyName = header[colIndex];
        let value = xlRow[colIndex];
        if (value && value.charAt(0) === "'") {
          if (this.isValidDate(value.substr(1))) {
            value = value.substr(1);
          }
        }
        row[propertyName] = value;
      }
      row['line'] = foundIndex + 1;
      table.push(row);
    }

    let key1Index, key2Index;

    if (key1) {
      key1Index = _.invertBy(table, key1);
    }

    if (key2) {
      key2Index = _.invertBy(table, key2);
    }

    let merges = 0,
      adds = 0;

    // Scans all items in incoming data.

    for (let i = 1; i < data.length; i++) {
      // Scans all sheet lines and compare keys.

      const row = data[i];
      let found;
      if (key1Index) {
        const key1Value = row[key1];
        const foundRow = key1Index[key1Value];
        if (foundRow) {
          found = table[foundRow[0]];
        }
      }

      if (found) {
        let keys = Object.keys(row);
        for (let j = 0; j < keys.length; j++) {
          const columnName = header[j];
          const value = row[keys[j]];
          const cell = `${this.numberToLetters(j)}${i + 1}`;
          const address = `${cell}:${cell}`;

          if (value !== found[columnName]) {
            await this.set({ pid, file, address, value });
            merges++;
          }
        }
      } else {
        let args = [file];
        let keys = Object.keys(row);
        for (let j = 0; j < keys.length; j++) {
          args.push(row[keys[j]]);
        }

        await this.save({ pid, args });
        adds++;
      }
    }

    if (table.length === 1) {
      GBLog.info(`BASIC: MERGE ran but updated zero rows.`);
      return null;
    } else {
      GBLog.info(`BASIC: MERGE updated (merges:${merges}, additions:${adds}.`);
      return table;
    }
  }

  public async tweet({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    const consumer_key = min.core.getParam(min.instance, 'Twitter Consumer Key', null);
    const consumer_secret = min.core.getParam(min.instance, 'Twitter Consumer Key Secret', null);
    const access_token_key = min.core.getParam(min.instance, 'Twitter Access Token', null);
    const access_token_secret = min.core.getParam(min.instance, 'Twitter Access Token Secret', null);

    if (!consumer_key || !consumer_secret || !access_token_key || !access_token_secret) {
      GBLog.info('Twitter not configured in .gbot.');
    }

    const client = new TwitterApi({
      appKey: consumer_key,
      appSecret: consumer_secret,
      accessToken: access_token_key,
      accessSecret: access_token_secret
    });

    await client.v2.tweet(text);
    GBLog.info(`Twitter Automation: ${text}.`);
  }
}
