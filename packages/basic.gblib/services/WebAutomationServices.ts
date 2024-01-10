/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__,\| (˅) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.             |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights,title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import urlJoin from 'url-join';
import Fs from 'fs';
import Path from 'path';
import url from 'url';

import { GBLog, GBMinInstance } from 'botlib';
import { GBServer } from '../../../src/app.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { GBSSR } from '../../core.gbapp/services/GBSSR.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';
import { DialogKeywords } from './DialogKeywords.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { Mutex } from 'async-mutex';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { SystemKeywords } from './SystemKeywords.js';

/**
 * Web Automation services of conversation to be called by BASIC.
 */
export class WebAutomationServices {
  static isSelector(name: any) {
    return name.startsWith('.') || name.startsWith('#') || name.startsWith('[');
  }
  private debugWeb: boolean;
  private lastDebugWeb: Date;

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

  public async closeHandles({ pid }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    await DialogKeywords.setOption({ pid, name: "filter", value: null });

    // Releases previous allocated OPEN semaphores.

    let keys = Object.keys(GBServer.globals.webSessions);
    for (let i = 0; i < keys.length; i++) {
      const session = GBServer.globals.webSessions[keys[i]];
      if (session.activePid === pid) {
        session.semaphore.release();
        GBLogEx.info(min, `Release for PID: ${pid} done.`);
      }
    }
  }

  /**
   * Returns the page object.
   *
   * @example OPEN "https://wikipedia.org"
   */

  public async openPage({ pid, handle, sessionKind, sessionName, url, username, password }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `BASIC: Web Automation OPEN ${sessionName ? sessionName : ''} ${url}.`);

    // Try to find an existing handle.

    let session;
    if (handle) {
      session = GBServer.globals.webSessions[handle];
    }
    else if (sessionName) {
      let keys = Object.keys(GBServer.globals.webSessions);
      for (let i = 0; i < keys.length; i++) {
        if (GBServer.globals.webSessions[keys[i]].sessionName === sessionName) {
          session = GBServer.globals.webSessions[keys[i]];
          handle = keys[i];
          break;
        }
      }
    }

    let page;
    if (session) {
      page = session.page;

      // Semaphore logic to block multiple entries on the same session.

      if (sessionName) {
        GBLogEx.info(min, `Acquiring (1) for PID: ${pid}...`);
        const release = await session.semaphore.acquire();
        GBLogEx.info(min, `Acquire (1) for PID: ${pid} done.`);
        try {
          session.activePid = pid;
          session.release = release;
        } catch {
          release();
        }
      }
    }

    // Creates the page if it is the first time.

    let browser;
    if (!page) {
      browser = await GBSSR.createBrowser(null);
      page = (await browser.pages())[0];
      if (username || password) {
        await page.authenticate({ pid, username: username, password: password });
      }
    }

    // There is no session yet or it is an unamed session.

    if ((!session && sessionKind === 'AS') || !sessionName) {
      // A new web session is being created.

      handle = WebAutomationServices.cyrb53(min.botId + url);

      session = {};
      session.sessionName = sessionName;
      session.page = page;
      session.browser = browser;
      session.semaphore = new Mutex();
      session.activePid = pid;

      GBServer.globals.webSessions[handle] = session;

      // Only uses semaphore logic in named web sessions.

      if (sessionName) {
        GBLogEx.info(min, `Acquiring (2) for PID: ${pid}...`);
        const release = await session.semaphore.acquire();
        session.release = release;
        GBLogEx.info(min, `Acquire (2) for PID: ${pid} done.`);
      }
    }

    // WITH is only valid in a previously defined session.

    if (!session && sessionKind == 'WITH') {
      const error = `NULL session for OPEN WITH #${sessionName}.`;
      GBLogEx.error(min, error);
    }

    await page.goto(url);

    return handle;
  }

  public static getPageByHandle(handle) {
    return GBServer.globals.webSessions[handle].page;
  }

  /**
   * Find element on page DOM.
   *
   * @example GET "selector"
   */
  public async getBySelector({ handle, selector, pid }) {
    const page = WebAutomationServices.getPageByHandle(handle);
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `BASIC: Web Automation GET element: ${selector}.`);
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
  public async getByFrame({ handle, frame, selector }) {
    const page = WebAutomationServices.getPageByHandle(handle);
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
  public async hover({ pid, handle, selector }) {
    const page = WebAutomationServices.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation HOVER element: ${selector}.`);
    await this.getBySelector({ handle, selector: selector, pid });
    await page.hover(selector);
    await this.debugStepWeb(pid, page);
  }

  /**
   * Clicks on an element in a web page.
   *
   * @example CLICK "#idElement"
   */
  public async click({ pid, handle, frameOrSelector, selector }) {
    const page = WebAutomationServices.getPageByHandle(handle);
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

  private async debugStepWeb(pid, page) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    let refresh = true;
    if (this.lastDebugWeb) {
      refresh = new Date().getTime() - this.lastDebugWeb.getTime() > 5000;
    }

    if (this.debugWeb && refresh) {
      const mobile = min.core.getParam(min.instance, 'Bot Admin Number', null);
      const filename = page;
      if (mobile) {
        await new DialogKeywords().sendFileTo({ pid: pid, mobile, filename, caption: 'General Bots Debugger' });
      }
      this.lastDebugWeb = new Date();
    }
  }

  /**
   * Press ENTER in a web page,useful for logins.
   *
   * @example PRESS ENTER ON page
   */
  public async pressKey({ handle, char, frame }) {
    const page = WebAutomationServices.getPageByHandle(handle);
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

  public async linkByText({ pid, handle, text, index }) {
    const page = WebAutomationServices.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation CLICK LINK TEXT: ${text} ${index}.`);
    if (!index) {
      index = 1;
    }
    const els = await page.$x(`//a[contains(.,'${text}')]`);
    await els[index - 1].click();
    await this.debugStepWeb(pid, page);
  }

  public async clickButton({ pid, handle, text, index }) {
    const page = WebAutomationServices.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation CLICK BUTTON: ${text} ${index}.`);
    if (!index) {
      index = 1;
    }
    const els = await page.$x(`//button[contains(.,'${text}')]`);
    await els[index - 1].click();
    await this.debugStepWeb(pid, page);
  }


  /**
   * Returns the screenshot of page or element
   *
   * @example file = SCREENSHOT "#selector"
   */
  public async screenshot({ pid, handle, selector }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const page = WebAutomationServices.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation SCREENSHOT ${selector}.`);

    const gbaiName = DialogKeywords.getGBAIPath(min.botId);
    const localName = Path.join('work', gbaiName, 'cache', `screen-${GBAdminService.getRndReadableIdentifier()}.jpg`);

    await page.screenshot({ path: localName });

    const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));
    GBLog.info(`BASIC: WebAutomation: Screenshot captured at ${url}.`);

    return { data: null, localName: localName, url: url };
  }

  /**
   * Types the text into the text field.
   *
   * @example SET page,"selector","text"
   */
  public async setElementText({ pid, handle, selector, text }) {
    text = `${text}`;
    const page = WebAutomationServices.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation TYPE on ${selector}: ${text}.`);
    const e = await this.getBySelector({ handle, selector, pid });
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
  public async download({ pid, handle, selector, folder }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    const page = WebAutomationServices.getPageByHandle(handle);

    const element = await this.getBySelector({ handle, selector, pid });
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
    let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    const botId = min.instance.botId;

    // Normalizes all slashes.

    folder = folder.replace(/\\/gi, '/');

    // Determines full path at source and destination.
    const path = DialogKeywords.getGBAIPath(min.botId, `gbdrive`);
    const root = path;
    const dstPath = urlJoin(root, folder, filename);

    // Checks if the destination contains subfolders that
    // need to be created.

    folder = await new SystemKeywords().createFolder(folder);

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

  private async recursiveFindInFrames(inputFrame, selector) {
    const frames = inputFrame.childFrames();
    const results = await Promise.all(
      frames.map(async frame => {
        const el = await frame.$(selector);
        if (el) return el;
        if (frame.childFrames().length > 0) {
          return await this.recursiveFindInFrames(frame, selector);
        }
        return null;
      })
    );

    return results.find(Boolean);
  }


  public async getTextOf({ pid, handle, frameOrSelector, selector }) {
    const page = WebAutomationServices.getPageByHandle(handle);
    GBLog.info(`BASIC: Web Automation CLICK element: ${frameOrSelector}.`);
    if (frameOrSelector) {
      const result = await page.$eval(
        frameOrSelector,
        (ul) => {
          let items = "";
          for (let i = 0; i < ul.children.length; i++) {
            items = `${ul.children[i].textContent}\n`;
          }
          return items;
        }
      )
      await this.debugStepWeb(pid, page);

      return result;
    }
  }
}
