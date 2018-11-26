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

import { IGBCoreService, IGBInstance } from 'botlib';
import { GBError } from 'botlib';
import { IGBPackage } from 'botlib';
const logger = require('../../../src/logger');
import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import * as fs from 'fs';
import { Messages } from '../strings';
import { DialogClass } from './GBAPIService';
import { GBDeployer } from './GBDeployer';
const util = require('util');
const vm = require('vm');
import processExists = require('process-exists');
import { Sequelize } from 'sequelize-typescript';
const UrlJoin = require('url-join');

/**
 * @fileoverview General Bots server core.
 */

export class GBVMService implements IGBCoreService {

  private script = new vm.Script();

  public async loadJS(
    filename: string,
    min: IGBInstance,
    core: IGBCoreService,
    deployer: GBDeployer,
    localPath: string
  ): Promise<void> {

    const code = fs.readFileSync(UrlJoin(localPath, filename), 'utf8');
    const sandbox = new DialogClass(min);

    const context = vm.createContext(sandbox);
    this.script.runInContext(context);
    console.log(util.inspect(sandbox));

    await deployer.deployScriptToStorage(
      min.instanceId,
      filename
    );
    logger.info(`[GBVMService] Finished loading of ${filename}`);
  }
}
