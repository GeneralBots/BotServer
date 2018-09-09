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

const logger = require("../../../src/logger");
const Path = require("path");
const _ = require("lodash");
const UrlJoin = require("url-join");

import { KBService } from './../../kb.gbapp/services/KBService';
import { GBImporter } from "./GBImporter";
import { GBServiceCallback, IGBCoreService, IGBInstance } from "botlib";
import { GBConfigService } from "./GBConfigService";
import { GBError } from "botlib";
import { GuaribasPackage } from '../models/GBModel';

/** Deployer service for bots, themes, ai and more. */
export class GBDeployer {

  core: IGBCoreService;

  importer: GBImporter;

  workDir: string = "./work";

  constructor(core: IGBCoreService, importer: GBImporter) {
    this.core = core;
    this.importer = importer;
  }

  /** 
   * Deploys a bot to the storage. 
   */

  async deployBot(localPath: string): Promise<IGBInstance> {
    let packageType = Path.extname(localPath);
    let packageName = Path.basename(localPath);
    let instance = await this.importer.importIfNotExistsBotPackage(
      packageName,
      localPath
    );
    return instance;
  }

  async deployPackageToStorage(
    instanceId: number,
    packageName: string): Promise<GuaribasPackage> {
    return GuaribasPackage.create({
      packageName: packageName,
      instanceId: instanceId
    });

  }

  deployTheme(localPath: string) {
    // DISABLED: Until completed, "/ui/public".
    // FsExtra.copy(localPath, this.workDir + packageName)
    //   .then(() => {

    //   })
    //   .catch(err => {
    //     var gberr = GBError.create(
    //       `GuaribasBusinessError: Error copying package: ${localPath}.`
    //     );
    //   });
  }

  async deployPackageFromLocalPath(localPath: string) {

    let packageType = Path.extname(localPath);

    switch (packageType) {
      case ".gbot":
        return this.deployBot(localPath);

      case ".gbtheme":
        return this.deployTheme(localPath);

      // PACKAGE: Put in package logic.
      case ".gbkb":
        let service = new KBService();
        return service.deployKb(this.core, this, localPath);

      case ".gbui":

        break;

      default:
        var err = GBError.create(
          `GuaribasBusinessError: Unknow package type: ${packageType}.`
        );
        Promise.reject(err);
        break;
    }
  }

  async undeployPackageFromLocalPath(
    instance: IGBInstance,
    localPath: string

  ) {
    let packageType = Path.extname(localPath);
    let packageName = Path.basename(localPath);

    let p = await this.getPackageByName(instance.instanceId, packageName);

    switch (packageType) {
      case ".gbot":
        // TODO: this.undeployBot(packageName, localPath);
        break;

      case ".gbtheme":
        // TODO: this.undeployTheme(packageName, localPath);
        break;

      case ".gbkb":
        let service = new KBService();
        return service.undeployKbFromStorage(instance, p.packageId);

      case ".gbui":

        break;

      default:
        var err = GBError.create(
          `GuaribasBusinessError: Unknown package type: ${packageType}.`
        );
        Promise.reject(err);
        break;
    }
  }

  async getPackageByName(instanceId: number, packageName: string):
    Promise<GuaribasPackage> {
    var where = { packageName: packageName, instanceId: instanceId };
    return GuaribasPackage.findOne({
      where: where
    });
  }

  /**
   *
   * Hot deploy processing.
   *
   */
  async scanBootPackage() {

    const deployFolder = "deploy";
    let bootPackage = GBConfigService.get("BOOT_PACKAGE");

    if (bootPackage === "none") {
      return Promise.resolve(true);
    } else {
      return this.deployPackageFromLocalPath(
        UrlJoin(deployFolder, bootPackage)
      );
    }
  }
}
