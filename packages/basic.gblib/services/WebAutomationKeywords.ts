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
import { GBServer } from '../../../src/app.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { createBrowser } from '../../core.gbapp/services/GBSSR.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';
import { DialogKeywords } from './DialogKeywords.js';

import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import urlJoin from 'url-join';
import Fs from 'fs';
import Path from 'path';
import url from 'url';
import { pid } from 'process';

/**
 * Web Automation services of conversation to be called by BASIC.
 */
export class WebAutomationKeywords {
  /**
   * Reference to minimal bot instance.
   */
  public min: GBMinInstance;

  /**
   * Reference to the base system keywords functions to be called.
   */
  public dk: DialogKeywords;

  /**
   * Current user object to get BASIC properties read.
   */
  public user;

  /**
   * HTML browser for conversation over page interaction.
   */
  browser: any;

  sys: any;

  /**
   * The number used in this execution for HEAR calls (useful for SET SCHEDULE).
   */
  hrOn: string;

  userId: GuaribasUser;
  debugWeb: boolean;
  lastDebugWeb: Date;

  /**
   * SYSTEM account maxLines,when used with impersonated contexts (eg. running in SET SCHEDULE).
   */
  maxLines: number = 2000;

  pageMap = {};

  public static cyrb53 = (str, seed = 0) => {
    let h1 = 0xdeadbeef ^ seed,
      h2 = 0x41c6ce57 ^ seed;
    for (let i = 0, ch; i < str.length; i++) {
      ch = str.charCodeAt(i);
      h1 = Math.imul(h1 ^ ch, 2654435761);
      h2 = Math.imul(h2 ^ ch, 1597334677);
    }

    h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
    h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);

    return 4294967296 * (2097151 & h2) + (h1 >>> 0);
  };

  /**
   * When creating this keyword facade,a bot instance is
   * specified among the deployer service.
   */
  constructor (min: GBMinInstance, user, dk) {
    this.min = min;
    this.user = user;
    this.dk = dk;

    this.debugWeb = this.min.core.getParam<boolean>(this.min.instance, 'Debug Web Automation', false);
  }

  /**
   * Returns the page object.
   *
   * @example OPEN "https://wikipedia.org"
   */
  public async getPage ({ pid, url, username, password }) {
    GBLog.info(`BASIC: Web Automation GET PAGE ${url}.`);
    if (!this.browser) {
      this.browser = await createBrowser(null);
    }
    const page = (await this.browser.pages())[0];
    if (username || password) {
      await page.authenticate({pid, username: username, password: password });
    }
    await page.goto(url);

    const handle = WebAutomationKeywords.cyrb53(this.min.botId + url);

    this.pageMap[handle] = page;

    return handle;
  }

  public getPageByHandle (hash) {
    return this.pageMap[hash];
  }

  /**
   * Find element on page DOM.
   *
   * @example GET "selector"
   */
  public async getBySelector ({ handle, selector }) {
    const page = this.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation GET element: ${selector}.`);
    await page.waitForSelector(selector);
    let elements = await page.$$(selector);
    if (elements && elements.length > 1) {
      return elements;
    } else {
      const el = elements[0];
      el['originalSelector'] = selector;
      el['href'] = await page.evaluate(e => e.getAttribute('href'), el);
      el['value'] = await page.evaluate(e => e.getAttribute('value'), el);
      el['name'] = await page.evaluate(e => e.getAttribute('name'), el);
      el['class'] = await page.evaluate(e => e.getAttribute('class'), el);
      return el;
    }
  }

  /**
   * Find element on page DOM.
   *
   * @example GET page,"frameSelector,"elementSelector"
   */
  public async getByFrame ({ handle, frame, selector }) {
    const page = this.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation GET element by frame: ${selector}.`);
    await page.waitForSelector(frame);
    let frameHandle = await page.$(frame);
    const f = await frameHandle.contentFrame();
    await f.waitForSelector(selector);
    const element = await f.$(selector);
    element['originalSelector'] = selector;
    element['href'] = await f.evaluate(e => e.getAttribute('href'), element);
    element['value'] = await f.evaluate(e => e.getAttribute('value'), element);
    element['name'] = await f.evaluate(e => e.getAttribute('name'), element);
    element['class'] = await f.evaluate(e => e.getAttribute('class'), element);
    element['frame'] = f;
    return element;
  }

  /**
   * Simulates a mouse hover an web page element.
   */
  public async hover ({ pid, handle, selector }) {
    const page = this.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation HOVER element: ${selector}.`);
    await this.getBySelector({ handle, selector: selector });
    await page.hover(selector);
    await this.debugStepWeb(pid, page);
  }

  /**
   * Clicks on an element in a web page.
   *
   * @example CLICK page,"#idElement"
   */
  public async click ({ pid, handle, frameOrSelector, selector }) {
    const page = this.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation CLICK element: ${frameOrSelector}.`);
    if (selector) {
      await page.waitForSelector(frameOrSelector);
      let frameHandle = await page.$(frameOrSelector);
      const f = await frameHandle.contentFrame();
      await f.waitForSelector(selector);
      await f.click(selector);
    } else {
      await page.waitForSelector(frameOrSelector);
      await page.click(frameOrSelector);
    }
    await this.debugStepWeb(pid, page);
  }

  private async debugStepWeb (pid, page) {
    let refresh = true;
    if (this.lastDebugWeb) {
      refresh = new Date().getTime() - this.lastDebugWeb.getTime() > 5000;
    }

    if (this.debugWeb && refresh) {
      const mobile = this.min.core.getParam(this.min.instance, 'Bot Admin Number', null);
      const filename = page;
      if (mobile) {
        await this.dk.sendFileTo({pid: pid,  mobile, filename, caption: 'General Bots Debugger' });
      }
      this.lastDebugWeb = new Date();
    }
  }

  /**
   * Press ENTER in a web page,useful for logins.
   *
   * @example PRESS ENTER ON page
   */
  public async pressKey ({ handle, char, frame }) {
    const page = this.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation PRESS ${char} ON element: ${frame}.`);
    if (char.toLowerCase() === 'enter') {
      char = '\n';
    }
    if (frame) {
      await page.waitForSelector(frame);
      let frameHandle = await page.$(frame);
      const f = await frameHandle.contentFrame();
      await f.keyboard.press(char);
    } else {
      await page.keyboard.press(char);
    }
  }

  public async linkByText ({ pid, handle, text, index }) {
    const page = this.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation CLICK LINK TEXT: ${text} ${index}.`);
    if (!index) {
      index = 1;
    }
    const els = await page.$x(`//a[contains(.,'${text}')]`);
    await els[index - 1].click();
    await this.debugStepWeb(pid, page);
  }

  /**
   * Returns the screenshot of page or element
   *
   * @example file = SCREENSHOT page
   */
  public async screenshot ({ handle, selector }) {
    const page = this.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation SCREENSHOT ${selector}.`);

    const gbaiName = `${this.min.botId}.gbai`;
    const localName = Path.join('work', gbaiName, 'cache', `screen-${GBAdminService.getRndReadableIdentifier()}.jpg`);

    await page.screenshot({ path: localName });

    const url = urlJoin(GBServer.globals.publicAddress, this.min.botId, 'cache', Path.basename(localName));
    GBLog.info(`BASIC: WebAutomation: Screenshot captured at ${url}.`);

    return url;
  }

  /**
   * Types the text into the text field.
   *
   * @example SET page,"selector","text"
   */
  public async setElementText ({ pid, handle, selector, text }) {
    const page = this.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation TYPE on ${selector}: ${text}.`);
    const e = await this.getBySelector({ handle, selector });
    await e.click({ clickCount: 3 });
    await page.keyboard.press('Backspace');
    await e.type(text, { delay: 200 });
    await this.debugStepWeb(pid, page);
  }

  /**
   * Performs the download to the .gbdrive Download folder.
   *
   * @example file = DOWNLOAD element, folder
   */
  public async download ({ handle, selector, folder }) {
    const page = this.getPageByHandle(handle);
    
    const element = await this.getBySelector({ handle, selector });
    // https://github.com/GeneralBots/BotServer/issues/311
    const container = element['_frame'] ? element['_frame'] : element['_page'];
    await page.setRequestInterception(true);
    await container.click(element.originalSelector);

    const xRequest = await new Promise(resolve => {
      page.on('request', interceptedRequest => {
        interceptedRequest.abort(); //stop intercepting requests
        resolve(interceptedRequest);
      });
    });

    const options = {
      encoding: null,
      method: xRequest['._method'],
      uri: xRequest['_url'],
      body: xRequest['_postData'],
      headers: xRequest['_headers']
    };

    const cookies = await page.cookies();
    options.headers.Cookie = cookies.map(ck => ck.name + '=' + ck.value).join(';');
    GBLog.info(`BASIC: DOWNLOADING '${options.uri}...'`);

    let local;
    let filename;
    if (options.uri.indexOf('file://') != -1) {
      local = url.fileURLToPath(options.uri);
      filename = Path.basename(local);
    } else {
      const getBasenameFormUrl = urlStr => {
        const url = new URL(urlStr);
        return Path.basename(url.pathname);
      };
      filename = getBasenameFormUrl(options.uri);
    }

    let result: Buffer;
    if (local) {
      result = Fs.readFileSync(local);
    } else {
      const res = await fetch(options.uri, options);
      result = Buffer.from(await res.arrayBuffer());
    }
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(this.min);
    const botId = this.min.instance.botId;

    // Normalizes all slashes.

    folder = folder.replace(/\\/gi, '/');

    // Determines full path at source and destination.

    const root = urlJoin(`/${botId}.gbai/${botId}.gbdrive`);
    const dstPath = urlJoin(root, folder, filename);

    // Checks if the destination contains subfolders that
    // need to be created.

    folder = await this.sys.createFolder(folder);

    // Performs the conversion operation getting a reference
    // to the source and calling /content on drive API.
    let file;
    try {
      file = await client.api(`${baseUrl}/drive/root:/${dstPath}:/content`).put(result);
    } catch (error) {
      if (error.code === 'nameAlreadyExists') {
        GBLog.info(`BASIC: DOWNLOAD destination file already exists: ${dstPath}.`);
      }
      throw error;
    }

    return file;
  }
}
