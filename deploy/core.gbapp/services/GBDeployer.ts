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

const logger = require("../../../src/logger");
const Path = require("path");
const UrlJoin = require("url-join");
const Fs = require("fs");
const WaitUntil = require("wait-until");
const express = require("express");

import { KBService } from "./../../kb.gbapp/services/KBService";
import { GBImporter } from "./GBImporter";
import { IGBCoreService, IGBInstance } from "botlib";
import { GBConfigService } from "./GBConfigService";
import { GBError } from "botlib";
import { GuaribasPackage, GuaribasInstance } from "../models/GBModel";
import { IGBPackage } from "botlib";
import { AzureSearch } from "pragmatismo-io-framework";

/** Deployer service for bots, themes, ai and more. */
export class GBDeployer {
  core: IGBCoreService;

  importer: GBImporter;

  workDir: string = "./work";

  static deployFolder = "deploy";

  constructor(core: IGBCoreService, importer: GBImporter) {
    this.core = core;
    this.importer = importer;
  }

  /**
   *
   * Performs package deployment in all .gbai or default.
   *
   * */
  public deployPackages(
    core: IGBCoreService,
    server: any,
    appPackages: Array<IGBPackage>
  ) {
    let _this = this;
    return new Promise((resolve, reject) => {
      try {
        let totalPackages = 0;
        let additionalPath = GBConfigService.get("ADDITIONAL_DEPLOY_PATH");
        let paths = [GBDeployer.deployFolder];
        if (additionalPath) {
          paths = paths.concat(additionalPath.toLowerCase().split(";"));
        }
        let botPackages = new Array<string>();
        let gbappPackages = new Array<string>();
        let generalPackages = new Array<string>();

        function doIt(path) {
          const isDirectory = source => Fs.lstatSync(source).isDirectory();
          const getDirectories = source =>
            Fs.readdirSync(source)
              .map(name => Path.join(source, name))
              .filter(isDirectory);

          let dirs = getDirectories(path);
          dirs.forEach(element => {
            if (element.startsWith(".")) {
              logger.info(`Ignoring ${element}...`);
            } else {
              if (element.endsWith(".gbot")) {
                botPackages.push(element);
              } else if (element.endsWith(".gbapp")) {
                gbappPackages.push(element);
              } else {
                generalPackages.push(element);
              }
            }
          });
        }

        logger.info(
          `Starting looking for packages (.gbot, .gbtheme, .gbkb, .gbapp)...`
        );
        paths.forEach(e => {
          logger.info(`Looking in: ${e}...`);
          doIt(e);
        });

        /** Deploys all .gbapp files first. */

        let appPackagesProcessed = 0;

        gbappPackages.forEach(e => {
          // Skips .gbapp inside deploy folder.
          if (!e.startsWith("deploy")) {
            logger.info(`Deploying app: ${e}...`);
            import(e)
              .then(m => {
                let p = new m.Package();
                p.loadPackage(core, core.sequelize);
                appPackages.push(p);
                logger.info(`App (.gbapp) deployed: ${e}.`);
                appPackagesProcessed++;
              })
              .catch(err => {
                logger.error(`Error deploying App (.gbapp): ${e}: ${err}`);
                appPackagesProcessed++;
              });
          } else {
            appPackagesProcessed++;
          }
        });

        WaitUntil()
          .interval(1000)
          .times(10)
          .condition(function(cb) {
            logger.info(`Waiting for app package deployment...`);
            cb(appPackagesProcessed == gbappPackages.length);
          })
          .done(async result => {
            logger.info(`App Package deployment done.`);

            try{
              await core.syncDatabaseStructure();
            }catch(e){
              throw e;
            }

            /** Deploys all .gbot files first. */

            botPackages.forEach(e => {
              logger.info(`Deploying bot: ${e}...`);
              _this.deployBot(e);
              logger.info(`Bot: ${e} deployed...`);
            });

            /** Then all remaining generalPackages are loaded. */

            generalPackages = generalPackages.filter(p => !p.endsWith(".git"));

            generalPackages.forEach(filename => {
              let filenameOnly = Path.basename(filename);
              logger.info(`Deploying package: ${filename}...`);

              /** Handles apps for general bots - .gbapp must stay out of deploy folder. */

              if (
                Path.extname(filename) === ".gbapp" ||
                Path.extname(filename) === ".gblib"
              ) {
                /** Themes for bots. */
              } else if (Path.extname(filename) === ".gbtheme") {
                server.use("/themes/" + filenameOnly, express.static(filename));
                logger.info(
                  `Theme (.gbtheme) assets accessible at: ${"/themes/" +
                    filenameOnly}.`
                );

                /** Knowledge base for bots. */
              } else if (Path.extname(filename) === ".gbkb") {
                server.use(
                  "/kb/" + filenameOnly + "/subjects",
                  express.static(UrlJoin(filename, "subjects"))
                );
                logger.info(
                  `KB (.gbkb) assets accessible at: ${"/kb/" + filenameOnly}.`
                );
              } else if (Path.extname(filename) === ".gbui") {
                // Already Handled
              } else {
                /** Unknown package format. */
                let err = new Error(`Package type not handled: ${filename}.`);
                reject(err);
              }
              totalPackages++;
            });

            WaitUntil()
              .interval(100)
              .times(5)
              .condition(function(cb) {
                logger.info(`Waiting for package deployment...`);
                cb(totalPackages == generalPackages.length);
              })
              .done(function(result) {
                if (botPackages.length === 0) {
                  logger.warn(
                    "The server is running with no bot instances, at least one .gbot file must be deployed."
                  );
                } else {
                  logger.info(`Package deployment done.`);
                }
                resolve();
              });
          });
      } catch (err) {
        logger.error(err);
        reject(err);
      }
    });
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
    packageName: string
  ): Promise<GuaribasPackage> {
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
    //     )
    //   })
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
        let service = new KBService(this.core.sequelize);
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

  async undeployPackageFromLocalPath(instance: IGBInstance, localPath: string) {
    let packageType = Path.extname(localPath);
    let packageName = Path.basename(localPath);

    let p = await this.getPackageByName(instance.instanceId, packageName);

    switch (packageType) {
      case ".gbot":
        // TODO: this.undeployBot(packageName, localPath)
        break;

      case ".gbtheme":
        // TODO: this.undeployTheme(packageName, localPath)
        break;

      case ".gbkb":
        let service = new KBService(this.core.sequelize);
        return service.undeployKbFromStorage(instance, this, p.packageId);

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

  public async rebuildIndex(instance: GuaribasInstance) {
    let search = new AzureSearch(
      instance.searchKey,
      instance.searchHost,
      instance.searchIndex,
      instance.searchIndexer
    );
    await search.deleteIndex();
    let kbService = new KBService(this.core.sequelize);
    await search.createIndex(
      kbService.getSearchSchema(instance.searchIndex),
      "gb"
    );
  }

  async getPackageByName(
    instanceId: number,
    packageName: string
  ): Promise<GuaribasPackage> {
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
