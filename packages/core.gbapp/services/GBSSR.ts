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

/**
 * @fileoverview General Bots SSR support based on https://www.npmjs.com/package/ssr-for-bots.
 */

'use strict';

import { createRequire } from 'module';
const require = createRequire(import.meta.url);

import path from 'path';
import fs from 'fs/promises';
import { NextFunction, Request, Response } from 'express';
import urljoin from 'url-join';
import { GBMinInstance } from 'botlib';
import { GBServer } from '../../../src/app.js';
import { GBLogEx } from './GBLogEx.js';
import urlJoin from 'url-join';
import { GBDeployer } from './GBDeployer.js';
import { GBMinService } from './GBMinService.js';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { GBUtil } from '../../../src/util.js';
const puppeteer = require('puppeteer-extra');
const hidden = require('puppeteer-extra-plugin-stealth');
const { executablePath } = require('puppeteer');

export class GBSSR {
  // https://hackernoon.com/tips-and-tricks-for-web-scraping-with-puppeteer-ed391a63d952
  // Dont download all resources, we just need the HTML
  // Also, this is huge performance/response time boost
  private static blockedResourceTypes = [
    'image',
    'media',
    'font',
    'texttrack',
    'object',
    'beacon',
    'csp_report',
    'imageset'
  ];

  // const whitelist = ["document", "script", "xhr", "fetch"];
  private static skippedResources = [
    'quantserve',
    'adzerk',
    'doubleclick',
    'adition',
    'exelator',
    'sharethrough',
    'cdn.api.twitter',
    'google-analytics',
    'googletagmanager',
    'google',
    'fontawesome',
    'facebook',
    'analytics',
    'optimizely',
    'clicktale',
    'mixpanel',
    'zedo',
    'clicksor',
    'tiqcdn'
  ];

  public static async preparePuppeteer(profilePath) {
    let args = [
      '--check-for-update-interval=2592000',
      '--disable-accelerated-2d-canvas',
      '--disable-dev-shm-usage',
      '--disable-features=site-per-process',
      '--disable-gpu',
      '--no-first-run',
      '--no-sandbox',
      '--no-default-browser-check'
    ];

    if (profilePath) {
      args.push(`--user-data-dir=${profilePath}`);

      const preferences = urljoin(profilePath, 'Default', 'Preferences');
      if (await GBUtil.exists(preferences)) {
        const file = await fs.readFile(preferences, 'utf8');
        const data = JSON.parse(file);
        data['profile']['exit_type'] = 'none';
        await fs.writeFile(preferences, JSON.stringify(data));
      }
    }

    return {
      args: args,
      ignoreHTTPSErrors: true,
      headless: process.env.CHROME_HEADLESS === 'true',
      defaultViewport: null,
      executablePath: process.env.CHROME_PATH ? process.env.CHROME_PATH : executablePath(),
      ignoreDefaultArgs: ['--enable-automation', '--enable-blink-features=IdleDetection']
    };
  }


  public static async createBrowser(profilePath): Promise<any> {
    const opts = await this.preparePuppeteer(profilePath);
    puppeteer.use(hidden());
    puppeteer.use(require("puppeteer-extra-plugin-minmax")());
    const browser = await puppeteer.launch(opts);
    return browser;
  }

  /**
   * Return the HTML of bot default.gbui.
   */
  public static async getHTML(min: GBMinInstance) {
    const url = urljoin(GBServer.globals.publicAddress, min.botId);
    const browser = await GBSSR.createBrowser(null);
    const stylesheetContents = {};
    let html;

    try {
      const page = await browser.newPage();
      await page.minimize();

      await page.setUserAgent(
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/83.0.4103.61 Safari/537.36'
      );
      await page.setRequestInterception(true);
      page.on('request', request => {
        const requestUrl = request.url().split('?')[0].split('#')[0];
        if (
          GBSSR.blockedResourceTypes.indexOf(request.resourceType()) !== -1 ||
          GBSSR.skippedResources.some(resource => requestUrl.indexOf(resource) !== -1)
        ) {
          request.abort();
        } else {
          request.continue();
        }
      });

      page.on('response', async resp => {
        const responseUrl = resp.url();
        const sameOrigin = new URL(responseUrl).origin === new URL(url).origin;
        const isStylesheet = resp.request().resourceType() === 'stylesheet';
        if (sameOrigin && isStylesheet) {
          stylesheetContents[responseUrl] = await resp.text();
        }
      });

      await page.setExtraHTTPHeaders({
        'ngrok-skip-browser-warning': '1'
      });
      const response = await page.goto(url, {
        timeout: 120000,
        waitUntil: 'networkidle0'
      });

      await GBUtil.sleep(6000);

      // Inject <base> on page to relative resources load properly.

      await page.evaluate(url => {
        const base = document.createElement('base');
        base.href = url;
        // Add to top of head, before all other resources.
        document.head.prepend(base);
      }, url);

      // Remove scripts and html imports. They've already executed.

      await page.evaluate(() => {
        const elements = document.querySelectorAll('script, link[rel="import"]');
        elements.forEach(e => {
          e.remove();
        });
      });

      // Replace stylesheets in the page with their equivalent <style>.

      await page.$$eval(
        'link[rel="stylesheet"]',
        (links, content) => {
          links.forEach((link: any) => {
            const cssText = content[link.href];
            if (cssText) {
              const style = document.createElement('style');
              style.textContent = cssText;
              link.replaceWith(style);
            }
          });
        },
        stylesheetContents
      );

      html = await page.content();

      // Close the page we opened here (not the browser).

      await page.close();
    } catch (e) {
      const html = e.toString();
      GBLogEx.error(min, `URL: ${url} Failed with message: ${html}`);
    } finally {
      await browser.close();
    }
    return html;
  }

  public static async ssrFilter(req: Request, res: Response, next) {
    let applyOptions = {
      prerender: [], // Array containing the user-agents that will trigger the ssr service
      exclude: [], // Array containing paths and/or extentions that will be excluded from being prerendered by the ssr service
      useCache: true, // Variable that determins if we will use page caching or not
      cacheRefreshRate: 86400 // Seconds of which the cache will be kept alive, pass 0 or negative value for infinite lifespan
    };

    // Default user agents
    const prerenderArray = [
      'bot',
      'googlebot',
      'Chrome-Lighthouse',
      'DuckDuckBot',
      'ia_archiver',
      'bingbot',
      'yandex',
      'baiduspider',
      'Facebot',
      'facebookexternalhit',
      'facebookexternalhit/1.1',
      'twitterbot',
      'rogerbot',
      'linkedinbot',
      'embedly',
      'quora link preview',
      'showyoubot',
      'outbrain',
      'pinterest',
      'slackbot',
      'vkShare',
      'W3C_Validator'
    ];

    // default exclude array
    const excludeArray = ['.xml', '.ico', '.txt', '.json'];
    const userAgent: string = req.headers['user-agent'] || '';
    const prerender = new RegExp([...prerenderArray, ...applyOptions.prerender].join('|').slice(0, -1), 'i').test(
      userAgent
    );
    const exclude = !new RegExp([...excludeArray, ...applyOptions.exclude].join('|').slice(0, -1)).test(
      req.originalUrl
    );

    // Tries to find botId from URL.

    const minBoot = GBServer.globals.minBoot;

    let onlyChars: any = /\/([A-Za-z0-9\-\_]+)\/*/.exec(req.originalUrl);
    onlyChars = onlyChars ? onlyChars[1] : minBoot.botId;

    let botId =
      req.originalUrl && req.originalUrl === '/' ?
        minBoot.botId :
        onlyChars;


    let min: GBMinInstance =
      req.url === '/'
        ? minBoot
        : GBServer.globals.minInstances.filter(p => p.instance.botId.toLowerCase() === botId.toLowerCase())[0];
    if (!min) {
      min = req.url === '/'
        ? minBoot
        : GBServer.globals.minInstances.filter(p =>
          p.instance.activationCode ? p.instance.activationCode.toLowerCase() === botId.toLowerCase()
            : null)[0];
    }
    if (!min) {
      botId = minBoot.botId;
    }


    let packagePath = GBUtil.getGBAIPath(botId, `gbui`);

    // Checks if the bot has an .gbui published or use default.gbui.

    if (!await GBUtil.exists(packagePath)) {
      packagePath = path.join(process.env.PWD, 'packages', `default.gbui`, 'build');
    }
    let parts = req.url.replace(`/${botId}`, '').split('?');
    let url = parts[0];

    if (min && req.originalUrl && prerender && exclude && await GBUtil.exists(packagePath)) {

      // Reads from static HTML when a bot is crawling.

      packagePath = path.join(process.env.PWD, 'work', packagePath, 'index.html');
      const html = await fs.readFile(packagePath, 'utf8');
      res.status(200).send(html);
      return true;
    } else {

      // Servers default.gbui web application.

      packagePath = path.join(
        process.env.PWD,
        GBDeployer.deployFolder,
        GBMinService.uiPackage,
        'build',
        url === '/' || url === '' ? `index.html` : url
      );
      if (GBServer.globals.wwwroot && url === '/') {
        packagePath = GBServer.globals.wwwroot + "/index.html"; // TODO.
      }
      if (!min && !url.startsWith("/images") && GBServer.globals.wwwroot) {
        packagePath = path.join(GBServer.globals.wwwroot, url);
      }
      if (!min && !url.startsWith("/static") && GBServer.globals.wwwroot) {
        packagePath = path.join(GBServer.globals.wwwroot, url);
      }
      if (await GBUtil.exists(packagePath)) {
        if (min) {
          let html = await fs.readFile(packagePath, 'utf8');
          html = html.replace(/\{p\}/gi, min.botId);
          html = html.replace(/\{botId\}/gi, min.botId);

          const theme =
            `theme-${await (min.core as any)['getParam'](min.instance, 'Theme Color', 'grey')}`;

          html = html.replace(/\{themeColor\}/gi, theme);

          html = html.replace(/\{theme\}/gi, min.instance.theme ? min.instance.theme :
            'default.gbtheme');
          html = html.replace(/\{title\}/gi, min.instance.title);
          res.send(html).end();
        } else {
          res.sendFile(packagePath);
        }
        return true;
      } else {
        GBLogEx.verbose(min, `HTTP 404: ${req.url}.`);
        res.status(404);
        res.end();
      }
    }
  }
}
