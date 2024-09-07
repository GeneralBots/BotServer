/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import { GBMinInstance, IGBCoreService, IGBInstance } from 'botlib';
import { CreateOptions } from 'sequelize/types';
import fs from 'fs/promises'; 
import urlJoin from 'url-join';
import { GBServer } from '../../../src/app.js';
import { GuaribasInstance } from '../models/GBModel.js';
import { GBConfigService } from './GBConfigService.js';
import { GBUtil } from '../../../src/util.js';

/**
 * Handles the importing of packages.
 */
export class GBImporter {
  public core: IGBCoreService;

  constructor (core: IGBCoreService) {
    this.core = core;
  }

  public async importIfNotExistsBotPackage (
    botId: string,
    packageName: string,
    localPath: string,
    additionalInstance: IGBInstance = null
  ) {
    const file = urlJoin(localPath, 'settings.json');

    let settingsJson = {botId: botId}; 
    if (await GBUtil.exists(file)){

      settingsJson = JSON.parse(await fs.readFile(file, 'utf8'));
      if (botId === undefined) {
        botId = settingsJson.botId;
      }
      
    }

    let instance: IGBInstance;
    if (botId === undefined) {
      botId = GBConfigService.get('BOT_ID');
      instance = await this.core.loadInstanceByBotId(botId);
      if (!instance) {
        instance = <IGBInstance>{};
        instance.state = 'active';
        instance.adminPass = GBConfigService.get('ADMIN_PASS');
        instance.botId = GBConfigService.get('BOT_ID');
        instance.cloudSubscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');
        instance.cloudLocation = GBConfigService.get('CLOUD_LOCATION');
        instance.cloudUsername = GBConfigService.get('CLOUD_USERNAME');
        instance.cloudPassword = GBConfigService.get('CLOUD_PASSWORD');
        instance.marketplaceId = GBConfigService.get('MARKETPLACE_ID');
        instance.marketplacePassword = GBConfigService.get('MARKETPLACE_SECRET');
        instance.storageDialect = GBConfigService.get('STORAGE_DIALECT');
        instance.storageServer = GBConfigService.get('STORAGE_SERVER');
        instance.storageName = GBConfigService.get('STORAGE_NAME');
        instance.storageUsername = GBConfigService.get('STORAGE_USERNAME');
        instance.storagePassword = GBConfigService.get('STORAGE_PASSWORD');
      }
    } else {
      instance = await this.core.loadInstanceByBotId(botId);
    }

    if (instance != undefined && !instance.botId) {
      console.log(`Null BotId after load instance with botId: ${botId}.`);
    } else {
      instance = additionalInstance;
    }

    return await this.createOrUpdateInstanceInternal(instance, botId, localPath, settingsJson);
  }

  public async createBotInstance (botId: string) {
    const fullSettingsJson = <GuaribasInstance>{  };
    fullSettingsJson['botId'] = botId;

    return await GuaribasInstance.create(fullSettingsJson);
  }

  private async createOrUpdateInstanceInternal (
    instance: IGBInstance,
    botId: string,
    localPath: string,
    settingsJson: any
  ) {
  
    const fullSettingsJson = { ...GBServer.globals.bootInstance, ...settingsJson,
    description:"General Bot", title:botId    
    };

    if (botId !== undefined) {
      fullSettingsJson.botId = botId;
    }

    if (instance !== null) {
      instance = { ...instance, ...fullSettingsJson };

      return await this.core.saveInstance(instance);
    } else {
      return await GuaribasInstance.create(fullSettingsJson);
    }
  }
}
