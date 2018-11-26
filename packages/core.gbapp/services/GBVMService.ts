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
import * as fs from 'fs';
import { BotAdapter } from 'botbuilder';
import { WaterfallDialog } from 'botbuilder-dialogs';
import { Messages } from '../strings';
import { GBDeployer } from './GBDeployer';
const util = require('util');
const vm = require('vm');

/**
 * @fileoverview General Bots server core.
 */

export class GBVMService implements IGBCoreService {

  public static setup(bot: BotAdapter, min: IGBInstance) {

  }

  public loadJS(
    filename: string,
    min: IGBInstance,
    core: IGBCoreService,
    deployer: GBDeployer,
    localPath: string
  ) {

    const sandbox = {
      animal: 'cat',
      count: 2,
    };

    const script = new vm.Script('count += 1; name = "kitty";');
    const context = vm.createContext(sandbox);

    for (let i = 0; i < 10; ++i) {
      script.runInContext(context);
    }

    console.log(util.inspect(sandbox));

    // { animal: 'cat', count: 12, name: 'kitty' }

    const packageType = Path.extname(localPath);
    const packageName = Path.basename(localPath);
    logger.info(`[GBDeployer] Opening package: ${localPath}`);
    const packageObject = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, 'package.json'), 'utf8'),
    );

    const instance = await core.loadInstance(packageObject.botId);
    logger.info(`[GBDeployer] Importing: ${localPath}`);
    const p = await deployer.deployPackageToStorage(
      instance.instanceId,
      packageName,
    );
    await this.importKbPackage(localPath, p, instance);

    deployer.rebuildIndex(instance);
    logger.info(`[GBDeployer] Finished import of ${localPath}`);
  }
}
