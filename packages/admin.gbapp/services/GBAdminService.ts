

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

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import { AuthenticationContext, TokenResponse } from 'adal-node';
import { GBError, GBLog, GBMinInstance, IGBAdminService, IGBCoreService, IGBDeployer, IGBInstance } from 'botlib';
import { FindOptions } from 'sequelize/types';
import urlJoin from 'url-join';
import { AzureDeployerService } from '../../azuredeployer.gbapp/services/AzureDeployerService.js';
import { GuaribasInstance } from '../../core.gbapp/models/GBModel.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { GBImporter } from '../../core.gbapp/services/GBImporterService.js';
import { GBSharePointService } from '../../sharepoint.gblib/services/SharePointService.js';
import { GuaribasAdmin } from '../models/AdminModel.js';
import msRestAzure from 'ms-rest-azure';
import Path from 'path';
import { caseSensitive_Numbs_SpecialCharacters_PW, lowercase_PW } from 'super-strong-password-generator'
import crypto from 'crypto';
import Fs from 'fs';
import { GBServer } from '../../../src/app.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';

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
    return crypto.randomUUID();
  }

  public static getNodeVersion() {
    const packageJson = urlJoin(process.cwd(), 'package.json');
    const pkg = JSON.parse(Fs.readFileSync(packageJson, 'utf8'));
    return pkg.engines.node.replace('=', '');
  }

  public static async getADALTokenFromUsername(username: string, password: string) {
    const credentials = await GBAdminService.getADALCredentialsFromUsername(username, password);

    return (credentials as any).tokenCache._entries[0].accessToken;
  }

  public static async getADALCredentialsFromUsername(username: string, password: string) {

    return await msRestAzure.loginWithUsernamePassword(username, password);
  }

  public static getMobileCode() {

    return this.getNumberIdentifier(6);
  }

  public static getRndPassword(): string {
    
    let password = caseSensitive_Numbs_SpecialCharacters_PW(15);
    password = password.replace(/[\@\[\=\:\;\?\"\'\#]/gi, '*');

    const removeRepeatedChars = (s, r) => {
      let res = '', last = null, counter = 0;
      s.split('').forEach(char => {
          if (char == last)
              counter++;
          else {
              counter = 0;
              last = char;
          }
          if (counter < r)
              res += char;
      });    
      return res;
    }

    return removeRepeatedChars(password, 1);
  }

  public static getRndReadableIdentifier(): string {

    return lowercase_PW(14);
  }

  public static getNumberIdentifier(digits: number = 14): string {

    if (digits <= 0) {
      throw new Error('Number of digits should be greater than 0.');
    }

    const min = 10 ** (digits - 1);
    const max = 10 ** digits - 1;
    const randomNumber = Math.floor(Math.random() * (max - min + 1)) + min;
    return randomNumber.toString();
  }

  /**
   * @see https://stackoverflow.com/a/52171480
   */
  public static getHash(str: string, seed = 0) {
    let h1 = 0xdeadbeef ^ seed,
      h2 = 0x41c6ce57 ^ seed;
    for (let i = 0, ch; i < str.length; i++) {
      ch = str.charCodeAt(i);
      h1 = Math.imul(h1 ^ ch, 2654435761);
      h2 = Math.imul(h2 ^ ch, 1597334677);
    }
    h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
    h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
    return 4294967296 * (2097151 & h2) + (h1 >>> 0);
  }

  public static async undeployPackageCommand(text: string, min: GBMinInstance) {

    const packageName = text.split(' ')[1];
    const importer = new GBImporter(min.core);
    const deployer = new GBDeployer(min.core, importer);
    const path = DialogKeywords.getGBAIPath(min.botId, null, packageName);
    const localFolder = Path.join('work', path);
    await deployer.undeployPackageFromLocalPath(min.instance, localFolder);
  }

  public static isSharePointPath(path: string) {
    return path.indexOf('sharepoint.com') !== -1;
  }
  public static async deployPackageCommand(min: GBMinInstance, user: GuaribasUser, text: string, deployer: IGBDeployer) {
    const packageName = text.split(' ')[1];

    if (!this.isSharePointPath(packageName)) {
      const additionalPath = GBConfigService.get('ADDITIONAL_DEPLOY_PATH');
      if (additionalPath === undefined) {
        throw new Error('ADDITIONAL_DEPLOY_PATH is not set and deployPackage was called.');
      }
      await deployer['deployPackage2'](min, user, urlJoin(additionalPath, packageName));
    } else {
      const folderName = text.split(' ')[2];
      const packageType = Path.extname(folderName).substr(1);
      const gbaiPath = DialogKeywords.getGBAIPath(min.instance.botId, packageType, null);
      const localFolder = Path.join('work', gbaiPath);

      // .gbot packages are handled using storage API, so no download
      // of local resources is required.
      const gbai = DialogKeywords.getGBAIPath(min.instance.botId);

      if (packageType === 'gbkb') {
        await deployer['cleanupPackage'](min.instance, packageName);
      }

      await deployer['downloadFolder'](min,
        Path.join('work', `${gbai}`),
        Path.basename(localFolder));
      await deployer['deployPackage2'](min, user, localFolder);
    }
  }
  public static async rebuildIndexPackageCommand(min: GBMinInstance, deployer: GBDeployer) {
    const service = await AzureDeployerService.createInstance(deployer);
    const searchIndex = min.instance.searchIndex ? min.instance.searchIndex : GBServer.globals.minBoot.instance.searchIndex;
    await deployer.rebuildIndex(
      min.instance,
      service.getKBSearchSchema(searchIndex)
    );
  }

  public static async syncBotServerCommand(min: GBMinInstance, deployer: GBDeployer) {
    const serverName = `${min.instance.botId}-server`;
    const service = await AzureDeployerService.createInstance(deployer);
    service.syncBotServerRepository(min.instance.botId, serverName);
  }

  public async setValue(instanceId: number, key: string, value: string) {
    const options = <FindOptions>{ where: {} };
    options.where = { key: key, instanceId: instanceId };
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
    const options = <FindOptions>{ where: {} };
    options.where = { instanceId: instanceId };
    const item = await GuaribasInstance.findOne(options);
    item.authenticatorTenant = authenticatorTenant;
    item.authenticatorAuthorityHostUrl = authenticatorAuthorityHostUrl;

    return item.save();
  }

  public async getValue(instanceId: number, key: string): Promise<string> {
    const options = <FindOptions>{ where: {} };
    options.where = { key: key, instanceId: instanceId };
    const obj = await GuaribasAdmin.findOne(options);
    return obj.value;
  }

  public async acquireElevatedToken(instanceId: number, root: boolean = false,
    tokenName: string = '',
    clientId: string = null,
    clientSecret: string = null,
    host: string = null,
    tenant: string = null
  ): Promise<string> {


    if (root) {
      const minBoot = GBServer.globals.minBoot;
      instanceId = minBoot.instance.instanceId;
    }
    GBLogEx.info(instanceId, `Acquiring token for instanceId: ${instanceId} ${tokenName} (root: ${root}).`);

    let expiresOnV;
    try {
      expiresOnV = await this.getValue(instanceId, `${tokenName}expiresOn`);
    } catch (error) {
      throw new Error(`/setupSecurity is required before running /publish.`);
    }


    return new Promise<string>(async (resolve, reject) => {
      const instance = await this.core.loadInstanceById(instanceId);

      const expiresOn = new Date(expiresOnV);
      if (expiresOn.getTime() > new Date().getTime()) {
        const accessToken = await this.getValue(instanceId, `${tokenName}accessToken`);
        resolve(accessToken);
      } else {

        if (tokenName && !root) {

          const refreshToken = await this.getValue(instanceId, `${tokenName}refreshToken`);

          let url = urlJoin(
            host,
            tenant, 'oauth/token');
          let buff = new Buffer(`${clientId}:${clientSecret}`);
          const base64 = buff.toString('base64');

          const options = {
            method: 'POST',
            headers: {
              Accept: '1.0',
              Authorization: `Basic ${base64}`,
              'Content-Type': 'application/x-www-form-urlencoded'
            },
            body: new URLSearchParams({
              'grant_type': 'refresh_token',
              'refresh_token': refreshToken
            })
          };
          const result = await fetch(url, options);

          if (result.status != 200) {
            const text = await result.text();
            throw new Error(`acquireElevatedToken refreshing token: ${result.status}: ${result.statusText} ${text}.`);
          }

          const text = await result.text();
          const token = JSON.parse(text);

          // Saves token to the database.

          await this.setValue(instanceId, `${tokenName}accessToken`, token['access_token']);
          await this.setValue(instanceId, `${tokenName}refreshToken`, token['refresh_token']);
          await this.setValue(instanceId, `${tokenName}expiresOn`,
            new Date(Date.now() + (token['expires_in'] * 1000)).toString());
          await this.setValue(instanceId, `${tokenName}AntiCSRFAttackState`, null);

          resolve(token['access_token']);

        }
        else {

          const oauth2 = tokenName ? 'oauth' : 'oauth2';
          const authorizationUrl = urlJoin(
            tokenName ? host : instance.authenticatorAuthorityHostUrl,
            tokenName ? tenant : instance.authenticatorTenant,
            `/${oauth2}/authorize`
          );

          const refreshToken = await this.getValue(instanceId, `${tokenName}refreshToken`);
          const resource = tokenName ? '' : 'https://graph.microsoft.com';
          const authenticationContext = new AuthenticationContext(authorizationUrl);

          authenticationContext.acquireTokenWithRefreshToken(
            refreshToken,
            tokenName ? clientId : instance.marketplaceId,
            tokenName ? clientSecret : instance.marketplacePassword,
            resource,
            async (err, res) => {
              if (err !== null) {
                reject(err);
              } else {
                const token = res as TokenResponse;
                try {
                  await this.setValue(instanceId, `${tokenName}accessToken`, token.accessToken);
                  await this.setValue(instanceId, `${tokenName}refreshToken`, token.refreshToken);
                  await this.setValue(instanceId, `${tokenName}expiresOn`, token.expiresOn.toString());
                  resolve(token.accessToken);
                } catch (error) {
                  reject(err);
                }
              }
            }
          );
        }
      }
    });
  }

  public async publish(min: GBMinInstance, packageName: string, republish: boolean): Promise<void> { }
}
