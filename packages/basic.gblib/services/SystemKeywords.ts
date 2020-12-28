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
import * as request from 'request-promise-native';
import urlJoin = require('url-join');
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { SecService } from '../../security.gbapp/services/SecService';
const request = require('request-promise-native');
const MicrosoftGraph = require('@microsoft/microsoft-graph-client');

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

  /**
   * When creating this keyword facade, a bot instance is
   * specified among the deployer service.
   */
  constructor(min: GBMinInstance, deployer: GBDeployer) {
    this.min = min;
    this.deployer = deployer;
  }

  /**
   * Retrives token and initialize drive client API.
   */
  private async internalGetDriveClient() {
    let token = await this.min.adminService.acquireElevatedToken(this.min.instance.instanceId);
    let siteId = process.env.STORAGE_SITE_ID;
    let libraryId = process.env.STORAGE_LIBRARY;

    let client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });
    const baseUrl = `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}`;
    return [client, baseUrl];
  }

  /**
   * Retrives the content of a given URL.
   */
  public async getFileContents(url) {
    const options = {
      url: url,
      method: 'GET',
      encoding: 'binary'
    };
    const res = await request(options);
    Buffer.from(res, 'binary').toString();
  }

  /**
   * Retrives a random id with a length of five, every time it is called.
   */
  public async getRandomId() {
    return GBAdminService.getRndReadableIdentifier().substr(5);
  }

  /**
   * Retrives stock inforation for a given symbol.
   */
  public async getStock(symbol) {
    var options = {
      uri: `http://live-nse.herokuapp.com/?symbol=${symbol}`
    };

    let data = await request.get(options);
    return data;
  }

  /**
   * Prepares the next dialog to be shown to the specified user.
   */
  public async gotoDialog(from: string, dialogName: string) {
    let sec = new SecService();
    let user = await sec.getUserFromSystemId(from);
    if (!user) {
      user = await sec.ensureUser(this.min.instance.instanceId, from, from, null, 'whatsapp', 'from');
    }
    await sec.updateUserHearOnDialog(user.userId, dialogName);
  }

  /**
   * Holds script execution for the number of seconds specified.
   * 
   * @example WAIT 5 ' This will wait five seconds.
   *  
   */
  public async wait(seconds: number) {
    // tslint:disable-next-line no-string-based-set-timeout
    GBLog.info(`BASIC: Talking to a specific user (TALK TO).`);
    const timeout = async (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
    await timeout(seconds * 1000);
  }

  /**
   * Sends a text message to the mobile number specified.
   * 
   * @example TALK TO "+199988887777", "Message text here"
   * 
   */
  public async talkTo(mobile: any, message: string) {
    GBLog.info(`BASIC: Talking '${message}' to a specific user (${mobile}) (TALK TO). `);
    await this.min.conversationalService.sendMarkdownToMobile(this.min, null, mobile, message);
  }

  /**
   * Sends a SMS message to the mobile number specified.
   * 
   * @example SEND SMS TO "+199988887777", "Message text here"
   * 
   */
  public async sendSmsTo(mobile, message) {
    await this.min.conversationalService.sendSms(this.min, mobile, message);
  }

  /**
   * Defines a cell value in the tabular file.
   * 
   * @example SET "file.xlsx", "A2", 4500
   * 
   */
  public async set(file: string, address: string, value: any): Promise<any> {
    GBLog.info(`BASIC: Defining '${address}' in '${file}' to '${value}' (SET). `);

    let [baseUrl, client] = await this.internalGetDriveClient();

    const botId = this.min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbdata`;

    address = address.indexOf(':') !== -1 ? address : address + ":" + address;

    let document = await this.internalGetDocument(client, baseUrl, path, file);

    let body = { values: [[]] };
    body.values[0][0] = value;

    let sheets = await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`)
      .get();

    await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${address}')`)
      .patch(body);
  }

  /**
   * Retrives a document from the drive, given a path and filename.
   */
  private async internalGetDocument(client: any, baseUrl: any, path: string, file: string) {
    let res = await client
      .api(`${baseUrl}/drive/root:${path}:/children`)
      .get();

    let documents = res.value.filter(m => {
      return m.name.toLowerCase() === file.toLowerCase();
    });

    if (!documents || documents.length === 0) {
      throw `File '${file}' specified on GBasic command not found. Check the .gbdata or the .gbdialog associated.`;
    }

    return documents[0];
  }

  /**
   * Saves the content of several variables to a new row in a tabular file.
   * 
   * @exaple SAVE "customers.xlsx", name, email, phone, address, city, state, country
   * 
   */
  public async save(file: string, ...args): Promise<any> {
    GBLog.info(`BASIC: Saving '${file}' (SAVE). Args: ${args.join(',')}.`);
    let [baseUrl, client] = await this.internalGetDriveClient();
    const botId = this.min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbdata`;

    let document = await this.internalGetDocument(client, baseUrl, path, file);
    let sheets = await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`)
      .get();

    await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A2:DX2')/insert`)
      .post({});

    if (args.length > 128) {
      throw `File '${file}' has a SAVE call with more than 128 arguments. Check the .gbdialog associated.`;
    }

    let body = { values: [[]] };
    for (let index = 0; index < 128; index++) {
      body.values[0][index] = args[index];
    }
    await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A2:DX2')`)
      .patch(body);
  }

  /**
   * Retrives the content of a cell in a tabular file.
   * 
   * @example value = GET "file.xlsx", "A2"
   * 
   */
  public async get(file: string, address: string): Promise<any> {
    let [baseUrl, client] = await this.internalGetDriveClient();
    const botId = this.min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbdata`;

    let document = await this.internalGetDocument(client, baseUrl, path, file);

    // Creates workbook session that will be discarded.
    let sheets = await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`)
      .get();

    let results = await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${address}')`)
      .get();

    let val = results.text[0][0];
    GBLog.info(`BASIC: Getting '${file}' (GET). Value= ${val}.`);
    return val;
  }

  /**
   * Finds a value or multi-value results in a tabular file.
   * 
   * @example 
   * 
   *  rows = FIND "file.xlsx", "A2=active"
   *  i = 1
   *  do while i < ubound(row)
   *    row = rows[i]
   *    send sms to "+" + row.mobile, "Hello " + row.namee + "! "
   *  loop
   * 
   */
  public async find(file: string, ...args): Promise<any> {
    let [baseUrl, client] = await this.internalGetDriveClient();
    const botId = this.min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbdata`;

    let document = await this.internalGetDocument(client, baseUrl, path, file);

    if (args.length > 1) {
      throw `File '${file}' has a FIND call with more than 1 arguments. Check the .gbdialog associated.`;
    }

    // Creates workbook session that will be discarded.
    const filter = args[0].split('=');
    const columnName = filter[0];
    const value = filter[1];
    let sheets = await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`)
      .get();

    let results = await client
      .api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A1:Z100')`)
      .get();

    let columnIndex = 0;
    const header = results.text[0];
    for (; columnIndex < header.length; columnIndex++) {
      if (header[columnIndex].toLowerCase() === columnName.toLowerCase()) {
        break;
      }
    }

    // As BASIC uses arrays starting with 1 (one) as index, 
    // a ghost element is added at 0 (zero) position.
    let array = [];
    array.push({ 'this is a base 1': 'array' });
    let foundIndex = 0;
    for (; foundIndex < results.text.length; foundIndex++) {
      // Filter results action.
      if (results.text[foundIndex][columnIndex].toLowerCase() === value.toLowerCase()) {
        let output = {};
        const row = results.text[foundIndex];
        for (let colIndex = 0; colIndex < row.length; colIndex++) {
          output[header[colIndex]] = row[colIndex];
        }
        output['line'] = foundIndex + 1;
        array.push(output);
      }
    }

    if (array.length === 1) {
      GBLog.info(`BASIC: FIND the data set is EMPTY (zero results).`);
      return null;
    } else if (array.length === 2) {
      GBLog.info(`BASIC: FIND single result: ${array[0]}.`);
      return array[1];
    } else {
      GBLog.info(`BASIC: FIND multiple result count: ${array.length}.`);
      return array;
    }
  }

  /**
   * Creates a folder in the bot instance drive.
   *
   * @example folder = CREATE FOLDER "notes\01"
   *
   */
  public async createFolder(name: string) {

    let [baseUrl, client] = await this.internalGetDriveClient();
    const botId = this.min.instance.botId;
    const path = urlJoin(`/${botId}.gbai/${botId}.gbdata`, name);

    return new Promise<any>((resolve, reject) => {
      const body = {
        "name": name,
        "folder": {},
        "@microsoft.graph.conflictBehavior": "rename"
      };
      client
        .api(`${baseUrl}/drive/root:/${path}:/children`)
        .post(body, (err, res) => {
          if (err) {
            reject(err);
          }
          else {
            resolve(res);
          }
        });
    });
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
  public async shareFolder(folderReference, email: string, message: string) {
    let [, client] = await this.internalGetDriveClient();
    const driveId = folderReference.parentReference.driveId;
    const itemId = folderReference.id;

    return new Promise<string>((resolve, reject) => {

      const body = {
        "recipients": [
          {
            "email": email
          }
        ],
        "message": message,
        "requireSignIn": true,
        "sendInvitation": true,
        "roles": ["write"]
      };

      client
        .api(`https://graph.microsoft.com/v1.0/drives/${driveId}/items/${itemId}/invite`)
        .post(body, (err, res) => {
          if (err) {
            reject(err);
          }
          else {
            resolve(res);
          }
        });
    });
  }

  /**
   * Copies a drive file from a place to another .
   * 
   * @example
   * 
   * COPY "template.xlsx", "reports\" + customerName + "\final.xlsx"
   * 
   */
  public async copyFile() {
    let [] = await this.internalGetDriveClient();

    // const botId = this.min.instance.botId;
    // const path = urlJoin(`/${botId}.gbai/${botId}.gbdata`, name);
    // const body =
    // {
    //   "parentReference": { driveId: gbaiDest.parentReference.driveId, id: gbaiDest.id },
    //   "name": `${botName}.${kind}`
    // }
    // const packageName = `${templateName.split('.')[0]}.${kind}`;
    // try {
    //   const src = await client.api(    //     `${baseUrl}/drive/root:/${source}`)
    //     .get();
    //   return await client.api(    //     `${baseUrl}/drive/items/${src.id}/copy`)
    //     .post(body);
    // } catch (error) {
    //   if (error.code === "itemNotFound") {
    //   } else if (error.code === "nameAlreadyExists") {
    //     let src = await client.api(    //       `${baseUrl}/drive/root:/${templateName}/${packageName}:/children`)
    //       .get();
    //     const dstName = `${botName}.gbai/${botName}.${kind}`;
    //     let dst = await client.api(`${baseUrl}/drive/root:/${dstName}`)
    //       .get();
    //     await CollectionUtil.asyncForEach(src.value, async item => {
    //       const body =
    //       {
    //         "parentReference": { driveId: dst.parentReference.driveId, id: dst.id }
    //       }
    //       await client.api(    //         `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateId}/drive/items/${item.id}/copy`)
    //         .post(body);
    //     });
    //   }
    //   else {
    //     GBLog.error(error);
    //     throw error;
    //   }
    // }
  }

  /** 
   * Generate a secure and unique password.
   * 
   * @example pass = PASSWORD
   * 
   */
  public generatePassword() {
    return GBAdminService.getRndPassword();
  }

  /**
   * Sends an e-mail.
   * 
   * @example 
   */
  public async sendEmail(to, subject, body) {
    // tslint:disable-next-line:no-console
    GBLog.info(`[E-mail]: to:${to}, subject: ${subject}, body: ${body}.`);
  }

  /**
   * Calls any REST API by using GET HTTP method.
   * 
   * @example user = get "http://server/users/1"
   * 
   */
  public async getByHttp(url: string) {
    const options = {
      uri: url
    };

    let result = await request.get(options);
    GBLog.info(`[GET]: ${url} : ${result}`);
    return JSON.parse(result);
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
  public async postByHttp(url: string, data) {
    const options = {
      uri: url,
      json: true,
      body: data
    };

    let result = await request.post(options);
    GBLog.info(`[POST]: ${url} (${data}): ${result}`);
    return JSON.parse(result);
  }

  public async numberOnly(text: string) {
    return text.replace(/\D/gi, '');
  }
}
