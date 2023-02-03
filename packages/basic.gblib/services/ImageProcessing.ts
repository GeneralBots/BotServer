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
import { DialogKeywords } from './DialogKeywords.js';
import sharp from 'sharp';

/**
 * Image processing services of conversation to be called by BASIC.
 */
export class ImageProcessing {
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

  sys: any;

  /**
   * When creating this keyword facade,a bot instance is
   * specified among the deployer service.
   */
  constructor(min: GBMinInstance, user, dk) {
    this.min = min;
    this.user = user;
    this.dk = dk;
  }

  /**
   * Returns the page object.
   *
   * @example OPEN "https://wikipedia.org"
   */
  public async sharpen({ pid, file: file }) {
    GBLog.info(`BASIC: Image Processing SHARPEN ${file}.`);

    const gbfile = DialogKeywords.getFileByHandle(file);
    const data = await sharp(gbfile.data)
      .sharpen({
        sigma: 2,
        m1: 0,
        m2: 3,
        x1: 3,
        y2: 15,
        y3: 15
      })
      .toBuffer();

    const newFile = {
      filename: gbfile.filename,
      data: data

    };
    return;
  }
}
