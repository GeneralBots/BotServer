/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/
'use strict';

import ComputerVisionClient from '@azure/cognitiveservices-computervision';
import ApiKeyCredentials from '@azure/ms-rest-js';
import { BlobServiceClient, BlockBlobClient, StorageSharedKeyCredential } from '@azure/storage-blob';
import { DataTypes, Sequelize } from '@sequelize/core';
import ai2html from 'ai2html';
import alasql from 'alasql';
import retry from 'async-retry';
import { GBLog } from 'botlib';
import csvdb from 'csv-database';
import Docxtemplater from 'docxtemplater';
import Excel from 'exceljs';
import { Page } from 'facebook-nodejs-business-sdk';
import fs from 'fs/promises';
import { IgApiClient } from 'instagram-private-api';
import { BufferWindowMemory } from 'langchain/memory';
import _ from 'lodash';
import mime from 'mime-types';
import ImageModule from 'open-docxtemplater-image-module';
import path from 'path';
import { pdfToPng, PngPageOutput } from 'pdf-to-png-converter';
import PizZip from 'pizzip';
import pptxTemplaterModule from 'pptxtemplater';
import { CollectionUtil } from 'pragmatismo-io-framework';
import urlJoin from 'url-join';
import { setFlagsFromString } from 'v8';
import { runInNewContext } from 'vm';
import { GBServer } from '../../../src/app.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GBSSR } from '../../core.gbapp/services/GBSSR.js';
import { ChatServices } from '../../llm.gblib/services/ChatServices.js';
import { SecService } from '../../security.gbapp/services/SecService.js';
import { DialogKeywords } from './DialogKeywords.js';
import { GBVMService } from './GBVMService.js';
import { KeywordsExpressions } from './KeywordsExpressions.js';
import { WebAutomationServices } from './WebAutomationServices.js';

import { md5 } from 'js-md5';
import { Client } from 'minio';
import { GBUtil } from '../../../src/util.js';

/**
 * @fileoverview General Bots server core.
 */

/**
 * BASIC system class for extra manipulation of bot behaviour.
 */
export class SystemKeywords {
  /**
   * @tags System
   */
  public async callVM({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const step = null;
    const deployer = null;

    return await GBVMService.callVM(text, min, step, pid, false, [text]);
  }

  public async append({ pid, args }) {
    if (!args) return [];
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

    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );
    GBLogEx.info(min, `GBVision (caption): '${caption.text}' (Confidence: ${caption.confidence.toFixed(2)})`);

    return await min.conversationalService.translate(min, caption.text, contentLocale);
  }

  /**
   *
   * @example SEE TEXT OF url AS variable
   *
   */
  public async seeText({ pid, url }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
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

    GBLogEx.info(min, `GBVision (text): '${final}'`);
    return final;
  }

  public async sortBy({ pid, array, memberName }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    memberName = memberName.trim();
    const contentLocale = min.core.getParam(
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
      if (data[0].gbarray) {
        return data;
      } // Already GB Table.
      if (Array.isArray(data)) {
        isObject = Object.keys(data[1]) !== null;
      } else {
        isObject = true;
      }

      if (isObject || JSON.parse(data) !== null) {
        // Copies data from JSON format into simple array.

        if (!Array.isArray(data)) {
          // If data is a single object, wrap it in an array
          data = [data];
        }

        // Ensure that keys is an array of strings representing the object keys
        const keys = Object.keys(data[0]);

        if (headers) {
          output[0] = [];

          // Copies headers as the first element.

          for (let i = 0; i < keys.length; i++) {
            output[0][i] = keys[i];
          }
        } else {
          output.push({ gbarray: '0' });
        }

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
   * @param renderImage
   * @returns
   *
   * @see http://tabulator.info/examples/5.2
   */
  private async renderTable(pid, data, renderPDF, renderImage) {
    if (data.length && !data[1]) {
      return null;
    }

    data = SystemKeywords.JSONAsGBTable(data, true);

    // Detects if it is a collection with repeated
    // headers.

    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const gbaiName = GBUtil.getGBAIPath(min.botId);
    const browser = await GBSSR.createBrowser(null);
    const page = await browser.newPage();
    await page.minimize();

    // Includes the associated CSS related to current theme.

    const theme: string = await DialogKeywords.getOption({ pid, name: 'theme', root: true });
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
        height:"auto",
        layout:"fitDataStretch",
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
      localName = path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.png`);
      await page.screenshot({ path: localName, fullPage: true });
      url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));
      GBLogEx.info(min, `Table image generated at ${url} .`);
    }

    // Handles PDF generation.

    if (renderPDF) {
      localName = path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.pdf`);
      url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));
      let pdf = await page.pdf({ format: 'A4' });
      GBLogEx.info(min, `Table PDF generated at ${url} .`);
    }

    await browser.close();
    return { url, localName };
  }

  public async closeHandles({ pid }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const memoryBeforeGC = process.memoryUsage().heapUsed / 1024 / 1024; // in MB

    delete this.cachedMerge[pid];

    // Capture memory usage before GC
    GBLogEx.info(min, ``);

    setFlagsFromString('--expose_gc');
    const gc = runInNewContext('gc'); // nocommit
    gc();

    // Capture memory usage after GC
    const memoryAfterGC = process.memoryUsage().heapUsed / 1024 / 1024; // in MB
    GBLogEx.info(
      min,
      `BASIC: Closing Handles... From ${memoryBeforeGC.toFixed(2)} MB to ${memoryAfterGC.toFixed(2)} MB`
    );
  }

  public async asPDF({ pid, data }) {
    let file = await this.renderTable(pid, data, true, false);
    return file;
  }

  public async asImage({ pid, data }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    // Checks if it is a GBFILE.

    if (data.data) {
      const gbfile = data.data;

      let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
      const botId = min.instance.botId;
      const gbaiName = GBUtil.getGBAIPath(min.botId);
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

      const localName = path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.png`);
      if (pngPages.length > 0) {
        const buffer = pngPages[0].content;
        const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));

        await fs.writeFile(localName, new Uint8Array(buffer), { encoding: null });

        return { localName: localName, url: url, data: buffer };
      }
    } else {
      let file = await this.renderTable(pid, data, false, true);
      return file;
    }
  }

  public async executeSQL({ pid, data, sql }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    if (!data || !data[0]) {
      return data;
    }
    let objectMode = false;
    if (data[0].gbarray) {
      objectMode = true;
    }

    let first;
    if (objectMode) {
      first = data.shift();
    }
    GBLogEx.info(min, `Executing SQL: ${sql}`);
    data = alasql(sql, [data]);
    if (objectMode) {
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
    const idGeneration = '1v'; // TODO:  this.dk['idGeneration'];
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
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    // tslint:disable-next-line no-string-based-set-timeout
    GBLogEx.info(min, `WAIT for ${seconds} second(s).`);
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
    GBLogEx.info(min, `Talking '${message}' to a specific user (${mobile}) (TALK TO). `);
    await min.conversationalService.sendMarkdownToMobile(min, null, mobile, message);
  }

  /**
   * Get a user object from a alias.
   *
   * @example user = USER "someone"
   *
   */
  public async getUser({ pid, username }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    let sec = new SecService();
    const user = await sec.getUserFromUsername(min.instance.instanceId, username);

    return { displayName: user.displayName, mobile: user.userSystemId, email: user.email };
  }

  /**
   * Sends a SMS message to the mobile number specified.
   *
   * @example SEND SMS TO "+199988887777", "Message text here"
   *
   */
  public async sendSmsTo({ pid, mobile, message }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `SEND SMS TO '${mobile}', message '${message}'.`);
    await min.conversationalService.sendSms(min, mobile, message);
  }

  /**
   * 1. Defines a cell value in the tabular file.
   * 2. Defines an element text on HTML page.
   *
   * @example SET "file.xlsx", "A2", 4500
   *
   * @example SET page, "elementHTMLSelector", "text"

   */
  public async set({ pid, handle, file, address, value, name = null }): Promise<any> {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    // Handles calls for HTML stuff

    if (handle && WebAutomationServices.isSelector(file)) {
      GBLogEx.info(min, `Web automation SET ${file}' to '${address}'    . `);
      await new WebAutomationServices().setElementText({ pid, handle, selector: file, text: address });

      return;
    }

    // TODO: Add a semaphore between FILTER and SET.

    // Processes FILTER option to ensure parallel SET calls.

    const filter = await DialogKeywords.getOption({ pid, name: 'filter' });
    if (filter) {
      const row = this.find({ pid, handle: null, args: [filter] });
      address += row['line'];
    }

    // Handles calls for BASIC persistence on sheet files.

    GBLogEx.info(min, `Defining '${address}' in '${file}' to '${value}' (SET). `);

    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

    const botId = min.instance.botId;
    const packagePath = GBUtil.getGBAIPath(botId, 'gbdata');
    let document = await this.internalGetDocument(client, baseUrl, packagePath, file);
    let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();
    let body = { values: [[]] };

    // Processes FILTER option to ensure parallel SET calls.

    let titleAddress;

    if (filter) {
      // Transforms address number (col index) to letter based.
      // Eg.: REM This is A column and index automatically specified by filter.
      //      SET file.xlsx, 1, 4000

      if (KeywordsExpressions.isNumber(address)) {
        address = `${this.numberToLetters(address)}`;
        titleAddress = `${address}1:${address}1`;
      }

      // Processes SET FILTER directive to calculate address.

      body.values[0][0] = 'id';
      const addressId = 'A1:A1';
      await client
        .api(
          `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${addressId}')`
        )
        .patch(body);

      const row = await this.find({ pid, handle: null, args: [file, filter] });
      if (row) {
        address += row['line']; // Eg.: "A" + 1 = "A1".
      }
    }
    address = address.indexOf(':') !== -1 ? address : address + ':' + address;

    if (titleAddress) {
      body.values[0][0] = name.trim().replace(/[^a-zA-Z]/gi, '');

      await client
        .api(
          `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${titleAddress}')`
        )
        .patch(body);
    }

    body.values[0][0] = value;
    await client
      .api(
        `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${address}')`
      )
      .patch(body);
  }

  /**
   * Retrives a document from the drive, given a path and filename.
   */
  public async internalGetDocument(client: any, baseUrl: any, path: string, file: string) {
    let res = await client.api(`${baseUrl}/drive/root:/${path}:/children`).get();

    let documents = res.value.filter(m => {
      return m.name.toLowerCase() === file.toLowerCase();
    });

    if (!documents || documents.length === 0) {
      throw new Error(
        `File '${file}' specified on GBasic command not found. Check the .gbdata or the .gbdialog associated.`,
        { cause: 404 }
      );
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
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `Saving '${file}' (SAVE file).`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const botId = min.instance.botId;
    const packagePath = GBUtil.getGBAIPath(min.botId, `gbdrive`);

    // Checks if it is a GB FILE object.

    if (data.data && data.filename) {
      data = data.data;
    }

    try {
      data = GBServer.globals.files[data].data; // TODO
      await client.api(`${baseUrl}/drive/root:/${packagePath}/${file}:/content`).put(data);
    } catch (error) {
      if (error.code === 'itemNotFound') {
        GBLogEx.info(min, `BASIC source file not found: ${file}.`);
      } else if (error.code === 'nameAlreadyExists') {
        GBLogEx.info(min, `BASIC destination file already exists: ${file}.`);
      }
      throw error;
    }
  }

  /**
   * Saves the content of variable into BLOB storage.
   *
   * MSFT uses MD5, see https://katelynsills.com/law/the-curious-case-of-md5.
   *
   * @exaple UPLOAD file.
   *
   */
  public async uploadFile({ pid, file }): Promise<any> {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `UPLOAD '${file.name}' ${file.size} bytes.`);

    // Checks if it is a GB FILE object.

    const accountName = min.core.getParam(min.instance, 'Blob Account');
    const accountKey = min.core.getParam(min.instance, 'Blob Key');

    const sharedKeyCredential = new StorageSharedKeyCredential(accountName, accountKey);
    const baseUrl = `https://${accountName}.blob.core.windows.net`;

    const blobServiceClient = new BlobServiceClient(`${baseUrl}`, sharedKeyCredential);

    // It is an SharePoint object that needs to be downloaded.

    const gbaiName = GBUtil.getGBAIPath(min.botId);
    const localName = path.join('work', gbaiName, 'cache', `${GBAdminService.getRndReadableIdentifier()}.tmp`);
    const url = file['url'];
    const response = await fetch(url);

    // Writes it to disk and calculate hash.

    const data = await response.arrayBuffer();
    await fs.writeFile(localName, new Uint8Array(Buffer.from(data)), { encoding: null });
    const hash = new Uint8Array(md5.array(data));

    // Performs uploading passing local hash.

    const container = blobServiceClient.getContainerClient(accountName);
    const blockBlobClient: BlockBlobClient = container.getBlockBlobClient(file.path);
    const res = await blockBlobClient.uploadFile(localName, {
      blobHTTPHeaders: {
        blobContentMD5: hash
      }
    });

    // If upload is OK including hash check, removes the temporary file.

    if (res._response.status === 201 && new Uint8Array(res.contentMD5).toString() === hash.toString()) {
      fs.rm(localName);

      file['md5'] = hash.toString();

      return file;
    } else {
      GBLog.error(`BLOB HTTP ${res.errorCode} ${res._response.status} .`);
    }
  }

  /**
   * Takes note inside a notes.xlsx of .gbdata.
   *
   * @example NOTE "text"
   *
   */
  public async note({ pid, text }): Promise<any> {
    await this.save({ pid, file: 'Notes.xlsx', args: [text] });
  }

  /**
   * Saves variables to storage, not a worksheet.
   *
   */
  public async saveToStorageBatch({ pid, table, rows }): Promise<void> {
    const { min } = await DialogKeywords.getProcessInfo(pid);

    if (!Array.isArray(rows) && typeof rows === 'object' && rows !== null) {
      rows = [rows];
    }

    if (rows.length === 0) {
      return;
    }

    const t = this.getTableFromName(table, min);
    let rowsDest = [];

    rows.forEach(row => {
      if (GBUtil.hasSubObject(row)) {
        row = this.flattenJSON(row);
      }

      let dst = {};
      let i = 0;
      Object.keys(row).forEach(column => {
        const field = column.charAt(0).toUpperCase() + column.slice(1);
        dst[field] = row[column];
        i++;
      });
      rowsDest.push(dst);
      dst = null;
      row = null;
    });
    GBLogEx.info(min, `SAVE '${table}': ${rows.length} row(s).`);

    // Capture the values we need for retries
    const tableName = table;
    const minRef = min;

    await retry(
      async (bail) => {
        const t = this.getTableFromName(tableName, minRef);
        try {
          await t.bulkCreate(rowsDest);
          rowsDest = null;
        } catch (error) {
          throw error;
        }
      },
      {
        retries: 5,
        onRetry: err => {
          GBLog.error(`SAVE (retry): ${GBUtil.toYAML(err)}.`);
        }
      }
    );
  }

  /**
   * Saves variables to storage, not a worksheet.
   *
   * @example SAVE "Billing",  columnName1, columnName2
   *
   */

  public async saveToStorageWithJSON({ pid, table, fieldsValues, fieldsNames }): Promise<any> {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `Saving to storage '${table}' (SAVE).`);
    const minBoot = GBServer.globals.minBoot as any;
    const definition = minBoot.core.sequelize.models[table];

    let out = [];
    let data = {},
      data2 = {};

    // Flattern JSON to a table.

    data = this.flattenJSON(fieldsValues);

    // Uppercases fields.

    Object.keys(data).forEach(field => {
      const field2 = field.charAt(0).toUpperCase() + field.slice(1);
      data2[field2] = data[field];
    });

    return await definition.create(data2);
  }

  /**
   * Saves the content of several variables to a new row in a tabular file.
   *
   * @example SAVE "customers.csv", name, email, phone, address, city, state, country
   *
   */
  /**
   * Saves the content of several variables to a new row in a tabular file.
   *
   * @example SAVE "customers.csv", id, name, email, phone
   *
   */
  public async save({ pid, file, args }): Promise<any> {
    if (!args) {
      return;
    }

    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `Saving '${file}' (SAVE). Args: ${args.join(',')}.`);

    // Handle gbcluster mode with Minio storage
    if (GBConfigService.get('GB_MODE') === 'gbcluster') {
      const fileUrl = urlJoin('/', `${min.botId}.gbdata`, file);
      GBLogEx.info(min, `Direct data from .csv: ${fileUrl}.`);

      const fileOnly = fileUrl.substring(fileUrl.lastIndexOf('/') + 1);

      const minioClient = new Client({
        endPoint: process.env.DRIVE_SERVER || 'localhost',
        port: parseInt(process.env.DRIVE_PORT || '9000', 10),
        useSSL: process.env.DRIVE_USE_SSL === 'true',
        accessKey: process.env.DRIVE_ACCESSKEY,
        secretKey: process.env.DRIVE_SECRET,
      });

      const gbaiName = GBUtil.getGBAIPath(min.botId);
      const bucketName = (process.env.DRIVE_ORG_PREFIX + min.botId + '.gbai').toLowerCase();
      const localName = path.join(
        'work',
        gbaiName,
        'cache',
        `${fileOnly.replace(/\s/gi, '')}-${GBAdminService.getNumberIdentifier()}.csv`
      );

      try {
        // Lock the file for editing
        await this.lockFile(minioClient, bucketName, fileUrl);

        // Download the file
        await minioClient.fGetObject(bucketName, fileUrl, localName);

        // Read the CSV file
        let csvData = await fs.readFile(localName, 'utf8');
        let rows = csvData.split('\n').filter(row => row.trim() !== '');

        // Check if first column is ID
        const headers = rows.length > 0 ? rows[0].split(',') : [];
        const hasIdColumn = headers.length > 0 && headers[0].toLowerCase() === 'id';

        // If ID exists in args[0] and we have an ID column, try to find and update the row
        let rowUpdated = false;
        if (hasIdColumn && args[0]) {
          for (let i = 1; i < rows.length; i++) {
            const rowValues = rows[i].split(',');
            if (rowValues[0] === args[0]) {
              // Update existing row
              rows[i] = args.join(',');
              rowUpdated = true;
              break;
            }
          }
        }

        // If no row was updated, add a new row
        if (!rowUpdated) {
          rows.push(args.join(','));
        }

        // Write back to the file
        await fs.writeFile(localName, rows.join('\n'));

        // Upload the updated file
        await minioClient.fPutObject(bucketName, fileUrl, localName);

        GBLogEx.info(min, `Successfully saved data to Minio storage: ${fileUrl}`);
      } catch (error) {
        GBLogEx.error(min, `Error saving to Minio storage: ${error.message}`);
        throw error;
      } finally {
        // Ensure the file is unlocked
        await this.unlockFile(minioClient, bucketName, fileUrl);
        // Clean up the local file
        try {
          await fs.unlink(localName);
        } catch (cleanupError) {
          GBLogEx.info(min, `Could not clean up local file: ${cleanupError.message}`);
        }
      }
      return;
    }

    // Original legacy mode handling
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const botId = min.instance.botId;
    const packagePath = GBUtil.getGBAIPath(botId, 'gbdata');

    let sheets;
    let document;
    try {
      document = await this.internalGetDocument(client, baseUrl, packagePath, file);
      sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();
    } catch (e) {
      if (e.cause === 404) {
        // Creates the file.
        const blank = path.join(process.env.PWD, 'blank.xlsx');
        const data = await fs.readFile(blank);
        await client.api(`${baseUrl}/drive/root:/${packagePath}/${file}:/content`).put(data);

        // Tries to open again.
        document = await this.internalGetDocument(client, baseUrl, packagePath, file);
        sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();
      } else {
        throw e;
      }
    }

    let address;
    let body = { values: [[]] };

    // Check if first column is ID
    const firstCell = await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A1:A1')`)
      .get();

    const hasIdColumn = firstCell.text.toLowerCase() === 'id';

    // If ID exists in args[0] and we have an ID column, try to find and update the row
    let rowUpdated = false;
    if (hasIdColumn && args[0]) {
      const allRows = await client
        .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/usedRange`)
        .get();

      for (let i = 1; i < allRows.values.length; i++) {
        if (allRows.values[i][0] === args[0]) {
          // Update existing row
          address = `A${i + 1}:${this.numberToLetters(args.length - 1)}${i + 1}`;
          for (let j = 0; j < args.length; j++) {
            body.values[0][j] = args[j];
          }
          rowUpdated = true;
          break;
        }
      }
    }

    // If no row was updated, add a new row
    if (!rowUpdated) {
      await client
        .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A2:DX2')/insert`)
        .post({});
      address = `A2:${this.numberToLetters(args.length - 1)}2`;
      for (let j = 0; j < args.length; j++) {
        body.values[0][j] = args[j];
      }
    }

    await retry(
      async bail => {
        const result = await client
          .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${address}')`)
          .patch(body);

        if (result.status != 200) {
          GBLogEx.info(min, `Waiting 5 secs. before retrying HTTP ${result.status} GET: ${result.url}`);
          await GBUtil.sleep(5 * 1000);
          throw new Error(`HTTP:${result.status} retry: ${result.statusText}.`);
        }
      },
      {
        retries: 5,
        onRetry: error => {
          GBLog.error(`Retrying HTTP GET due to: ${error.message}.`);
        }
      }
    );
  }

  // Helper methods for Minio file locking (unchanged)
  private async lockFile(minioClient: Client, bucketName: string, filePath: string): Promise<void> {
    const lockFile = `${filePath}.lock`;
    try {
      await minioClient.statObject(bucketName, lockFile);
      throw new Error(`File ${filePath} is currently locked for editing`);
    } catch (error) {
      if (error.code === 'NotFound') {
        // Create lock file
        await minioClient.putObject(bucketName, lockFile, 'locked');
        return;
      }
      throw error;
    }
  }

  private async unlockFile(minioClient: Client, bucketName: string, filePath: string): Promise<void> {
    const lockFile = `${filePath}.lock`;
    try {
      await minioClient.removeObject(bucketName, lockFile);
    } catch (error) {
      GBLog.error(`Error removing lock file: ${error.message}`);
    }
  }
  /**
   * Retrives the content of a cell in a tabular file.
   *
   * @example value = GET "file.xlsx", "A2"
   *
   */
  public async getHttp({ pid, file, addressOrHeaders, httpUsername, httpPs, qs, streaming }): Promise<any> {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
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
      GBLogEx.info(min, `GET '${addressOrHeaders}' in '${file}'.`);
      let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
      const botId = min.instance.botId;

      const packagePath = GBUtil.getGBAIPath(botId, 'gbdata');

      let document = await this.internalGetDocument(client, baseUrl, packagePath, file);

      // Creates workbook session that will be discarded.

      let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();

      let results = await client
        .api(
          `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${addressOrHeaders}')`
        )
        .get();

      let val = results.text[0][0];
      GBLogEx.info(min, `Getting '${file}' (GET). Value= ${val}.`);
      return val;
    }
  }

  public async isValidDate({ pid, dt }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const contentLocale = min.core.getParam(
      min.instance,
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

  public async isValidNumber({ pid, number }) {
    return KeywordsExpressions.isNumber(number);
  }

  public isValidHour({ pid, value }) {
    return /^([01]?[0-9]|2[0-3]):[0-5][0-9]$/.test(value);
  }

  public static async getFilter(text) {
    let filter;
    const operators = [/\<\=/, /\<\>/, /\>\=/, /\</, /\>/, /\blike\b/, /\bnot in\b/, /\bin\b/, /\=/];
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
  }

  /**
   * Finds a value or multi-value results in a tabular file.
   *
   * @example
   *
   *  rows = FIND "file.xlsx", "A2=active", "A2 < 12/06/2010 15:00"
   *  i = 1
   *  do while i <= ubound(row)
   *    row = rows[i]
   *    send sms to "+" + row.mobile, "Hello " + row.name + "! "
   *  loop
   * @see NPM package data-forge
   *
   */
  public async find({ pid, handle, args }): Promise<any> {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    const file = args[0];
    args.shift();

    const botId = min.instance.botId;
    const packagePath = GBUtil.getGBAIPath(botId, 'gbdata');

    // MAX LINES property.

    let maxLines = 5000;
    if (params && params.maxLines) {
      if (params.maxLines.toString().toLowerCase() !== 'default') {
        maxLines = Number.parseInt(params.maxLines).valueOf();
      }
    } else {
      maxLines = maxLines;
    }
    GBLogEx.info(min, `FIND running on ${file} (maxLines: ${maxLines}) and args: ${JSON.stringify(args)}...`);

    // Choose data sources based on file type (HTML Table, data variable or sheet file)

    let results;
    let header, rows;
    let page;
    if (handle) {
      page = WebAutomationServices.getPageByHandle(handle);
    }

    if (handle && page['$eval'] && WebAutomationServices.isSelector(file)) {
      const container = page['frame'] ? page['frame'] : page;
      const originalSelector = file;

      // Transforms table

      let resultH = await container.evaluate(originalSelector => {
        const rows = document.querySelectorAll(`${originalSelector} tr`);
        return Array.from(rows, row => {
          const columns = row.querySelectorAll('th');
          return Array.from(columns, column => column.innerText);
        });
      }, originalSelector);

      let result = await container.evaluate(originalSelector => {
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
      resultH = null;

      rows = [];
      rows[0] = header;
      for (let i = 1; i < result.length; i++) {
        rows[i] = result[i];
      }
      result = null;
    } else if (file['cTag']) {
      const gbaiName = GBUtil.getGBAIPath(min.botId);
      const localName = path.join('work', gbaiName, 'cache', `csv${GBAdminService.getRndReadableIdentifier()}.csv`);
      const url = file['@microsoft.graph.downloadUrl'];
      const response = await fetch(url);
      await fs.writeFile(localName, new Uint8Array(Buffer.from(await response.arrayBuffer())), { encoding: null });

      var workbook = new Excel.Workbook();
      let worksheet = await workbook.csv.readFile(localName);
      header = [];
      rows = [];

      for (let i = 0; i < worksheet.rowCount; i++) {
        const r = worksheet.getRow(i + 1);
        let outRow = [];
        let hasValue = false;
        for (let j = 0; j < r.cellCount; j++) {
          const value = r.getCell(j + 1).text;
          if (value) {
            hasValue = true;
          }
          outRow.push(value);
        }

        if (i == 0) {
          header = outRow;
        } else if (hasValue) {
          rows.push(outRow);
        }
      }
      worksheet = null;
    } else if (file.indexOf('.xlsx') !== -1) {
      let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

      let document;
      document = await this.internalGetDocument(client, baseUrl, packagePath, file);

      // Creates workbook session that will be discarded.

      let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();

      results = await client
        .api(
          `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A1:CZ${maxLines}')`
        )
        .get();

      header = results.text[0];
      rows = results.text;
    } else if (file.indexOf('.csv') !== -1) {
      let res;
      let packagePath = GBUtil.getGBAIPath(min.botId, `gbdata`);

      if (GBConfigService.get('GB_MODE') === 'gbcluster') {

        const fileUrl = urlJoin('/', `${min.botId}.gbdata`, file);
        GBLogEx.info(min, `Direct data from .csv: ${fileUrl}.`);

        const fileOnly = fileUrl.substring(fileUrl.lastIndexOf('/') + 1);

        const minioClient = new Client({
          endPoint: process.env.DRIVE_SERVER || 'localhost',
          port: parseInt(process.env.DRIVE_PORT || '9000', 10),
          useSSL: process.env.DRIVE_USE_SSL === 'true',
          accessKey: process.env.DRIVE_ACCESSKEY,
          secretKey: process.env.DRIVE_SECRET,
        });

        const gbaiName = GBUtil.getGBAIPath(min.botId);
        const bucketName = (process.env.DRIVE_ORG_PREFIX + min.botId + '.gbai').toLowerCase();
        const localName = path.join(
          'work',
          gbaiName,
          'cache',
          `${fileOnly.replace(/\s/gi, '')}-${GBAdminService.getNumberIdentifier()}.csv`
        );

        await minioClient.fGetObject(bucketName, fileUrl, localName);
      }

      const csvFile = path.join(GBConfigService.get('STORAGE_LIBRARY'), packagePath, file);
      const data = await fs.readFile(csvFile, 'utf8');

      const firstLine = data.split('\n')[0];
      const headers = firstLine.split(',');
      const db = await csvdb(csvFile, headers, ',');
      if (args[0]) {
        const systemFilter = await SystemKeywords.getFilter(args[0]);
        let filter = {};
        filter[systemFilter.columnName] = systemFilter.value;
        res = await db.get(filter);
      } else {
        res = await db.get();
      }

      return res.length > 1 ? res : res[0];
    } else {
      const t = this.getTableFromName(file, min);

      if (!t) {
        throw new Error(`TABLE ${file} not found. Check TABLE keywords.`);
      }
      let res;
      if (args[0]) {
        const systemFilter = await SystemKeywords.getFilter(args[0]);
        let filter = {};
        filter[systemFilter.columnName] = systemFilter.value;
        res = await t.findAll({ where: filter });
      } else {
        res = await t.findAll();
      }

      return res.length > 1 ? res : res[0];
    }

    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    // Increments columnIndex by looping until find a column match.

    let filters = [];
    let predefinedFilterTypes;
    if (params.filterTypes) {
      predefinedFilterTypes = params.filterTypes.split(',');
    }

    let filterIndex = 0;
    await CollectionUtil.asyncForEach(args, async arg => {
      const filter = await SystemKeywords.getFilter(arg);
      if (!filter) {
        throw new Error(`FIND filter has an error: ${arg} check this and publish .gbdialog again.`);
      }

      let columnIndex = 0;
      for (; columnIndex < header.length; columnIndex++) {
        if (header[columnIndex].toLowerCase() === filter.columnName.toLowerCase()) {
          break;
        }
      }
      filter.columnIndex = columnIndex;
      const fixed = predefinedFilterTypes ? predefinedFilterTypes[filterIndex] : null;

      if (this.isValidHour(filter.value)) {
        filter.dataType = fixed ? fixed : 'hourInterval';
      } else if (await this.isValidDate({ pid, dt: filter.value })) {
        filter.value = SystemKeywords.getDateFromLocaleString(pid, filter.value, contentLocale);
        filter.dataType = fixed ? fixed : 'date';
      } else if (await this.isValidNumber({ pid, number: filter.value })) {
        filter.value = Number.parseInt(filter.value);
        filter.dataType = fixed ? fixed : 'number';
      } else {
        filter.value = filter.value;
        filter.dataType = fixed ? fixed : 'string';
      }
      filters.push(filter);
      filterIndex++;
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
        if (user && params && params.wholeWord) {
          wholeWord = params.wholeWord;
        }
        if (!result) {
          return;
        }

        switch (filter.dataType) {
          case 'string':
            const v1 = GBConversationalService.removeDiacritics(result.toLowerCase().trim());
            const v2 = GBConversationalService.removeDiacritics(filter.value.toLowerCase().trim());
            GBLogEx.info(min, `FIND filter: ${v1} ${filter.operator} ${v2}.`);

            switch (filter.operator) {
              case '=':
                if (v1 === v2) {
                  filterAcceptCount++;
                }
                break;
              case '<>':
                if (v1 !== v2) {
                  filterAcceptCount++;
                }
                break;
              case 'not in':
                if (v1.indexOf(v2) === -1) {
                  filterAcceptCount++;
                }
                break;
              case 'in':
                if (wholeWord) {
                  if (v1 === v2) {
                    filterAcceptCount++;
                  }
                } else {
                  if (v1.indexOf(v2) > -1) {
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
                if (v1 === v2) {
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
            if (resultDate) {
              if (filter.value['dateOnly']) {
                resultDate.setHours(0, 0, 0, 0);
              }
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
        let xlRow = rows[foundIndex];
        let hasValue = false;
        for (let colIndex = 0; colIndex < xlRow.length; colIndex++) {
          const propertyName = header[colIndex].trim();

          let value = xlRow[colIndex];
          if (value) {
            hasValue = true;
            value = value.trim();
            if (value.charAt(0) === "'") {
              if (await this.isValidDate({ pid, dt: value.substr(1) })) {
                value = value.substr(1);
              }
            }
          }

          row[propertyName] = value;
          value = null;
        }
        xlRow = null;
        row['ordinal'] = rowCount;
        row['line'] = foundIndex + 1;
        if (hasValue) {
          table.push(row);
        }
        row = null;
      }
    }

    const outputArray = await DialogKeywords.getOption({ pid, name: 'output' });
    filters = null;
    header = null;
    rows = null;

    if (table.length === 1) {
      GBLogEx.info(min, `FIND returned no results (zero rows).`);
      return null;
    } else if (table.length === 2 && !outputArray) {
      GBLogEx.info(min, `FIND returned single result: ${table[0]}.`);
      return table[1];
    } else {
      GBLogEx.info(min, `FIND returned multiple results (Count): ${table.length - 1}.`);
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

  public async setSystemPrompt({ pid, text }) {
    let { min, user } = await DialogKeywords.getProcessInfo(pid);

    if (user) {
      ChatServices.userSystemPrompt[user.userSystemId] = text;

      const packagePath = GBUtil.getGBAIPath(min.botId);
      const systemPromptFile = urlJoin(
        process.cwd(),
        'work',
        packagePath,
        'users',
        user.userSystemId,
        'systemPrompt.txt'
      );
      await fs.writeFile(systemPromptFile, text);
    }
  }

  /**
   * Creates a folder in the bot instance drive.
   *
   * @example folder = CREATE FOLDER "notes\01"
   *
   */
  public async createFolder({ pid, name }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const botId = min.instance.botId;
    let packagePath = GBUtil.getGBAIPath(min.botId, `gbdrive`);

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
        lastFolder = await client.api(`${baseUrl}/drive/root:/${packagePath}:/children`).post(body);
      } catch (error) {
        if (error.code !== 'nameAlreadyExists') {
          throw error;
        } else {
          lastFolder = await client.api(`${baseUrl}/drive/root:/${urlJoin(packagePath, item)}`).get();
        }
      }

      // Increments path to the next child be created.

      packagePath = urlJoin(packagePath, item);
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
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const packagePath = GBUtil.getGBAIPath(min.botId, `gbdrive`);
    const root = urlJoin(packagePath, folder);

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

  public async internalCreateDocument(min, filePath, content) {
    GBLogEx.info(min, `CREATE DOCUMENT '${filePath}...'`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const gbaiName = GBUtil.getGBAIPath(min.botId);
    const tmpDocx = urlJoin(gbaiName, filePath);

    // Templates a blank {content} tag inside the blank.docx.

    const blank = path.join(process.env.PWD, 'blank.docx');
    let buf = await fs.readFile(blank);
    let zip = new PizZip(buf);
    let doc = new Docxtemplater();
    doc.setOptions({ linebreaks: true });
    doc.loadZip(zip);
    doc.setData({ content: content }).render();
    buf = doc.getZip().generate({ type: 'nodebuffer', compression: 'DEFLATE' });

    // Performs the upload.

    await client.api(`${baseUrl}/drive/root:/${tmpDocx}:/content`).put(buf);
  }

  public async createDocument({ pid, packagePath, content }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    this.internalCreateDocument(min, packagePath, content);
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
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `BEGINING COPY '${src}' to '${dest}'`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const botId = min.instance.botId;

    // Normalizes all slashes.

    src = src.replace(/\\/gi, '/');
    dest = dest.replace(/\\/gi, '/');

    // Determines full path at source and destination.

    const root = GBUtil.getGBAIPath(botId, 'gbdrive');
    const srcPath = urlJoin(root, src);
    const dstPath = urlJoin(root, dest);

    // Checks if the destination contains subfolders that
    // need to be created.

    let folder;
    if (dest.indexOf('/') !== -1) {
      const pathOnly = path.dirname(dest);
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
        name: `${path.basename(dest)}`
      };
      const file = await client.api(`${baseUrl}/drive/items/${srcFile.id}/copy`).post(destFile);
      GBLogEx.info(min, `FINISHED COPY '${src}' to '${dest}'`);
      return file;
    } catch (error) {
      if (error.code === 'itemNotFound') {
        GBLogEx.info(min, `COPY source file not found: ${srcPath}.`);
      } else if (error.code === 'nameAlreadyExists') {
        GBLogEx.info(min, `COPY destination file already exists: ${dstPath}.`);
      }
      throw error;
    }
  }

  /**
   * Converts a drive file from a place to another .
   *
   * Supported sources ai, csv, doc, docx, odp, ods, odt, pot, potm, potx, pps,
   * ppsx, ppsxm, ppt, pptm, pptx, rtf, xls, xlsx
   *
   * @example
   *
   * CONVERT "customers.xlsx" TO "reports\" + today + ".pdf"
   *
   */
  public async convert({ pid, src, dest }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `CONVERT '${src}' to '${dest}'`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const botId = min.instance.botId;

    // Normalizes all slashes.

    src = src.replace(/\\/gi, '/');
    dest = dest.replace(/\\/gi, '/');

    // Determines full path at source and destination.
    const packagePath = GBUtil.getGBAIPath(min.botId, `gbdrive`);
    const root = packagePath;
    const srcPath = urlJoin(root, src);
    const dstPath = urlJoin(packagePath, dest);


    if (path.extname(srcPath) === 'ai') {

      // TODO: To be done.

    } else {

      // Checks if the destination contains subfolders that
      // need to be created.

      let folder;
      if (dest.indexOf('/') !== -1) {
        const pathOnly = path.dirname(dest);
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
          GBLogEx.info(min, `CONVERT source file not found: ${srcPath}.`);
        } else if (error.code === 'nameAlreadyExists') {
          GBLogEx.info(min, `CONVERT destination file already exists: ${dstPath}.`);
        }
        throw error;
      }
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

  private flattenJSON(obj, res = {}, separator = '_', parent = null) {
    for (let key in obj) {
      if (!obj.hasOwnProperty(key) || typeof obj[key] === 'function') {
        continue;
      }
      if (typeof obj[key] !== 'object' || obj[key] instanceof Date) {
        // If not defined already, add the flattened field.
        const newKey = `${parent ? parent + separator : ''}${key}`;
        if (!res.hasOwnProperty(newKey)) {
          res[newKey] = obj[key];
        } else {
          GBLog.verbose(`Ignoring duplicated field in flatten operation to storage: ${key}.`);
        }
      } else {
        // Create a temporary reference to the nested object to prevent memory leaks.
        const tempObj = obj[key];
        this.flattenJSON(tempObj, res, separator, `${parent ? parent + separator : ''}${key}`);
        // Clear the reference to avoid holding unnecessary objects in memory.
        obj[key] = null;
      }
    }
    return res;
  }

  public async getCustomToken({ pid, tokenName }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `BASIC internal getCustomToken: ${tokenName}`);

    const token = await (min.adminService as any)['acquireElevatedToken'](
      min.instance.instanceId,
      false,
      tokenName,
      min.core.getParam(min.instance, `${tokenName} Client ID`, null),
      min.core.getParam(min.instance, `${tokenName} Client Secret`, null),
      min.core.getParam(min.instance, `${tokenName} Host`, null),
      min.core.getParam(min.instance, `${tokenName} Tenant`, null)
    );
    const expiresOn = await min.adminService.getValue(min.instance.instanceId, `${tokenName}expiresOn`);

    return { token, expiresOn };
  }

  /**
   * Calls any REST API by using GET HTTP method.
   *
   * @example user = get "http://server/users/1"
   *
   */
  public async getByHttp({ pid, url, headers, username, ps, qs }) {
    let options = {};

    const { min, user, params, proc } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `GET: ${url}`);

    let pageMode = await DialogKeywords.getOption({ pid, name: 'pageMode' });
    let continuationToken = await DialogKeywords.getOption({ pid, name: `${proc.executable}-continuationToken` });

    if (pageMode === 'auto' && continuationToken) {
      headers = headers ? headers : {};

      headers['MS-ContinuationToken'] = continuationToken;
    }

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
    let result;
    await retry(
      async bail => {
        result = await fetch(url, options);

        if (result.status === 401) {
          GBLogEx.info(min, `Waiting 5 secs. before retrying HTTP 401 GET: ${url}`);
          await GBUtil.sleep(5 * 1000);
          throw new Error(`HTTP:${result.status} retry: ${result.statusText}.`);
        }
        if (result.status === 429) {
          GBLogEx.info(min, `Waiting 1min. before retrying HTTP 429 GET: ${url}`);
          await GBUtil.sleep(60 * 1000);
          throw new Error(`HTTP:${result.status} retry: ${result.statusText}.`);
        }
        if (result.status === 503) {
          GBLogEx.info(min, `Waiting 1h before retrying GET 503: ${url}`);
          await GBUtil.sleep(60 * 60 * 1000);
          throw new Error(`HTTP:${result.status} retry: ${result.statusText}.`);
        }

        if (result.status === 2000) {
          // Token expired.

          await DialogKeywords.setOption({ pid, name: `${proc.executable}-continuationToken`, value: null });
          bail(new Error(`Expired Token for ${url}.`));
        }
        if (result.status != 200) {
          throw new Error(`GET ${result.status}: ${result.statusText}.`);
        }
      },
      {
        retries: 5,
        onRetry: error => {
          GBLog.error(`Retrying HTTP GET due to: ${error.message}.`);
        }
      }
    );
    let res = JSON.parse(await result.text());

    function process(key, value, o) {
      if (value === '0000-00-00') {
        o[key] = null;
      }
    }

    function traverse(o, func) {
      for (var i in o) {
        func.apply(this, [i, o[i], o]);
        if (o[i] !== null && typeof o[i] == 'object') {
          traverse(o[i], func);
        }
      }
    }

    traverse(res, process);

    if (pageMode === 'auto') {
      continuationToken = res.next?.headers['MS-ContinuationToken'];

      if (continuationToken) {
        GBLogEx.info(min, `Updating continuationToken for ${url}.`);
        await DialogKeywords.setOption({ pid, name: 'continuationToken', value: continuationToken });
      }
    } else {
      pageMode = 'none';
    }

    if (res) {
      res['pageMode'] = pageMode;
    }
    result = null;
    return res;
  }

  /**
   * Calls any REST API by using POST HTTP method.
   *
   * @example
   *
   * user = put "http://server/path", "data"
   * talk "The updated user area is" + area
   *
   */
  public async putByHttp({ pid, url, data, headers }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const options = {
      json: data,
      headers: headers,
      method: 'PUT'
    };

    if (typeof data === 'object') {
      options['body'] = JSON.stringify(data);
      options.headers['Content-Type'] = 'application/json';
    } else {
      options['body'] = data;
    }

    let result = await fetch(url, options);
    const text = await result.text();
    GBLogEx.info(min, `PUT ${url} (${data}): ${text}`);

    if (result.status != 200 && result.status != 201) {
      throw new Error(`PUT ${result.status}: ${result.statusText}.`);
    }

    let res = JSON.parse(text);
    return res;
  }

  /**
   * Calls any REST API by using POST HTTP method.
   *
   * @example
   *
   * user = post "http://server/path", "data"
   * talk "The updated user area is" + area
   *
   */
  public async postByHttp({ pid, url, data, headers }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const options = {
      headers: headers,
      method: 'POST'
    };

    if (typeof data === 'object') {
      options['body'] = JSON.stringify(data);
      options.headers['Content-Type'] = 'application/json';
    } else {
      options['body'] = data;
    }

    let result = await fetch(url, options);
    const text = await result.text();
    GBLogEx.info(min, `POST ${url} (${data}): ${text}`);

    if (result.status != 200 && result.status != 201) {
      throw new Error(`POST ${result.status}: ${result.statusText}.`);
    }

    let res = JSON.parse(text);
    return res;
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
    const botId = min.instance.botId;
    const gbaiName = GBUtil.getGBAIPath(botId, 'gbdata');
    let localName;

    // Downloads template from .gbdrive.

    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    let packagePath = '/' + urlJoin(gbaiName, `${botId}.gbdrive`);
    let template = await this.internalGetDocument(client, baseUrl, packagePath, templateName);
    let url = template['@microsoft.graph.downloadUrl'];
    const res = await fetch(url);
    let buf: any = Buffer.from(await res.arrayBuffer());
    localName = path.join('work', gbaiName, 'cache', `tmp${GBAdminService.getRndReadableIdentifier()}.docx`);
    await fs.writeFile(localName, new Uint8Array(buf), { encoding: null });

    // Replace image path on all elements of data.

    const images = [];
    let index = 0;
    packagePath = path.join(gbaiName, 'cache', `tmp${GBAdminService.getRndReadableIdentifier()}.docx`);
    url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));

    const traverseDataToInjectImageUrl = async o => {
      for (var i in o) {
        let value = o[i];

        if (value && value.gbarray) {
          o.shift();
          value = o[i];
        }

        for (const kind of ['png', 'jpg', 'jpeg']) {
          if (value.endsWith && value.endsWith(`.${kind}`)) {
            const { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

            packagePath = urlJoin(gbaiName, `${botId}.gbdrive`);
            if (value.indexOf('/') !== -1) {
              packagePath = '/' + urlJoin(packagePath, path.dirname(value));
              value = path.basename(value);
            }

            const ref = await this.internalGetDocument(client, baseUrl, packagePath, value);
            let url = ref['@microsoft.graph.downloadUrl'];
            const imageName = path.join(
              'work',
              gbaiName,
              'cache',
              `tmp${GBAdminService.getRndReadableIdentifier()}-${value}.png`
            );
            const response = await fetch(url);
            const buf = Buffer.from(await response.arrayBuffer());
            await fs.writeFile(imageName, new Uint8Array(buf), { encoding: null });

            const getNormalSize = ({ width, height, orientation }) => {
              return (orientation || 0) >= 5 ? [height, width] : [width, height];
            };

            // TODO: sharp. const metadata = await sharp(buf).metadata();
            const size = getNormalSize({
              width: 400,
              height: 400,
              orientation: '0'
            });
            url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(imageName));
            images[index++] = { url: url, size: size, buf: buf };
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

    // Loads the file as binary content.

    let zip = new PizZip(buf);
    let doc = new Docxtemplater();
    doc.setOptions({ paragraphLoop: true, linebreaks: true });
    doc.loadZip(zip);
    if (localName.endsWith('.pptx')) {
      doc.attachModule(pptxTemplaterModule);
    }
    doc.attachModule(new ImageModule(opts));

    await traverseDataToInjectImageUrl(data);
    doc.setData(data).render();

    buf = doc.getZip().generate({ type: 'nodebuffer', compression: 'DEFLATE' });
    await fs.writeFile(localName, new Uint8Array(buf), { encoding: null });

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

  private getTableFromName(file, min) {
    const minBoot = GBServer.globals.minBoot;
    const parts = file.split('.');
    const con = min[parts[0]];
    if (con) {
      return con.models[parts[1]];
    } else {
      return minBoot.core.sequelize.models[file];
    }
  }

  private cachedMerge: any = {};

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
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    if (!data || data.length === 0) {
      GBLog.verbose(`MERGE running on ${file}: NO DATA.`);
      return data;
    }

    GBLogEx.info(min, `MERGE running on ${file} and key1: ${key1}, key2: ${key2}...`);
    if (!this.cachedMerge[pid]) {
      this.cachedMerge[pid] = { file: {} };
    }

    // MAX LINES property.

    let maxLines = 1000;
    if (user && params && params.maxLines) {
      if (params.maxLines.toString().toLowerCase() !== 'default') {
        maxLines = Number.parseInt(params.maxLines).valueOf();
      }
    }

    // Choose data sources based on file type (HTML Table, data variable or sheet file)

    let storage = file.indexOf('.xlsx') === -1;
    let results;
    let header = [],
      rows = [];
    let t;
    let fieldsNames = [];
    let fieldsSizes = [];
    let fieldsValuesList = [];

    if (storage) {
      t = this.getTableFromName(file, min);

      if (!t) {
        throw new Error(`TABLE ${file} not found. Check TABLE keywords.`);
      }

      Object.keys(t.fieldRawAttributesMap).forEach(e => {
        fieldsNames.push(e);
      });

      Object.keys(t.fieldRawAttributesMap).forEach(e => {
        fieldsSizes.push(t.fieldRawAttributesMap[e].size);
      });

      header = Object.keys(t.fieldRawAttributesMap);

      // In a single execution, several MERGE calls will benift
      // from caching results across calls.

      if (!this.cachedMerge[pid][file]) {
        await retry(
          async bail => {
            rows = await t.findAll();
            GBLogEx.info(min, `MERGE cached: ${rows.length} row(s)...`);
          },
          {
            retries: 5,
            onRetry: error => {
              GBLog.error(`MERGE: Retrying SELECT ALL on table: ${error.message}.`);
            }
          }
        );
      } else {
        rows = this.cachedMerge[pid][file];
      }
    } else {
      const botId = min.instance.botId;
      const packagePath = GBUtil.getGBAIPath(botId, 'gbdata');

      let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

      let document;
      document = await this.internalGetDocument(client, baseUrl, packagePath, file);

      // Creates workbook session that will be discarded.

      let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();

      results = await client
        .api(
          `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A1:CZ${maxLines}')`
        )
        .get();

      header = results.text[0];
      rows = results.text;
      results = null;
    }

    let table = [];
    let foundIndex = 0;

    // Fills the row variable on the base dataset.

    if (!storage || !this.cachedMerge[pid][file]) {
      for (; foundIndex < rows.length; foundIndex++) {
        let row = {};
        let tmpRow = rows[foundIndex];
        row = tmpRow.dataValues ? tmpRow.dataValues : tmpRow;

        for (let colIndex = 0; colIndex < tmpRow.length; colIndex++) {
          const propertyName = header[colIndex];
          let value = tmpRow[colIndex];

          if (value && typeof value === 'string' && value.charAt(0) === "'") {
            if (await this.isValidDate({ pid, dt: value.substr(1) })) {
              value = value.substr(1);
            }
          }

          row[propertyName] = value;
          value = null;
        }
        row['line'] = foundIndex + 1;
        table.push(row);
        row = null;
        tmpRow = null;
      }

      if (storage) {
        this.cachedMerge[pid][file] = table;
      }
    } else {
      table = this.cachedMerge[pid][file];
    }

    let key1Index, key2Index;

    if (key1) {
      key1Index = _.invertBy(table, key1);
    }

    if (key2) {
      key2Index = _.invertBy(table, key2);
    }

    let updates = 0,
      adds = 0,
      skipped = 0;

    // Scans all items in incoming data.

    for (let i = 0; i < data.length; i++) {
      // Scans all sheet lines and compare keys.

      let row = data[i];

      if (GBUtil.hasSubObject(row)) {
        row = this.flattenJSON(row);
      }

      let found;
      let key1Value;
      let key1Original = key1;
      if (key1Index) {
        key1 = key1.charAt(0).toLowerCase() + key1.slice(1);

        Object.keys(row).forEach(e => {
          if (e.toLowerCase() === key1.toLowerCase()) {
            key1Value = row[e];
          }
        });

        let foundRow = key1Index[key1Value];
        if (foundRow) {
          found = table[foundRow[0]];
        }
        foundRow = null;
      }

      if (found) {
        let merge = false;
        for (let j = 0; j < header.length; j++) {
          const columnName = header[j];
          let columnNameFound = false;

          let value;
          Object.keys(row).forEach(e => {
            if (columnName.toLowerCase() === e.toLowerCase()) {
              value = row[e];
              if (typeof value === 'string') {
                value = value.substring(0, fieldsSizes[j]);
              }

              columnNameFound = true;
            }
          });

          if (value === undefined) {
            value = null;
          }

          let valueFound;
          Object.keys(found).forEach(e => {
            if (columnName.toLowerCase() === e.toLowerCase()) {
              valueFound = found[e];
            }
          });

          const equals =
            typeof value === 'string' && typeof valueFound === 'string'
              ? value?.toLowerCase() != valueFound?.toLowerCase()
              : value != valueFound;

          if (equals && columnNameFound) {
            if (storage) {
              let obj = {};
              obj[columnName] = value;
              let criteria = {};
              criteria[key1Original] = key1Value;

              await retry(
                async bail => {
                  await t.update(obj, { where: criteria });
                },
                { retries: 5 }
              );
              obj = null;
            } else {
              const cell = `${this.numberToLetters(j)}${i + 1}`;
              const address = `${cell}:${cell}`;

              await this.set({ pid, handle: null, file, address, value });
            }
            merge = true;
          }
        }

        merge ? updates++ : skipped++;
      } else {
        let fieldsValues = [];

        for (let j = 0; j < fieldsNames.length; j++) {
          let add = false;
          Object.keys(row).forEach(p => {
            if (fieldsNames[j].toLowerCase() === p.toLowerCase()) {
              let value = row[p];
              if (typeof value === 'string') {
                value = value.substring(0, fieldsSizes[j]);
              }

              fieldsValues.push(value);
              add = true;
            }
          });
          if (!add) {
            fieldsValues.push(null);
          }
        }

        if (storage) {
          // Uppercases fields.

          let dst = {};
          let i = 0;
          Object.keys(fieldsValues).forEach(fieldSrc => {
            const name = fieldsNames[i];
            const field = name.charAt(0).toUpperCase() + name.slice(1);
            dst[field] = fieldsValues[fieldSrc];
            i++;
          });

          fieldsValuesList.push(dst);
          this.cachedMerge[pid][file].push(dst);
          dst = null;
        } else {
          await this.save({ pid, file, args: fieldsValues });
        }
        fieldsValues = null;
        adds++;
      }
      row = null;
      found = null;
    }

    // In case of storage, persist to DB in batch.

    if (fieldsValuesList.length) {
      await this.saveToStorageBatch({ pid, table: file, rows: fieldsValuesList });
    }
    key1Index = null;
    key2Index = null;

    table = null;
    fieldsValuesList = null;
    rows = null;
    header = null;
    results = null;
    t = null;

    GBLogEx.info(min, `MERGE results: adds:${adds}, updates:${updates} , skipped: ${skipped}.`);
    return { title: file, adds, updates, skipped };
  }

  /**
   * Publishs a post to BlueSky .
   *
   * BlueSky "My BlueSky text"
   */
  public async postToBlueSky({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    const consumer_key = min.core.getParam(min.instance, 'BlueSky Consumer Key', null);
    const consumer_secret = min.core.getParam(min.instance, 'BlueSky Consumer Key Secret', null);
    const access_token_key = min.core.getParam(min.instance, 'BlueSky Access Token', null);
    const access_token_secret = min.core.getParam(min.instance, 'BlueSky Access Token Secret', null);

    if (!consumer_key || !consumer_secret || !access_token_key || !access_token_secret) {
      GBLogEx.info(min, 'BlueSky not configured in .gbot.');
    }
    throw new Error('Not implemented yet.');

    GBLogEx.info(min, `BlueSky Automation: ${text}.`);
  }


  /**
   */
  public async answer({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const answer = await ChatServices.answerByLLM(pid, min, user, text)
    GBLogEx.info(min, `ANSWER ${text} TO ${answer}`);
    return answer.answer;
  }

  /**
   * HEAR description
   * text = REWRITE description
   * SAVE "logs.xlsx", username, text
   */
  public async rewrite({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const prompt = `Rewrite this sentence in a better way: ${text}`;
    const answer = await ChatServices.invokeLLM(min, prompt);
    GBLogEx.info(min, `REWRITE ${text} TO ${answer}`);
    return answer;
  }

  /**
   *
   * qrcode = PAY "10000", "Name", 100
   * SEND FILE qrcode
   *
   */
  public async pay({ pid, orderId, customerName, ammount }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    const gbaiName = GBUtil.getGBAIPath(min.botId);

    const merchantId = min.core.getParam(min.instance, 'Merchant ID', null);
    const merchantKey = min.core.getParam(min.instance, 'Merchant Key', null);

    if (!merchantId || !merchantKey) {
      throw new Error('Payment not configured in .gbot.');
    }

    const apiUrl = 'https://apisandbox.cieloecommerce.cielo.com.br/1/sales/';
    const requestId = GBAdminService.generateUuid();

    GBLogEx.info(min, `GBPay: ${requestId}, ${orderId}, ${ammount}... `);

    const requestData = {
      MerchantOrderId: orderId,
      Customer: {
        Name: customerName
      },
      Payment: {
        Type: 'qrcode',
        Amount: ammount,
        Installments: 1,
        Capture: false,
        Modality: 'Debit'
      }
    };

    const response = await fetch(apiUrl, {
      method: 'POST',
      body: JSON.stringify(requestData),
      headers: {
        'Content-Type': 'application/json',
        MerchantId: merchantId,
        MerchantKey: merchantKey,
        RequestId: requestId
      }
    });

    if (!response.ok) {
      throw new Error(`HTTP error! Status: ${response.status}`);
    }
    const data = await response.json();

    // Prepare an image on cache and return the GBFILE information.

    const buf = Buffer.from(data.Payment.QrCodeBase64Image, 'base64');
    const localName = path.join('work', gbaiName, 'cache', `qr${GBAdminService.getRndReadableIdentifier()}.png`);
    await fs.writeFile(localName, new Uint8Array(buf), { encoding: null });
    const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));

    GBLogEx.info(min, `GBPay: ${data.MerchantOrderId} OK: ${url}.`);

    return {
      name: path.basename(localName),
      localName: localName,
      url: url,
      data: buf,
      text: data.Payment.QrCodeString
    };
  }

  /**
   * HEAR logo AS FILE
   * file = AUTO SAVE logo
   * TALK "Your " + file.name + " file is saved."
   */
  public async autoSave({ pid, handle }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    this.internalAutoSave({ min, handle });
  }

  private async internalAutoSave({ min, handle }) {
    const file = GBServer.globals.files[handle];
    GBLogEx.info(min, `Auto saving '${file.filename}' (SAVE file).`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

    const packagePath = GBUtil.getGBAIPath(min.botId, `gbdrive`);
    const fileName = file.url ? file.url : file.name;
    const contentType = mime.lookup(fileName);
    const ext = path.extname(fileName).substring(1);
    const kind = await this.getExtensionInfo(ext);

    let d = new Date(),
      month = '' + (d.getMonth() + 1),
      day = '' + d.getDate(),
      year = d.getFullYear();

    const today = [day, month, year].join('-');
    const result = await client
      .api(`${baseUrl}/drive/root:/${packagePath}/${today}/${kind.category}/${fileName}:/content`)
      .put(file.data);

    return { contentType, ext, kind, category: kind['category'] };
  }

  public async deleteFromStorage({ pid, table, criteria }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `DELETE '${table}' where ${criteria}.`);

    const definition = this.getTableFromName(table, min);
    const filter = await SystemKeywords.getFilter(criteria);

    await retry(
      async bail => {
        const options = { where: {} };
        options.where = {};

        options.where[filter['columnName']] = filter['value'];
        await definition.destroy(options);
      },
      {
        retries: 5,
        onRetry: error => {
          GBLog.error(`Retrying deleteFromStorage due to: ${error.message}.`);
        }
      }
    );
  }

  public async deleteFile({ pid, file }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `DELETE '${file.name}'.`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

    const gbaiPath = GBUtil.getGBAIPath(min.botId);
    const fileName = file.name;
    const contentType = mime.lookup(fileName);
    const ext = path.extname(fileName).substring(1);
    const kind = await this.getExtensionInfo(ext);

    await client.api(`${baseUrl}/drive/root:/${gbaiPath}/${file.path}`).delete();

    return { contentType, ext, kind, category: kind['category'] };
  }

  public async getExtensionInfo(ext: any): Promise<any> {

    // TODO: Load exts.

    let array = []; // exts.filter((v, i, a) => a[i]['extension'] === ext);
    if (array[0]) {
      return array[0];
    }
    return { category: 'Other', description: 'General documents' };
  }

  /**
   * Loads all para from tabular file Config.xlsx.
   */
  public async dirFolder({ pid, remotePath, baseUrl = null, client = null, array = null }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `dirFolder: remotePath=${remotePath}, baseUrl=${baseUrl}`);

    // In case of empty files, build an zero element array.

    if (!array) {
      array = [];
    }

    if (!baseUrl) {
      let obj = await GBDeployer.internalGetDriveClient(min);
      baseUrl = obj.baseUrl;
      client = obj.client;
    }

    remotePath = remotePath.replace(/\\/gi, '/');

    // Retrieves all files in remote folder.

    let packagePath = GBUtil.getGBAIPath(min.botId);
    packagePath = urlJoin(packagePath, remotePath);
    let url = `${baseUrl}/drive/root:/${packagePath}:/children`;

    const res = await client.api(url).get();
    const documents = res.value;
    if (documents === undefined || documents.length === 0) {
      GBLogEx.info(min, `${remotePath} is an empty folder.`);
      return array;
    }

    // Navigate files / directory to recurse.

    await CollectionUtil.asyncForEach(documents, async item => {
      if (item.folder) {
        remotePath = urlJoin(remotePath, item.name);
        array = [...array, ...(await this.dirFolder({ pid, remotePath, baseUrl, client, array }))];
      } else {
        // TODO:  https://raw.githubusercontent.com/ishanarora04/quickxorhash/master/quickxorhash.js

        let obj = {};
        obj['modified'] = item.lastModifiedDateTime;
        obj['name'] = item.name;
        obj['size'] = item.size;
        obj['hash'] = item.file?.hashes?.quickXorHash;
        obj['path'] = path.join(remotePath, item.name);
        obj['url'] = item['@microsoft.graph.downloadUrl'];

        array.push(obj);
      }
    });

    return array;
  }

  public async log({ pid, obj }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, GBUtil.toYAML(obj));
  }

  public async getPdf({ pid, file }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `BASIC GET (pdf): ${file}`);
    try {
      let data;

      if (GBConfigService.get('GB_MODE') === 'legacy') {
        let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
        const gbaiName = GBUtil.getGBAIPath(min.botId);
        let packagePath = '/' + urlJoin(gbaiName, `${min.botId}.gbdrive`);
        let template = await this.internalGetDocument(client, baseUrl, packagePath, file);
        let url = template['@microsoft.graph.downloadUrl'];
        const res = await fetch(url);
        let buf: any = Buffer.from(await res.arrayBuffer());
        data = new Uint8Array(buf);
      } else {
        let packagePath = GBUtil.getGBAIPath(min.botId, `gbdrive`);
        let filePath = path.join(GBConfigService.get('STORAGE_LIBRARY'), packagePath, file);
        data = await fs.readFile(filePath);
        data = new Uint8Array(data);
      }
      return await GBUtil.getPdfText(data);
    } catch (error) {
      GBLogEx.error(min, error);
      return null;
    }
  }

  public async setContext({ pid, text }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    ChatServices.userSystemPrompt[user.userSystemId] = text;

    await this.setMemoryContext({ pid, erase: true });
  }

  public async setMemoryContext({ pid, input = null, output = null, erase }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    let memory;
    if (erase || !ChatServices.memoryMap[user.userSystemId]) {
      memory = new BufferWindowMemory({
        returnMessages: true,
        memoryKey: 'chat_history',
        inputKey: 'input',
        k: 2
      });

      ChatServices.memoryMap[user.userSystemId] = memory;
    } else {
      memory = ChatServices.memoryMap[user.userSystemId];
    }

    if (memory && input)
      await memory.saveContext(
        {
          input: input
        },
        {
          output: output
        }
      );
  }

  public async postToFacebook({ pid, imagePath, caption, pageId }) {
    // Obtendo informações do processo para logs (ajuste conforme necessário)
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);

    // Leitura do arquivo de imagem
    const imageBuffer = await fs.readFile(path.resolve(imagePath));

    // Criação de um arquivo temporário para enviar
    const tempFilePath = path.resolve('temp_image.jpg');
    await fs.writeFile(tempFilePath, new Uint8Array(imageBuffer));

    // Publicação da imagem
    const page = new Page(pageId);
    const response = await page.createFeed({
      message: caption,
      attached_media: [
        {
          media_fbid: tempFilePath
        }
      ]
    });

    // Log do resultado
    GBLogEx.info(min, `Imagem publicada no Facebook: ${JSON.stringify(response)}`);

    // Limpeza do arquivo temporário
    fs.unlink(tempFilePath);
  }

  public async postToInstagram({ pid, username, password, imagePath, caption }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);

    const ig = new IgApiClient();
    ig.state.generateDevice(username);
    await ig.account.login(username, password);
    const imageBuffer = await fs.readFile(imagePath);
    const publishResult = await ig.publish.photo({ file: imageBuffer, caption });

    GBLogEx.info(min, `Image posted on IG: ${publishResult}`);
  }

  public async setAnswerMode({ pid, mode }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);

    ChatServices.usersMode[user.userSystemId] = mode;

    GBLogEx.info(min, `LLM Mode (${user.userSystemId}): ${mode}`);
  }

  /**
   * Saves variables to storage, not a worksheet.
   *
   * @example SAVE "Billing",  columnName1, columnName2
   *
   */
  public async saveToStorage({ pid, table, fieldsValues, fieldsNames }): Promise<any> {
    if (!fieldsValues || fieldsValues.length === 0 || !fieldsValues[0]) {
      return;
    }

    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `SAVE '${table}': 1 row.`);

    // Uppercase fields
    const dst = {};
    fieldsNames.forEach((fieldName, index) => {
      const field = fieldName.charAt(0).toUpperCase() + fieldName.slice(1);
      dst[field] = fieldsValues[Object.keys(fieldsValues)[index]];
    });

    let item;
    await retry(
      async bail => {
        if (table.endsWith('.csv')) {
          // CSV handling
          const packagePath = GBUtil.getGBAIPath(min.botId, 'gbdata');
          const csvFile = path.join(GBConfigService.get('STORAGE_LIBRARY'), packagePath, `${table}`);

          try {
            // Try to read the file to get headers
            const data = await fs.readFile(csvFile, 'utf8');
            const headers = data.split('\n')[0].split(',');
            const db = await csvdb(csvFile, headers, ',');

            // Append new row
            await db.add(dst);
            item = dst;
          } catch (error) {
            if (error.code === 'ENOENT') {
              // File doesn't exist, create it with headers and data
              const headers = Object.keys(dst);
              await fs.writeFile(csvFile, headers.join(',') + '\n');
              const db = await csvdb(csvFile, headers, ',');
              await db.add(dst);
              item = dst;
            } else {
              throw error;
            }
          }
        } else {
          const definition = this.getTableFromName(table, min);
          item = await definition.create(dst);
        }
      },
      {
        retries: 5,
        onRetry: error => {
          GBLog.error(`Retrying SaveToStorage due to: ${error.message}.`);
        }
      }
    );
    return item;
  }

  public async showImage({ pid, file }) {
    const { min, user, params, step } = await DialogKeywords.getProcessInfo(pid);

    const url = file?.url ? file.url : file;
    GBLog.info(`PLAY IMAGE: ${url}.`);

    await min.kbService.showImage(min, min.conversationalService, step, url);

    await this.setMemoryContext({ pid, erase: true });
  }

  public async convertAI2HTML(aiFilePath) {

    // Convert the AI file to HTML and assets
    const result = await ai2html.convertFile(aiFilePath, {
      outputFormat: 'html',
      outputWriteMethod: 'write-file',
      outputPath: path.dirname(aiFilePath),
      useDocumentSettings: true,
    });

    // Get the generated HTML file path
    const htmlFilePath = result.outputFiles.find((file) => file.endsWith('.html')).filePath;

    // Read the HTML content
    const htmlContent = await fs.readFile(htmlFilePath, 'utf8');

    // Save the HTML and assets to a cache directory
    const cacheDir = path.join('work', 'cache');
    await fs.mkdir(cacheDir, { recursive: true });
    const cacheFilePath = path.join(cacheDir, path.basename(htmlFilePath));
    await fs.writeFile(cacheFilePath, htmlContent);

    return cacheFilePath;

  }


  public async refreshDataSourceCache({ pid, connectionName }) {
    const { min, user, params, step } = await DialogKeywords.getProcessInfo(pid);

    let sqliteFilePath = path.join('work', GBUtil.getGBAIPath(min.botId), `${connectionName}.sqlite`);

    // Step 1: Clean the SQLite file if it already exists
    if (await GBUtil.exists(sqliteFilePath)) {
      await fs.unlink(sqliteFilePath); // Remove the file
      GBLogEx.info(min, `${sqliteFilePath} has been cleaned.`);
    }

    // Step 2: Connect to SQLite (Local)
    const sqlite = new Sequelize({
      dialect: 'sqlite',
      storage: sqliteFilePath
    });

    // Get the connection details from the min object
    let con = min[connectionName];
    const dialect = con.dialect.name;

    // Step 3: Get the list of all tables from the source database
    const tables = await GBUtil.listTables(dialect, con);

    // Function to map source database datatypes to SQLite-compatible datatypes
    const mapToSQLiteType = (columnType) => {
      const typeMapping = {
        'VARCHAR': DataTypes.STRING,
        'CHAR': DataTypes.STRING,
        'TEXT': DataTypes.TEXT,
        'TINYINT': DataTypes.INTEGER,
        'SMALLINT': DataTypes.INTEGER,
        'MEDIUMINT': DataTypes.INTEGER,
        'INT': DataTypes.INTEGER,
        'INTEGER': DataTypes.INTEGER,
        'BIGINT': DataTypes.BIGINT,
        'FLOAT': DataTypes.FLOAT,
        'DOUBLE': DataTypes.DOUBLE,
        'DECIMAL': DataTypes.DECIMAL,
        'DATE': DataTypes.DATE,
        'DATETIME': DataTypes.DATE,
        'TIMESTAMP': DataTypes.DATE,
        'BLOB': DataTypes.BLOB,
        'BOOLEAN': DataTypes.BOOLEAN,
        // Add more mappings as needed
      };

      return typeMapping[columnType.toUpperCase()] || DataTypes.STRING;
    };

    // Step 4: Retrieve and export data for each table
    for (const table of tables) {
      // Retrieve rows from the source table
      const [rows] = await con.query(`SELECT * FROM ${table}`);

      if (rows.length === 0) continue; // Skip if the table has no data

      // Get the schema for the current table from the source database
      const columns = await con.queryInterface.describeTable(table);

      // Create a schema object for SQLite
      const schema = {};
      let pkAdded = false;
      Object.keys(columns).forEach(col => {
        const columnType = columns[col].type;

        // Map source type to SQLite type
        schema[col] = {
          type: mapToSQLiteType(columnType)
        };

        // If the column is named 'id' or 'Id', set it as the primary key
        if (!pkAdded && (col.toLowerCase() === 'id' || col.toLowerCase() === 'internal_id')) {
          schema[col].primaryKey = true;
          pkAdded = true;
        }
      });

      // Define the model dynamically for each table in SQLite
      const Model = sqlite.define(table, schema, { timestamps: false });

      // Sync the model (create table)
      await Model.sync({ force: true });

      // Transform data to match schema types before bulk insert
      const transformedRows = rows.map(row => {
        const transformedRow = {};
        for (const key in row) {
          const columnType = schema[key].type;

          // Handle different data types
          if (columnType === DataTypes.STRING) {
            transformedRow[key] = row[key] !== null ? String(row[key]) : null; // Convert to string
          } else if (columnType === DataTypes.INTEGER || columnType === DataTypes.BIGINT) {
            transformedRow[key] = row[key] !== null ? Number(row[key]) : null; // Convert to number
          } else if (columnType === DataTypes.FLOAT || columnType === DataTypes.DOUBLE) {
            transformedRow[key] = row[key] !== null ? parseFloat(row[key]) : null; // Convert to float
          } else if (columnType === DataTypes.BOOLEAN) {
            transformedRow[key] = row[key] !== null ? Boolean(row[key]) : null; // Convert to boolean
          } else if (columnType === DataTypes.DATE) {
            transformedRow[key] = row[key] !== null ? new Date(row[key]) : null; // Convert to date
          } else {
            transformedRow[key] = row[key]; // Keep original value for unsupported types
          }
        }
        return transformedRow;
      });

      // Bulk insert rows into the SQLite table
      await Model.bulkCreate(transformedRows);
    }

    GBLogEx.info(min, `All tables have been successfully exported to ${sqliteFilePath}`);

    // Close SQLite connection
    await sqlite.close();
  }
}
