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
const Fs = require("fs");
const FsExtra = require("fs-extra");
const _ = require("lodash");
const Async = require("async");
const UrlJoin = require("url-join");
const Walk = require("fs-walk");
const WaitUntil = require("wait-until");

import { KBService } from './../../kb.gbapp/services/KBService';
import { GBImporter } from "./GBImporter";
import { GBCoreService } from "./GBCoreService";
import { GBServiceCallback, IGBCoreService, IGBInstance } from "botlib";
import { Sequelize } from "sequelize-typescript";
import { Promise } from "bluebird";
import { GBConfigService } from "./GBConfigService";
import { DataTypeUUIDv1 } from "sequelize";
import { GBError, GBERROR_TYPE } from "botlib";

import { GBConversationalService } from "./GBConversationalService";
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

  /** Deploys a bot to the storage. */
  deployBot(localPath: string, cb: GBServiceCallback<any>) {
    let packageType = Path.extname(localPath);
    let packageName = Path.basename(localPath);

    this.importer.importIfNotExistsBotPackage(
      packageName,
      localPath,
      (data, err) => {
        if (err) {
          logger.trace(err);
        } else {
          cb(data, null);
        }
      }
    );
  }

  deployPackageToStorage(
    instanceId: number,
    packageName: string,
    cb: GBServiceCallback<GuaribasPackage>
  ) {
    GuaribasPackage.create({
      packageName: packageName,
      instanceId: instanceId
    }).then((item: GuaribasPackage) => {
      cb(item, null);
    });
  }

  deployTheme(localPath: string, cb: GBServiceCallback<any>) {
    // DISABLED: Until completed, "/ui/public".
    // FsExtra.copy(localPath, this.workDir + packageName)
    //   .then(() => {
    //     cb(null, null);
    //   })
    //   .catch(err => {
    //     var gberr = GBError.create(
    //       `GuaribasBusinessError: Error copying package: ${localPath}.`
    //     );
    //     cb(null, gberr);
    //   });
  }

  deployPackageFromLocalPath(localPath: string, cb: GBServiceCallback<any>) {
    let packageType = Path.extname(localPath);

    switch (packageType) {
      case ".gbot":
        this.deployBot(localPath, cb);
        break;

      case ".gbtheme":
        this.deployTheme(localPath, cb);
        break;

      // PACKAGE: Put in package logic.
      case ".gbkb":
        let service = new KBService();
        service.deployKb(this.core, this, localPath, cb);
        break;

      case ".gbui":
        break;

      default:
        var err = GBError.create(
          `GuaribasBusinessError: Unknow package type: ${packageType}.`
        );
        cb(null, err);
        break;
    }
  }

  undeployPackageFromLocalPath(
    instance: IGBInstance,
    localPath: string,
    cb: GBServiceCallback<any>
  ) {
    let packageType = Path.extname(localPath);
    let packageName = Path.basename(localPath);

    this.getPackageByName(instance.instanceId, packageName, (p, err) => {
      switch (packageType) {
        case ".gbot":
          // TODO: this.undeployBot(packageName, localPath, cb);
          break;

        case ".gbtheme":
          // TODO: this.undeployTheme(packageName, localPath, cb);
          break;

        case ".gbkb":
          let service = new KBService();
          service.undeployKbFromStorage(instance, p.packageId, cb);
          break;

        case ".gbui":
          break;

        default:
          var err = GBError.create(
            `GuaribasBusinessError: Unknow package type: ${packageType}.`
          );
          cb(null, err);
          break;
      }
    });
  }

  getPackageByName(
    instanceId: number,
    packageName: string,
    cb: GBServiceCallback<GuaribasPackage>
  ) {

    var where = { packageName: packageName, instanceId: instanceId };

    GuaribasPackage.findOne({
      where: where
    })
      .then((value: GuaribasPackage) => {
        cb(value, null);
      })
      .error(reason => {
        cb(null, reason);
      });
  }


  /**
   *
   * Hot deploy processing.
   *
   */
  scanBootPackage(cb: GBServiceCallback<boolean>) {

    const deployFolder = "deploy";
    let bootPackage = GBConfigService.get("BOOT_PACKAGE");

    if (bootPackage === "none") {
      cb(true, null);
    } else {
      this.deployPackageFromLocalPath(
        UrlJoin(deployFolder, bootPackage),
        (data, err) => {
          logger.trace(`Boot package deployed: ${bootPackage}`);
          if (err) logger.trace(err);
        }
      );
    }
  }
}
