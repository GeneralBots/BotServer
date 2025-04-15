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
 * @fileoverview General Bots server core.
 */

'use strict';

import { GBLog, IGBInstallationDeployer, IGBInstance } from 'botlib';
import fs from 'fs/promises'; 
import { GBAdminService } from '../../../packages/admin.gbapp/services/GBAdminService.js';
import { GBConfigService } from '../../../packages/core.gbapp/services/GBConfigService.js';
import scanf from 'scanf';
import { AzureDeployerService } from '../services/AzureDeployerService.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GBUtil } from '../../../src/util.js';

/**
 * Handles command-line dialog for getting info for Boot Bot.
 */
export class StartDialog {
  public static async createBaseInstance (deployer, freeTier) {
    // No .env so asks for cloud credentials to start a new farm.

    if (!await GBUtil.exists(`.env`)) {
      process.stdout.write(
        'A empty enviroment is detected. To start automatic deploy, please enter some information:\n'
      );
    }

    let botId: string;
    while (botId === undefined) {
      botId = this.retrieveBotId();
    }

    let username: string;
    while (username === undefined) {
      username = this.retrieveUsername();
    }

    let password: string;
    while (password === undefined) {
      password = this.retrievePassword();
    }

    // Connects to the cloud and retrieves subscriptions.

    const credentials = await GBAdminService.getADALCredentialsFromUsername(username, password);
    
    let subscriptionId: string;
    while (subscriptionId === undefined) {
      const list = await (new AzureDeployerService()).getSubscriptions(credentials);
      subscriptionId = this.retrieveSubscriptionId(list);
    }

    const installationDeployer = await AzureDeployerService.createInstanceWithADALCredentials(
       deployer, freeTier, subscriptionId, credentials);

    let location: string;
    while (location === undefined) {
      location = this.retrieveLocation();
    }

    let appId: string;
    while (appId === undefined) {
      appId = this.retrieveAppId();
    }

    let appPassword: string;
    while (appPassword === undefined) {
      appPassword = this.retrieveAppPassword();
    }

    // Prepares the first instance on bot farm.
    
    const instance = <IGBInstance>{};

    instance.botId = botId;
    instance.state = 'active';
    instance.cloudUsername = username;
    instance.cloudPassword = password;
    instance.cloudSubscriptionId = subscriptionId;
    instance.cloudLocation = location;
    instance.marketplaceId = appId;
    instance.marketplacePassword = appPassword;
    instance.adminPass = GBAdminService.getRndPassword();

    return { instance, credentials, subscriptionId , installationDeployer};
  }

  private static retrieveUsername () {
    let value = GBConfigService.get('CLOUD_USERNAME');
    if (value === undefined) {
      process.stdout.write(`${GBAdminService.GB_PROMPT}CLOUD_USERNAME:`);
      value = scanf('%s').replace(/(\n|\r)+$/, '');
    }

    return value;
  }

  private static retrievePassword () {
    let password = GBConfigService.get('CLOUD_PASSWORD');
    if (password === undefined) {
      process.stdout.write(`${GBAdminService.GB_PROMPT}CLOUD_PASSWORD:`);
      password = scanf('%s').replace(/(\n|\r)+$/, '');
    }

    return password;
  }

  private static retrieveBotId () {
    let botId = GBConfigService.get('BOT_ID');
    if (botId === undefined) {
      process.stdout.write(
        `${GBAdminService.GB_PROMPT}Choose a unique bot Id containing lowercase letters, digits or
dashes (cannot use dash as the first two or last one characters),
cannot start or end with or contain consecutive dashes and having 4 to 42 characters long.\n`
      );
      process.stdout.write(`${GBAdminService.GB_PROMPT}BOT_ID:`);
      botId = scanf('%s').replace(/(\n|\r)+$/, '');
    }

    return botId;
  }

  /**
   *
   * Update Manifest in Azure: "signInAudience": "AzureADandPersonalMicrosoftAccount" and "accessTokenAcceptedVersion": 2.
   */
  private static retrieveAppId () {
    let appId = GBConfigService.get('MARKETPLACE_ID');
    if (appId === undefined) {
      process.stdout.write(
        `Sorry, this part cannot be automated yet due to Microsoft schedule,
please go to https://apps.dev.microsoft.com/portal/register-app to
generate manually an App ID and App Secret.\n`
      );

      process.stdout.write('Generated Application Id (MARKETPLACE_ID):');
      appId = scanf('%s').replace(/(\n|\r)+$/, '');
    }

    return appId;
  }

  private static retrieveAppPassword () {
    let appPassword = GBConfigService.get('MARKETPLACE_SECRET');
    if (appPassword === undefined) {
      process.stdout.write('Generated Password (MARKETPLACE_SECRET):');
      appPassword = scanf('%s').replace(/(\n|\r)+$/, '');
    }

    return appPassword;
  }

  private static retrieveSubscriptionId (list) {
    let subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');
    if (subscriptionId){
      
      return subscriptionId;
    }
    const map = {};
    let index = 1;
    list.forEach(element => {
      GBLogEx.info(0, `${index}: ${element.displayName} (${element.subscriptionId})`);
      map[index++] = element;
    });
    let subscriptionIndex;
    if (!subscriptionIndex && subscriptionId === undefined) {
      process.stdout.write('CLOUD_SUBSCRIPTIONID (type a number):');
      subscriptionIndex = scanf('%d');
      subscriptionId = map[subscriptionIndex].subscriptionId;
    }

    return subscriptionId;
  }

  private static retrieveLocation () {
    let location = GBConfigService.get('CLOUD_LOCATION');
    if (location === undefined) {
      process.stdout.write('CLOUD_LOCATION (eg. westus):');
      location = scanf('%s');
    }

    return location;
  }
}
