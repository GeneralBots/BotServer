/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
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

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import { GBLog, IGBInstallationDeployer, IGBInstance } from 'botlib';
import * as fs from 'fs';
import { GBAdminService } from '../../../packages/admin.gbapp/services/GBAdminService';
import { GBConfigService } from '../../../packages/core.gbapp/services/GBConfigService';
const scanf = require('scanf');

/**
 * Handles command-line dialog for getting info for Boot Bot.
 */
export class StartDialog {
  public static async createBaseInstance(installationDeployer: IGBInstallationDeployer) {
    // No .env so asks for cloud credentials to start a new farm.

    if (!fs.existsSync(`.env`)) {
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
    const list = await installationDeployer.getSubscriptions(credentials);

    let subscriptionId: string;
    while (subscriptionId === undefined) {
      subscriptionId = this.retrieveSubscriptionId(list);
    }

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

    let authoringKey: string;
    while (authoringKey === undefined) {
      authoringKey = this.retrieveAuthoringKey();
    }

    // Prepares the first instance on bot farm.
    const instance = <IGBInstance>{};

    instance.botId = botId;
    instance.state = 'active';
    instance.cloudUsername = username;
    instance.cloudPassword = password;
    instance.cloudSubscriptionId = subscriptionId;
    instance.cloudLocation = location;
    instance.nlpAuthoringKey = authoringKey;
    instance.marketplaceId = appId;
    instance.marketplacePassword = appPassword;
    instance.adminPass = GBAdminService.getRndPassword();

    return { instance, credentials, subscriptionId };
  }

  private static retrieveUsername() {
    let value = GBConfigService.get('CLOUD_USERNAME');
    if (value === undefined) {
      process.stdout.write(`${GBAdminService.GB_PROMPT}CLOUD_USERNAME:`);
      value = scanf('%s').replace(/(\n|\r)+$/, '');
    }

    return value;
  }

  private static retrievePassword() {
    let password = GBConfigService.get('CLOUD_PASSWORD');
    if (password === undefined) {
      process.stdout.write(`${GBAdminService.GB_PROMPT}CLOUD_PASSWORD:`);
      password = scanf('%s').replace(/(\n|\r)+$/, '');
    }

    return password;
  }

  private static retrieveBotId() {
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

  private static retrieveAuthoringKey() {
    let authoringKey = GBConfigService.get('NLP_AUTHORING_KEY');
    if (authoringKey === undefined) {
      process.stdout.write(
        `${
        GBAdminService.GB_PROMPT
        }Due to this opened issue: https://github.com/Microsoft/botbuilder-tools/issues/550\n`
      );
      process.stdout.write(
        `${
        GBAdminService.GB_PROMPT
        }Please enter your LUIS Authoring Key, get it here: https://www.luis.ai/user/settings and paste it to me:`
      );
      authoringKey = scanf('%s').replace(/(\n|\r)+$/, '');
    }

    return authoringKey;
  }

  private static retrieveAppId() {
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

  private static retrieveAppPassword() {
    let appPassword = GBConfigService.get('MARKETPLACE_SECRET');
    if (appPassword === undefined) {
      process.stdout.write('Generated Password (MARKETPLACE_SECRET):');
      appPassword = scanf('%s').replace(/(\n|\r)+$/, '');
    }

    return appPassword;
  }

  private static retrieveSubscriptionId(list) {
    let subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');
    const map = {};
    let index = 1;
    list.forEach(element => {
      GBLog.info(`${index}: ${element.displayName} (${element.subscriptionId})`);
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

  private static retrieveLocation() {
    let location = GBConfigService.get('CLOUD_LOCATION');
    if (location === undefined) {
      process.stdout.write('CLOUD_LOCATION (eg. westus):');
      location = scanf('%s');
    }

    return location;
  }
}
