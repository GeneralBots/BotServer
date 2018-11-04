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

"use strict";

import { GuaribasAdmin } from "../models/AdminModel";
import { IGBCoreService } from "botlib";
import { AuthenticationContext, TokenResponse } from "adal-node";
const UrlJoin = require("url-join");
const msRestAzure = require("ms-rest-azure");
const PasswordGenerator = require("strict-password-generator").default;

export class GBAdminService {

  static GB_PROMPT: string = "GeneralBots: "

  static generateUuid(): string {
    return msRestAzure.generateUuid();
  }
  static masterBotInstanceId = 0;

  public static StrongRegex = new RegExp(
    "^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9])(?=.*[!@#$%^&*+_-])(?=.{8,})"
  );

  core: IGBCoreService;

  constructor(core: IGBCoreService) {
    this.core = core;
  }

  public async setValue(
    instanceId: number,
    key: string,
    value: string
  ): Promise<GuaribasAdmin> {
    let options = { where: {} };
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
    let options = { where: {} };
    options.where = { key: key, instanceId: instanceId };
    let obj = await GuaribasAdmin.findOne(options);
    return Promise.resolve(obj.value);
  }

  public async acquireElevatedToken(instanceId): Promise<string> {
    return new Promise<string>(async (resolve, reject) => {
      let instance = await this.core.loadInstanceById(instanceId);

      let expiresOn = new Date(await this.getValue(instanceId, "expiresOn"));
      if (expiresOn.getTime() > new Date().getTime()) {
        let accessToken = await this.getValue(instanceId, "accessToken");
        resolve(accessToken);
      } else {
        let authorizationUrl = UrlJoin(
          instance.authenticatorAuthorityHostUrl,
          instance.authenticatorTenant,
          "/oauth2/authorize"
        );

        let refreshToken = await this.getValue(instanceId, "refreshToken");
        let resource = "https://graph.microsoft.com";
        var authenticationContext = new AuthenticationContext(authorizationUrl);
        authenticationContext.acquireTokenWithRefreshToken(
          refreshToken,
          instance.authenticatorClientId,
          instance.authenticatorClientSecret,
          resource,
          async (err, res) => {
            if (err) {
              reject(err);
            } else {
              let token = res as TokenResponse;
              await this.setValue(
                instanceId,
                "accessToken",
                token.accessToken
              );
              await this.setValue(
                instanceId,
                "refreshToken",
                token.refreshToken
              );
              await this.setValue(
                instanceId,
                "expiresOn",
                token.expiresOn.toString()
              );
              resolve(token.accessToken);
            }
          }
        );
      }
    });
  }

  public static async getADALTokenFromUsername(
    username: string,
    password: string
  ) {
    let credentials = await GBAdminService.getADALCredentialsFromUsername(
      username,
      password
    );
    let accessToken = credentials.tokenCache._entries[0].accessToken;
    return accessToken;
  }

  public static async getADALCredentialsFromUsername(
    username: string,
    password: string
  ) {
    let credentials = await msRestAzure.loginWithUsernamePassword(
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
    password = password.replace(/@[=:;\?]/g, "#");
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
    let name = passwordGenerator.generatePassword(options);
    return name;
  }

}
