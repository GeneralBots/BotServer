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

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import { AuthenticationContext, TokenResponse } from 'adal-node';
import { IGBCoreService } from 'botlib';
import { GuaribasAdmin } from '../models/AdminModel';
const UrlJoin = require('url-join');
const msRestAzure = require('ms-rest-azure');
const PasswordGenerator = require('strict-password-generator').default;

export class GBAdminService {

  public static GB_PROMPT: string = 'GeneralBots: ';
  public static masterBotInstanceId = 0;

  public static StrongRegex = new RegExp(
    '^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9])(?=.*[!@#$%^&*+_-])(?=.{8,})'
  );

  public core: IGBCoreService;

  constructor(core: IGBCoreService) {
    this.core = core;
  }

  public static generateUuid(): string {
    return msRestAzure.generateUuid();
  }

  public static async getADALTokenFromUsername(
    username: string,
    password: string
  ) {
    const credentials = await GBAdminService.getADALCredentialsFromUsername(
      username,
      password
    );
    const accessToken = credentials.tokenCache._entries[0].accessToken;
    return accessToken;
  }

  public static async getADALCredentialsFromUsername(
    username: string,
    password: string
  ) {
    const credentials = await msRestAzure.loginWithUsernamePassword(
      username,
      password
    );
    return credentials;
  }

  public static getRndPassword() {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: true,
      lowerCaseAlpha: true,
      number: true,
      specialCharacter: true,
      minimumLength: 12,
      maximumLength: 14
    };
    let password = passwordGenerator.generatePassword(options);
    password = password.replace(/@[=:;\?]/g, '#');
    return password;
  }

  public static getRndReadableIdentifier() {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: false,
      lowerCaseAlpha: true,
      number: false,
      specialCharacter: false,
      minimumLength: 12,
      maximumLength: 14
    };
    const name = passwordGenerator.generatePassword(options);
    return name;
  }

  public async setValue(
    instanceId: number,
    key: string,
    value: string
  ): Promise<GuaribasAdmin> {
    const options = { where: {} };
    options.where = { key: key };
    let admin = await GuaribasAdmin.findOne(options);
    if (admin == null) {
      admin = new GuaribasAdmin();
      admin.key = key;
    }
    admin.value = value;
    admin.instanceId = instanceId;
    return admin.save();
  }

  public async getValue(instanceId: number, key: string) {
    const options = { where: {} };
    options.where = { key: key, instanceId: instanceId };
    const obj = await GuaribasAdmin.findOne(options);
    return Promise.resolve(obj.value);
  }

  public async acquireElevatedToken(instanceId): Promise<string> {
    return new Promise<string>(async (resolve, reject) => {
      const instance = await this.core.loadInstanceById(instanceId);

      const expiresOn = new Date(await this.getValue(instanceId, 'expiresOn'));
      if (expiresOn.getTime() > new Date().getTime()) {
        const accessToken = await this.getValue(instanceId, 'accessToken');
        resolve(accessToken);
      } else {
        const authorizationUrl = UrlJoin(
          instance.authenticatorAuthorityHostUrl,
          instance.authenticatorTenant,
          '/oauth2/authorize'
        );

        const refreshToken = await this.getValue(instanceId, 'refreshToken');
        const resource = 'https://graph.microsoft.com';
        const authenticationContext = new AuthenticationContext(authorizationUrl);
        authenticationContext.acquireTokenWithRefreshToken(
          refreshToken,
          instance.authenticatorClientId,
          instance.authenticatorClientSecret,
          resource,
          async (err, res) => {
            if (err) {
              reject(err);
            } else {
              const token = res as TokenResponse;
              await this.setValue(
                instanceId,
                'accessToken',
                token.accessToken
              );
              await this.setValue(
                instanceId,
                'refreshToken',
                token.refreshToken
              );
              await this.setValue(
                instanceId,
                'expiresOn',
                token.expiresOn.toString()
              );
              resolve(token.accessToken);
            }
          }
        );
      }
    });
  }

}
