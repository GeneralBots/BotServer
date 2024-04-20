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

import { GBSSR }from '../../core.gbapp/services/GBSSR.js';

export class ChartServices {
  /**
   * Generate chart image screenshot
   * @param {object} options billboard.js generation option object
   * @param {string} path screenshot image full path with file name
   */
  public static async screenshot (args, path) {
    const browser = await GBSSR.createBrowser(null);
    const page = await browser.newPage();

    // load billboard.js assets from CDN.
    await page.addStyleTag({ url: 'https://cdn.jsdelivr.net/npm/billboard.js/dist/theme/datalab.min.css' });
    await page.addScriptTag({ url: 'https://cdn.jsdelivr.net/npm/billboard.js/dist/billboard.pkgd.min.js' });

    await page.evaluate(`bb.generate(${JSON.stringify(args)});`);

    const content = await page.$('.bb');

    await content.screenshot({
      path,
      omitBackground: true
    });

    await page.close();
    await browser.close();
  }
}
