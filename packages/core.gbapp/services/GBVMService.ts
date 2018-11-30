/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
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

import { GBMinInstance, IGBCoreService } from 'botlib';
import * as fs from 'fs';
import { DialogClass } from './GBAPIService';
import { GBDeployer } from './GBDeployer';
const util = require('util');
const logger = require('../../../src/logger');
const vm = require('vm');
const UrlJoin = require('url-join');

/**
 * @fileoverview Virtualization services for emulation of BASIC.
 */

export class GBVMService implements IGBCoreService {
  private script = new vm.Script();

  public async loadJS(
    filename: string,
    min: GBMinInstance,
    core: IGBCoreService,
    deployer: GBDeployer,
    localPath: string
  ): Promise<void> {

    localPath = UrlJoin(localPath, 'bot.vbs.js');
    const code: string = fs.readFileSync(localPath, 'utf8');
    const sandbox: DialogClass = new DialogClass(min);
    const context = vm.createContext(sandbox);
    vm.runInContext(code, context);

    await deployer.deployScriptToStorage(min.instanceId, filename);
    logger.info(`[GBVMService] Finished loading of ${filename}`);

    min.sandbox = sandbox;
  }
}
