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

import { AuthenticationContext, TokenResponse } from 'adal-node';
import { GBLog, GBMinInstance, IGBAdminService, IGBCoreService, IGBDeployer, IGBInstance } from 'botlib';
import urlJoin = require('url-join');
import { AzureDeployerService } from '../../azuredeployer.gbapp/services/AzureDeployerService';
import { GuaribasInstance } from '../../core.gbapp/models/GBModel';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { GBImporter } from '../../core.gbapp/services/GBImporterService';
import { GBSharePointService } from '../../sharepoint.gblib/services/SharePointService';
import { GuaribasAdmin } from '../models/AdminModel';
const Path = require('path');
const msRestAzure = require('ms-rest-azure');
const PasswordGenerator = require('strict-password-generator').default;

/**
 * Services for server administration.
 */
export class GBAdminService implements IGBAdminService {
  public static GB_PROMPT: string = 'GeneralBots: ';
  public static masterBotInstanceId = 0;

  public static StrongRegex = new RegExp('^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9])(?=.*[!@#$%^&*+_-])(?=.{8,})');

  public core: IGBCoreService;

  constructor(core: IGBCoreService) {
    this.core = core;
  }

  public static generateUuid(): string {
    return msRestAzure.generateUuid();
  }

  public static getNodeVersion() {
    const packageJson = urlJoin(process.cwd(), 'package.json');
    // tslint:disable-next-line: non-literal-require
    const pjson = require(packageJson);

    return pjson.engines.node.replace('=', '');
  }

  public static async getADALTokenFromUsername(username: string, password: string) {
    const credentials = await GBAdminService.getADALCredentialsFromUsername(username, password);

    return credentials.tokenCache._entries[0].accessToken;
  }

  public static async getADALCredentialsFromUsername(username: string, password: string) {
    return await msRestAzure.loginWithUsernamePassword(username, password);
  }

  public static getMobileCode() {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: false,
      lowerCaseAlpha: false,
      number: true,
      specialCharacter: false,
      minimumLength: 6,
      maximumLength: 6
    };

    return passwordGenerator.generatePassword(options);
  }

  public static getRndPassword(): string {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: true,
      lowerCaseAlpha: true,
      number: true,
      specialCharacter: true,
      minimumLength: 14,
      maximumLength: 14
    };
    let password = passwordGenerator.generatePassword(options);
    password = password.replace(/[\@\[\=\:\;\?]/gi, '#');

    return password;
  }

  public static getRndReadableIdentifier() {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: false,
      lowerCaseAlpha: true,
      number: false,
      specialCharacter: false,
      minimumLength: 14,
      maximumLength: 14
    };

    return passwordGenerator.generatePassword(options);
  }

  public static getNumberIdentifier() {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: false,
      lowerCaseAlpha: false,
      number: true,
      specialCharacter: false,
      minimumLength: 14,
      maximumLength: 14
    };

    return passwordGenerator.generatePassword(options);
  }

  public static async undeployPackageCommand(text: any, min: GBMinInstance) {
    const packageName = text.split(' ')[1];
    const importer = new GBImporter(min.core);
    const deployer = new GBDeployer(min.core, importer);
    const localFolder = Path.join('work', `${min.instance.botId}.gbai`, Path.basename(packageName));
    await deployer.undeployPackageFromLocalPath(min.instance, localFolder);
  }

  public static isSharePointPath(path: string) {
    return path.indexOf('sharepoint.com') > 0;
  }
  public static async deployPackageCommand(min: GBMinInstance, text: string, deployer: IGBDeployer) {
    const packageName = text.split(' ')[1];

    if (!this.isSharePointPath(packageName)) {
      const additionalPath = GBConfigService.get('ADDITIONAL_DEPLOY_PATH');
      if (additionalPath === undefined) {
        throw new Error('ADDITIONAL_DEPLOY_PATH is not set and deployPackage was called.');
      }
      await deployer.deployPackage(min, urlJoin(additionalPath, packageName));
    } else {
      const siteName = text.split(' ')[1];
      const folderName = text.split(' ')[2];

      const s = new GBSharePointService();

      const localFolder = Path.join('work', `${min.instance.botId}.gbai`, Path.basename(folderName));

      // .gbot packages are handled using storage API, so no download
      // of local resources is required.

      if (!localFolder.endsWith('.gbot')) {
        GBLog.warn(`${GBConfigService.get('CLOUD_USERNAME')} must be authorized on SharePoint related site`);
        await s.downloadFolder(
          localFolder,
          siteName,
          folderName,
          GBConfigService.get('CLOUD_USERNAME'),
          GBConfigService.get('CLOUD_PASSWORD')
        );
      }
      await deployer.deployPackage(min, localFolder);
    }
  }
  public static async rebuildIndexPackageCommand(min: GBMinInstance, deployer: IGBDeployer) {
    await deployer.rebuildIndex(
      min.instance,
      new AzureDeployerService(deployer).getKBSearchSchema(min.instance.searchIndex)
    );
  }

  public static async syncBotServerCommand(min: GBMinInstance, deployer: GBDeployer) {
    const serverName = `${min.instance.botId}-server`;
    const service = await AzureDeployerService.createInstance(deployer);
    service.syncBotServerRepository(min.instance.botId, serverName);
  }

  public async setValue(instanceId: number, key: string, value: string) {
    const options = { where: {} };
    options.where = { key: key };
    let admin = await GuaribasAdmin.findOne(options);
    if (admin === null) {
      admin = new GuaribasAdmin();
      admin.key = key;
    }
    admin.value = value;
    admin.instanceId = instanceId;
    await admin.save();
  }

  public async updateSecurityInfo(
    instanceId: number,
    authenticatorTenant: string,
    authenticatorAuthorityHostUrl: string
  ): Promise<IGBInstance> {
    const options = { where: {} };
    options.where = { instanceId: instanceId };
    const item = await GuaribasInstance.findOne(options);
    item.authenticatorTenant = authenticatorTenant;
    item.authenticatorAuthorityHostUrl = authenticatorAuthorityHostUrl;

    return item.save();
  }

  public async getValue(instanceId: number, key: string): Promise<string> {
    const options = { where: {} };
    options.where = { key: key, instanceId: instanceId };
    const obj = await GuaribasAdmin.findOne(options);

    return obj.value;
  }

  public async acquireElevatedToken(instanceId: number): Promise<string> {
    // TODO: Use boot bot as base for authentication.

    const botId = GBConfigService.get('BOT_ID');
    instanceId = (await this.core.loadInstanceByBotId(botId)).instanceId;

    return new Promise<string>(async (resolve, reject) => {
      const instance = await this.core.loadInstanceById(instanceId);

      const expiresOn = new Date(await this.getValue(instanceId, 'expiresOn'));
      if (expiresOn.getTime() > new Date().getTime()) {
        const accessToken = await this.getValue(instanceId, 'accessToken');
        resolve(accessToken);
      } else {
        const authorizationUrl = urlJoin(
          instance.authenticatorAuthorityHostUrl,
          instance.authenticatorTenant,
          '/oauth2/authorize'
        );

        const refreshToken = await this.getValue(instanceId, 'refreshToken');
        const resource = 'https://graph.microsoft.com';
        const authenticationContext = new AuthenticationContext(authorizationUrl);
        authenticationContext.acquireTokenWithRefreshToken(
          refreshToken,
          instance.marketplaceId,
          instance.marketplacePassword,
          resource,
          async (err, res) => {
            if (err !== null) {
              reject(err);
            } else {
              const token = res as TokenResponse;
              try {
                await this.setValue(instanceId, 'accessToken', token.accessToken);
                await this.setValue(instanceId, 'refreshToken', token.refreshToken);
                await this.setValue(instanceId, 'expiresOn', token.expiresOn.toString());
                resolve(token.accessToken);
              } catch (error) {
                reject(err);
              }
            }
          }
        );
      }
    });
  }

  public async publish(min: GBMinInstance, packageName: string, republish: boolean): Promise<void> { }
}
