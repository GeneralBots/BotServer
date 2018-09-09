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
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
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

const UrlJoin = require("url-join");
import Fs = require("fs");
import Path = require("path");
import { IGBCoreService, IGBInstance } from "botlib";
import { SecService } from "../../security.gblib/services/SecService";
import { GuaribasInstance } from "../models/GBModel";

export class GBImporter {
  core: IGBCoreService;

  constructor(core: IGBCoreService) {
    this.core = core;
  }

  async importIfNotExistsBotPackage(
    packageName: string,
    localPath: string) {

    let packageJson = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, "package.json"), "utf8")
    );

    let botId = packageJson.botId;

    let instance = await this.core.loadInstance(botId);
    if (instance) {
      return Promise.resolve(instance);
    } else {
      return this.createInstanceInternal(packageName, localPath, packageJson);
    }
  }

  private async createInstanceInternal(
    packageName: string,
    localPath: string,
    packageJson: any
  ) {
    const settings = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, "settings.json"), "utf8")
    );
    const servicesJson = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, "services.json"), "utf8")
    );

    packageJson = Object.assign(packageJson, settings, servicesJson);

    GuaribasInstance.create(packageJson).then((instance: IGBInstance) => {

      let service = new SecService();
      // TODO: service.importSecurityFile(localPath, instance);

      Promise.resolve(instance);
    });
  }
}