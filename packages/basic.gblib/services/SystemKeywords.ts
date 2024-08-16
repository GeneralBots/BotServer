/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/
'use strict';

import { IgApiClient } from 'instagram-private-api';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { getDocument } from 'pdfjs-dist/legacy/build/pdf.mjs';
import { GBLog, GBMinInstance } from 'botlib';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { DialogKeywords } from './DialogKeywords.js';
import { GBServer } from '../../../src/app.js';
import { GBVMService } from './GBVMService.js';
import Fs from 'fs';
import { GBSSR } from '../../core.gbapp/services/GBSSR.js';
import urlJoin from 'url-join';
import Excel from 'exceljs';
import { BufferWindowMemory } from 'langchain/memory';
import { TwitterApi } from 'twitter-api-v2';
import Path from 'path';
import ComputerVisionClient from '@azure/cognitiveservices-computervision';
import ApiKeyCredentials from '@azure/ms-rest-js';
import alasql from 'alasql';
import PizZip from 'pizzip';
import Docxtemplater from 'docxtemplater';
import pptxTemplaterModule from 'pptxtemplater';
import _ from 'lodash';
import { pdfToPng, PngPageOutput } from 'pdf-to-png-converter';
import sharp from 'sharp';
import ImageModule from 'open-docxtemplater-image-module';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService.js';
import { WebAutomationServices } from './WebAutomationServices.js';
import { KeywordsExpressions } from './KeywordsExpressions.js';
import { ChatServices } from '../../gpt.gblib/services/ChatServices.js';
import mime from 'mime-types';
import exts from '../../../extensions.json' assert { type: 'json' };
import { SecService } from '../../security.gbapp/services/SecService.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import retry from 'async-retry';
import { BlobServiceClient, BlockBlobClient, StorageSharedKeyCredential } from '@azure/storage-blob';

import { md5 } from 'js-md5';
import { GBUtil } from '../../../src/util.js';

/**
 * @fileoverview General Bots server core.
 */

/**
 * BASIC system class for extra manipulation of bot behaviour.
 */
export class SystemKeywords {

  public async setSystemPrompt({ pid, text }) {
    let { min, user } = await DialogKeywords.getProcessInfo(pid);

    if (user) {
      ChatServices.userSystemPrompt[user.userSystemId] = text;

      const path = DialogKeywords.getGBAIPath(min.botId);
      const systemPromptFile = urlJoin(process.cwd(), 'work', path, 'users', user.userSystemId, 'systemPrompt.txt');
      Fs.writeFileSync(systemPromptFile, text);
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
    let path = DialogKeywords.getGBAIPath(min.botId, `gbdrive`);

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
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const path = DialogKeywords.getGBAIPath(min.botId, `gbdrive`);
    const root = urlJoin(path, folder);

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

  public async internalCreateDocument(min, path, content) {
    GBLogEx.info(min, `BASIC: CREATE DOCUMENT '${path}...'`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const gbaiName = DialogKeywords.getGBAIPath(min.botId);
    const tmpDocx = urlJoin(gbaiName, path);

    // Templates a blank {content} tag inside the blank.docx.

    const blank = Path.join(process.env.PWD, 'blank.docx');
    let buf = Fs.readFileSync(blank);
    let zip = new PizZip(buf);
    let doc = new Docxtemplater();
    doc.setOptions({ linebreaks: true });
    doc.loadZip(zip);
    doc.setData({ content: content }).render();
    buf = doc.getZip().generate({ type: 'nodebuffer', compression: 'DEFLATE' });

    // Performs the upload.

    await client.api(`${baseUrl}/drive/root:/${tmpDocx}:/content`).put(buf);
  }

  public async createDocument({ pid, path, content }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    this.internalCreateDocument(min, path, content);
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
    GBLogEx.info(min, `BASIC: BEGINING COPY '${src}' to '${dest}'`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const botId = min.instance.botId;

    // Normalizes all slashes.

    src = src.replace(/\\/gi, '/');
    dest = dest.replace(/\\/gi, '/');

    // Determines full path at source and destination.

    const root = DialogKeywords.getGBAIPath(botId, 'gbdrive');
    const srcPath = urlJoin(root, src);
    const dstPath = urlJoin(root, dest);

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
      const file = await client.api(`${baseUrl}/drive/items/${srcFile.id}/copy`).post(destFile);
      GBLogEx.info(min, `BASIC: FINISHED COPY '${src}' to '${dest}'`);
      return file;
    } catch (error) {
      if (error.code === 'itemNotFound') {
        GBLogEx.info(min, `BASIC: COPY source file not found: ${srcPath}.`);
      } else if (error.code === 'nameAlreadyExists') {
        GBLogEx.info(min, `BASIC: COPY destination file already exists: ${dstPath}.`);
      }
      throw error;
    }
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
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `BASIC: CONVERT '${src}' to '${dest}'`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const botId = min.instance.botId;

    // Normalizes all slashes.

    src = src.replace(/\\/gi, '/');
    dest = dest.replace(/\\/gi, '/');

    // Determines full path at source and destination.
    const path = DialogKeywords.getGBAIPath(min.botId, `gbdrive`);
    const root = path;
    const srcPath = urlJoin(root, src);
    const dstPath = urlJoin(path, dest);

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
        GBLogEx.info(min, `BASIC: CONVERT source file not found: ${srcPath}.`);
      } else if (error.code === 'nameAlreadyExists') {
        GBLogEx.info(min, `BASIC: CONVERT destination file already exists: ${dstPath}.`);
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

  private flattenJSON(obj, res = {}, separator = '_', parent = null) {
    for (let key in obj) {
      if (typeof obj[key] === 'function') {
        continue;
      }
      if (typeof obj[key] !== 'object' || obj[key] instanceof Date) {
        // If not defined already add the flattened field.

        const newKey = `${parent ? parent + separator : ''}${key}`;
        if (!res[newKey]) {
          res[newKey] = obj[key];
        } else {
          GBLog.verbose(`Ignoring duplicated field in flatten operation to storage: ${key}.`);
        }
      } else {
        obj[key] = this.flattenJSON(obj[key], res, separator, `${parent ? parent + separator : ''}${key}`);
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
          throw new Error(`BASIC: HTTP:${result.status} retry: ${result.statusText}.`);
        }
        if (result.status === 429) {
          GBLogEx.info(min, `Waiting 1min. before retrying HTTP 429 GET: ${url}`);
          await GBUtil.sleep(60 * 1000);
          throw new Error(`BASIC: HTTP:${result.status} retry: ${result.statusText}.`);
        }
        if (result.status === 503) {
          GBLogEx.info(min, `Waiting 1h before retrying GET 503: ${url}`);
          await GBUtil.sleep(60 * 60 * 1000);
          throw new Error(`BASIC: HTTP:${result.status} retry: ${result.statusText}.`);
        }

        if (result.status === 2000) {
          // Token expired.

          await DialogKeywords.setOption({ pid, name: `${proc.executable}-continuationToken`, value: null });
          bail(new Error(`Expired Token for ${url}.`));
        }
        if (result.status != 200) {
          throw new Error(`BASIC: GET ${result.status}: ${result.statusText}.`);
        }
      },
      {
        retries: 5,
        onRetry: err => {
          GBLog.error(`Retrying HTTP GET due to: ${err.message}.`);
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
    GBLogEx.info(min, `BASIC: PUT ${url} (${data}): ${text}`);

    if (result.status != 200 && result.status != 201) {
      throw new Error(`BASIC: PUT ${result.status}: ${result.statusText}.`);
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
    GBLogEx.info(min, `BASIC: POST ${url} (${data}): ${text}`);

    if (result.status != 200 && result.status != 201) {
      throw new Error(`BASIC: POST ${result.status}: ${result.statusText}.`);
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
    const gbaiName = DialogKeywords.getGBAIPath(botId, 'gbdata');
    let localName;

    // Downloads template from .gbdrive.

    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    let path = '/' + urlJoin(gbaiName, `${botId}.gbdrive`);
    let template = await this.internalGetDocument(client, baseUrl, path, templateName);
    let url = template['@microsoft.graph.downloadUrl'];
    const res = await fetch(url);
    let buf: any = Buffer.from(await res.arrayBuffer());
    localName = Path.join('work', gbaiName, 'cache', `tmp${GBAdminService.getRndReadableIdentifier()}.docx`);
    Fs.writeFileSync(localName, buf, { encoding: null });

    // Replace image path on all elements of data.

    const images = [];
    let index = 0;
    path = Path.join(gbaiName, 'cache', `tmp${GBAdminService.getRndReadableIdentifier()}.docx`);
    url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));

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
              return (orientation || 0) >= 5 ? [height, width] : [width, height];
            };

            const metadata = await sharp(buf).metadata();
            const size = getNormalSize({
              width: metadata['width'],
              height: metadata['height'],
              orientation: metadata['orientation']
            });
            url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(imageName));
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
      GBLog.verbose(`BASIC: MERGE running on ${file}: NO DATA.`);
      return data;
    }

    GBLogEx.info(min, `BASIC: MERGE running on ${file} and key1: ${key1}, key2: ${key2}...`);
    if (!this.cachedMerge[pid]) {
      this.cachedMerge[pid] = { file: {} };
    }

    // Check if is a tree or flat object.

    const hasSubObject = t => {
      for (var key in t) {
        if (!t.hasOwnProperty(key)) continue;
        if (typeof t[key] === 'object') return true;
      }
      return false;
    };

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
            let page = 0,
              pageSize = 1000;
            let count = 0;

            while (page === 0 || count === pageSize) {
              const paged = await t.findAll({ offset: page * pageSize, limit: pageSize, subquery: false, where: {} });
              rows = [...paged, ...rows];
              page++;
              count = paged.length;

              GBLogEx.info(min, `BASIC: MERGE cached: ${rows.length} from page: ${page}.`);
            }
          },
          {
            retries: 5,
            onRetry: err => {
              GBLog.error(`MERGE: Retrying SELECT ALL on table: ${err.message}.`);
            }
          }
        );
      } else {
        rows = this.cachedMerge[pid][file];
      }
    } else {
      const botId = min.instance.botId;
      const path = DialogKeywords.getGBAIPath(botId, 'gbdata');

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
    }

    let table = [];
    let foundIndex = 0;

    // Fills the row variable on the base dataset.

    if (!storage || !this.cachedMerge[pid][file]) {
      for (; foundIndex < rows.length; foundIndex++) {
        let row = {};
        const tmpRow = rows[foundIndex];
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
        }
        row['line'] = foundIndex + 1;
        table.push(row);
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

      if (hasSubObject(row)) {
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

        const foundRow = key1Index[key1Value];
        if (foundRow) {
          found = table[foundRow[0]];
        }
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

          const dst = {};
          let i = 0;
          Object.keys(fieldsValues).forEach(fieldSrc => {
            const name = fieldsNames[i];
            const field = name.charAt(0).toUpperCase() + name.slice(1);
            dst[field] = fieldsValues[fieldSrc];
            i++;
          });

          fieldsValuesList.push(dst);
          this.cachedMerge[pid][file].push(dst);
        } else {
          await this.save({ pid, file, args: fieldsValues });
        }
        adds++;
      }
    }

    // In case of storage, persist to DB in batch.

    if (fieldsValuesList.length) {
      await this.saveToStorageBatch({ pid, table: file, rows: fieldsValuesList });
    }

    GBLogEx.info(min, `BASIC: MERGE results: adds:${adds}, updates:${updates} , skipped: ${skipped}.`);
    return { title: file, adds, updates, skipped };
  }

  /**
   * Publishs a tweet to X.
   *
   * TWEET "My tweet text"
   */
  public async tweet({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    const consumer_key = min.core.getParam(min.instance, 'Twitter Consumer Key', null);
    const consumer_secret = min.core.getParam(min.instance, 'Twitter Consumer Key Secret', null);
    const access_token_key = min.core.getParam(min.instance, 'Twitter Access Token', null);
    const access_token_secret = min.core.getParam(min.instance, 'Twitter Access Token Secret', null);

    if (!consumer_key || !consumer_secret || !access_token_key || !access_token_secret) {
      GBLogEx.info(min, 'Twitter not configured in .gbot.');
    }

    const client = new TwitterApi({
      appKey: consumer_key,
      appSecret: consumer_secret,
      accessToken: access_token_key,
      accessSecret: access_token_secret
    });

    await client.v2.tweet(text);
    GBLogEx.info(min, `Twitter Automation: ${text}.`);
  }

  /**
   * HEAR description
   * text = REWRITE description
   * SAVE "logs.xlsx", username, text
   */
  public async rewrite({ pid, text }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const prompt = `rewrite this sentence in a better way: ${text}`;
    const answer = await ChatServices.continue(min, prompt, 0);
    GBLogEx.info(min, `BASIC: REWRITE ${text} TO ${answer}`);
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

    const gbaiName = DialogKeywords.getGBAIPath(min.botId);

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
    const localName = Path.join('work', gbaiName, 'cache', `qr${GBAdminService.getRndReadableIdentifier()}.png`);
    Fs.writeFileSync(localName, buf, { encoding: null });
    const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));

    GBLogEx.info(min, `GBPay: ${data.MerchantOrderId} OK: ${url}.`);

    return {
      name: Path.basename(localName),
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
    GBLogEx.info(min, `BASIC: Auto saving '${file.filename}' (SAVE file).`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

    const path = DialogKeywords.getGBAIPath(min.botId, `gbdrive`);
    const fileName = file.url ? file.url : file.name;
    const contentType = mime.lookup(fileName);
    const ext = Path.extname(fileName).substring(1);
    const kind = await this.getExtensionInfo(ext);

    let d = new Date(),
      month = '' + (d.getMonth() + 1),
      day = '' + d.getDate(),
      year = d.getFullYear();

    const today = [day, month, year].join('-');
    const result = await client
      .api(`${baseUrl}/drive/root:/${path}/${today}/${kind.category}/${fileName}:/content`)
      .put(file.data);

    return { contentType, ext, kind, category: kind['category'] };
  }

  public async deleteFromStorage({ pid, table, criteria }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `BASIC: DELETE (storage) '${table}' where ${criteria}.`);

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
        onRetry: err => {
          GBLog.error(`Retrying SaveToStorageBatch due to: ${err.message}.`);
        }
      }
    );
  }

  public async deleteFile({ pid, file }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `BASIC: DELETE '${file.name}'.`);
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

    const gbaiPath = DialogKeywords.getGBAIPath(min.botId);
    const fileName = file.name;
    const contentType = mime.lookup(fileName);
    const ext = Path.extname(fileName).substring(1);
    const kind = await this.getExtensionInfo(ext);

    await client.api(`${baseUrl}/drive/root:/${gbaiPath}/${file.path}`).delete();

    return { contentType, ext, kind, category: kind['category'] };
  }

  public async getExtensionInfo(ext: any): Promise<any> {
    let array = exts.filter((v, i, a) => a[i]['extension'] === ext);
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

    let path = DialogKeywords.getGBAIPath(min.botId);
    path = urlJoin(path, remotePath);
    let url = `${baseUrl}/drive/root:/${path}:/children`;

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
        obj['path'] = Path.join(remotePath, item.name);
        obj['url'] = item['@microsoft.graph.downloadUrl'];

        array.push(obj);
      }
    });

    return array;
  }

  public async log({ pid, text: obj }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);

    let level = 0;
    const mydump = (text, level) => {
      var dumped_text = '';

      var level_padding = '';
      for (var j = 0; j < level + 1; j++) level_padding += '    ';

      if (typeof text == 'object') {
        for (var item in text) {
          var value = text[item];

          if (typeof value == 'object') {
            dumped_text += level_padding + "'" + item + "' ...\n";
            dumped_text += mydump(value, level + 1);
          } else {
            dumped_text += level_padding + "'" + item + '\' => "' + value + '"\n';
          }
        }
      } else {
        dumped_text = text + '(' + typeof text + ')';
      }
      return dumped_text;
    };

    GBLogEx.info(min, mydump(obj, level));
  }

  public async getPdf({ pid, file }) {
    const { min } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `BASIC GET (pdf): ${file}`);

    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const gbaiName = DialogKeywords.getGBAIPath(min.botId);
    let path = '/' + urlJoin(gbaiName, `${min.botId}.gbdrive`);
    let template = await this.internalGetDocument(client, baseUrl, path, file);
    let url = template['@microsoft.graph.downloadUrl'];
    const res = await fetch(url);
    let buf: any = Buffer.from(await res.arrayBuffer());
    const data = new Uint8Array(buf);
    const pdf = await getDocument({ data }).promise;
    let pages = [];

    for (let i = 1; i <= pdf.numPages; i++) {
      const page = await pdf.getPage(i);
      const textContent = await page.getTextContent();
      const text = textContent.items
        .map(item => item['str'])
        .join('')
        .replace(/\s/g, '');
      pages.push(text);
    }

    return pages.join('');
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

  public async postToInstagram({ pid, username, password, imagePath, caption }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);

    const ig = new IgApiClient();
    ig.state.generateDevice(username);
    await ig.account.login(username, password);
    const imageBuffer = readFileSync(resolve(imagePath));
    const publishResult = await ig.publish.photo({
      file: imageBuffer,
      caption
    });

    GBLogEx.info(min, `Image posted on IG: ${publishResult}`);
  }
}
