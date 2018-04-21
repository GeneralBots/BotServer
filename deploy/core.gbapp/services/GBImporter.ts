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

const _ = require("lodash");
const Parse = require("csv-parse");
const Async = require("async");
const UrlJoin = require("url-join");
const Walk = require("fs-walk");
const logger = require("../../../src/logger");

import { KBService } from './../../kb.gbapp/services/KBService';
import { Sequelize } from "sequelize-typescript";
import { Promise } from "bluebird";
import Fs = require("fs");
import Path = require("path");
import { DataTypeUUIDv1 } from "sequelize";
import { GBConfigService } from "./GBConfigService";
import { GBCoreService } from "./GBCoreService";
import { GBServiceCallback, IGBCoreService, IGBInstance } from "botlib";
import { SecService } from "../../security.gblib/services/SecService";
import { GuaribasInstance } from "../models/GBModel";

export class GBImporter {
  core: IGBCoreService;

  constructor(core: IGBCoreService) {
    this.core = core;
  }
  importIfNotExistsBotPackage(
    packageName: string,
    localPath: string,
    cb: GBServiceCallback<IGBInstance>
  ) {
    let _this = this;

    let packageJson = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, "package.json"), "utf8")
    );

    let botId = packageJson.botId;

    this.core.loadInstance(botId, (instance, err) => {
      if (instance) {
        cb(instance, null);
      } else {
        this.createInstanceInternal(packageName, localPath, packageJson, cb);
      }
    });
  }

  private createInstanceInternal(
    packageName: string,
    localPath: string,
    packageJson: any,
    cb: GBServiceCallback<IGBInstance>
  ) {
    const settings = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, "settings.json"), "utf8")
    );
    const servicesJson = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, "services.json"), "utf8")
    );

    packageJson = Object.assign(packageJson, settings, servicesJson);

    GuaribasInstance.create(packageJson).then((instance: IGBInstance) => {
      
      // PACKAGE: security.json loading
      let service = new SecService();
      service.importSecurityFile(localPath, instance);

      cb(instance, null);
    });
  }
}