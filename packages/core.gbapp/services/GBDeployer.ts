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

const logger = require('../../../src/logger');
const Path = require('path');
const UrlJoin = require('url-join');
const Fs = require('fs');
const WaitUntil = require('wait-until');
const express = require('express');

import { GBMinInstance, IGBCoreService, IGBInstance } from 'botlib';
import { GBError } from 'botlib';
import { IGBPackage } from 'botlib';
import { AzureSearch } from 'pragmatismo-io-framework';
import { AzureDeployerService } from '../../azuredeployer.gbapp/services/AzureDeployerService';
import { GuaribasInstance, GuaribasPackage } from '../models/GBModel';
import { KBService } from './../../kb.gbapp/services/KBService';
import { GBConfigService } from './GBConfigService';
import { GBImporter } from './GBImporterService';
import { GBVMService } from './GBVMService';

/** Deployer service for bots, themes, ai and more. */
export class GBDeployer {
  public static deployFolder = 'packages';
  public core: IGBCoreService;
  public importer: GBImporter;
  public workDir: string = './work';

  constructor(core: IGBCoreService, importer: GBImporter) {
    this.core = core;
    this.importer = importer;
  }

  public static getConnectionStringFromInstance(instance: GuaribasInstance) {
    return `Server=tcp:${
      instance.storageServer
    }.database.windows.net,1433;Database=${instance.storageName};User ID=${
      instance.storageUsername
    };Password=${
      instance.storagePassword
    };Trusted_Connection=False;Encrypt=True;Connection Timeout=30;`;
  }

  /**
   *
   * Performs package deployment in all .gbai or default.
   *
   * */
  public deployPackages(
    core: IGBCoreService,
    server: any,
    appPackages: IGBPackage[]
  ) {
    const _this = this;
    return new Promise(
      (resolve: any, reject: any): any => {
        let totalPackages = 0;
        const additionalPath = GBConfigService.get('ADDITIONAL_DEPLOY_PATH');
        let paths = [GBDeployer.deployFolder];
        if (additionalPath) {
          paths = paths.concat(additionalPath.toLowerCase().split(';'));
        }
        const botPackages = new Array<string>();
        const gbappPackages = new Array<string>();
        let generalPackages = new Array<string>();

        function doIt(path) {
          const isDirectory = source => Fs.lstatSync(source).isDirectory();
          const getDirectories = source =>
            Fs.readdirSync(source)
              .map(name => Path.join(source, name))
              .filter(isDirectory);

          const dirs = getDirectories(path);
          dirs.forEach(element => {
            if (element.startsWith('.')) {
              logger.info(`Ignoring ${element}...`);
            } else {
              if (element.endsWith('.gbot')) {
                botPackages.push(element);
              } else if (element.endsWith('.gbapp')) {
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
          if (!e.startsWith('packages')) {
            logger.info(`Deploying app: ${e}...`);
            import(e)
              .then(m => {
                const p = new m.Package();
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

            try {
              await core.syncDatabaseStructure();
            } catch (e) {
              throw e;
            }

            /** Deploys all .gbot files first. */

            botPackages.forEach(e => {
              if (e!=='packages\\boot.gbot'){
              logger.info(`Deploying bot: ${e}...`);
              _this.deployBot(e);
              logger.info(`Bot: ${e} deployed...`);
              }
            });

            /** Then all remaining generalPackages are loaded. */

            generalPackages = generalPackages.filter(p => !p.endsWith('.git'));

            generalPackages.forEach(filename => {
              const filenameOnly = Path.basename(filename);
              logger.info(`Deploying package: ${filename}...`);

              /** Handles apps for general bots - .gbapp must stay out of deploy folder. */

              if (
                Path.extname(filename) === '.gbapp' ||
                Path.extname(filename) === '.gblib'
              ) {
                /** Themes for bots. */
              } else if (Path.extname(filename) === '.gbtheme') {
                server.use('/themes/' + filenameOnly, express.static(filename));
                logger.info(
                  `Theme (.gbtheme) assets accessible at: ${'/themes/' +
                    filenameOnly}.`
                );

                /** Knowledge base for bots. */
              } else if (Path.extname(filename) === '.gbkb') {
                server.use(
                  '/kb/' + filenameOnly + '/subjects',
                  express.static(UrlJoin(filename, 'subjects'))
                );
                logger.info(
                  `KB (.gbkb) assets accessible at: ${'/kb/' + filenameOnly}.`
                );
              } else if (Path.extname(filename) === '.gbui') {
                // Already Handled
              } else if (Path.extname(filename) === '.gbdialog') {
                // Already Handled
              } else {
                /** Unknown package format. */
                const err = new Error(`Package type not handled: ${filename}.`);
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
                  logger.info(
                    'No external packages to load, please use ADDITIONAL_DEPLOY_PATH to point to a .gbai package folder.'
                  );
                } else {
                  logger.info(`Package deployment done.`);
                }
                resolve();
              });
          });
      }
    );
  }

  /**
   * Deploys a bot to the storage.
   */

  public async deployBot(localPath: string): Promise<IGBInstance> {
    const packageType = Path.extname(localPath);
    const packageName = Path.basename(localPath);
    const instance = await this.importer.importIfNotExistsBotPackage(null,
      packageName,
      localPath
    );

    return instance;
  }

  public async deployPackageToStorage(
    instanceId: number,
    packageName: string
  ): Promise<GuaribasPackage> {
    return GuaribasPackage.create({
      packageName: packageName,
      instanceId: instanceId
    });
  }

  public deployScriptToStorage(instanceId: number, localPath: string) {}

  public deployTheme(localPath: string) {
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

  public async deployPackageFromLocalPath(min: GBMinInstance, localPath: string) {
    const packageType = Path.extname(localPath);

    switch (packageType) {
      case '.gbot':
        return this.deployBot(localPath);

      case '.gbtheme':
        return this.deployTheme(localPath);

      // PACKAGE: Put in package logic.
      case '.gbkb':
        const service = new KBService(this.core.sequelize);
        return service.deployKb(this.core, this, localPath);

      case '.gbui':
        break;

      case '.gbdialog':
        const vm = new GBVMService();
        return vm.loadJS(localPath, min, this.core, this, localPath);

      default:
        const err = GBError.create(
          `GuaribasBusinessError: Unknown package type: ${packageType}.`
        );
        Promise.reject(err);
        break;
    }
  }

  public async undeployPackageFromLocalPath(
    instance: IGBInstance,
    localPath: string
  ) {
    const packageType = Path.extname(localPath);
    const packageName = Path.basename(localPath);

    const p = await this.getPackageByName(instance.instanceId, packageName);

    switch (packageType) {
      case '.gbot':
        // TODO: this.undeployBot(packageName, localPath)
        break;

      case '.gbtheme':
        // TODO: this.undeployTheme(packageName, localPath)
        break;

      case '.gbkb':
        const service = new KBService(this.core.sequelize);
        return service.undeployKbFromStorage(instance, this, p.packageId);

      case '.gbui':
        break;

      case '.gbdialog':
        break;

      default:
        const err = GBError.create(
          `GuaribasBusinessError: Unknown package type: ${packageType}.`
        );
        Promise.reject(err);
        break;
    }
  }

  public async rebuildIndex(instance: GuaribasInstance) {
    const search = new AzureSearch(
      instance.searchKey,
      instance.searchHost,
      instance.searchIndex,
      instance.searchIndexer
    );

    const connectionString = GBDeployer.getConnectionStringFromInstance(
      instance
    );

    const dsName = 'gb';
    try {
      await search.deleteDataSource(dsName);
    } catch (err) {
      if (err.code != 404) {
        // First time, nothing to delete.
        throw err;
      }
    }

    await search.createDataSource(
      dsName,
      dsName,
      'GuaribasQuestion',
      'azuresql',
      connectionString
    );

    try {
      await search.deleteIndex();
    } catch (err) {
      if (err.code != 404) {
        // First time, nothing to delete.
        throw err;
      }
    }
    await search.createIndex(
      AzureDeployerService.getKBSearchSchema(instance.searchIndex),
      dsName
    );
  }

  public async getPackageByName(
    instanceId: number,
    packageName: string
  ): Promise<GuaribasPackage> {
    const where = { packageName: packageName, instanceId: instanceId };
    return GuaribasPackage.findOne({
      where: where
    });
  }
}
