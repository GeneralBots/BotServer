/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
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

const Path = require('path');
import urlJoin = require('url-join');
const Fs = require('fs');
const WaitUntil = require('wait-until');
const express = require('express');
const child_process = require('child_process');
const graph = require('@microsoft/microsoft-graph-client');
const rimraf = require('rimraf');

import { GBError, GBLog, GBMinInstance, IGBCoreService, IGBInstance, IGBPackage } from 'botlib';
import { AzureSearch } from 'pragmatismo-io-framework';
import { GBServer } from '../../../src/app';
import { GuaribasPackage } from '../models/GBModel';
import { GBAdminService } from './../../admin.gbapp/services/GBAdminService';
import { AzureDeployerService } from './../../azuredeployer.gbapp/services/AzureDeployerService';
import { KBService } from './../../kb.gbapp/services/KBService';
import { GBConfigService } from './GBConfigService';
import { GBImporter } from './GBImporterService';
import { GBVMService } from './GBVMService';
import { min } from 'moment';
import { GBMinService } from './GBMinService';

/**
 *
 * Deployer service for bots, themes, ai and more.
 */

export class GBDeployer {
  public static deployFolder = 'packages';
  public static workFolder = 'work';
  public core: IGBCoreService;
  public importer: GBImporter;

  constructor(core: IGBCoreService, importer: GBImporter) {
    this.core = core;
    this.importer = importer;
  }

  public static getConnectionStringFromInstance(instance: IGBInstance) {
    return `Server=tcp:${instance.storageServer}.database.windows.net,1433;Database=${
      instance.storageName
      };User ID=${
      instance.storageUsername
      };Password=${
      instance.storagePassword
      };Trusted_Connection=False;Encrypt=True;Connection Timeout=30;`;
  }

  /**
   *
   * Performs package deployment in all .gbai or default.
   *
   */
  public async deployPackages(core: IGBCoreService, server: any, appPackages: IGBPackage[]) {
    const _this = this;

    return new Promise(
      (resolve: any, reject: any): any => {
        GBLog.info(`PWD ${process.env.PWD}...`);
        let totalPackages = 0;
        let paths = [urlJoin(process.env.PWD, GBDeployer.deployFolder), urlJoin(process.env.PWD, GBDeployer.workFolder)];
        const additionalPath = GBConfigService.get('ADDITIONAL_DEPLOY_PATH');
        if (additionalPath !== undefined && additionalPath !== '') {
          paths = paths.concat(additionalPath.toLowerCase().split(';'));
        }
        const botPackages: string[] = [];
        const gbappPackages: string[] = [];
        let generalPackages: string[] = [];

        function doIt(path) {
          const isDirectory = source => Fs.lstatSync(source).isDirectory();
          const getDirectories = source =>
            Fs.readdirSync(source)
              .map(name => Path.join(source, name))
              .filter(isDirectory);

          const dirs = getDirectories(path);
          dirs.forEach(element => {
            if (element.startsWith('.')) {
              GBLog.info(`Ignoring ${element}...`);
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

        GBLog.info(`Starting looking for packages (.gbot, .gbtheme, .gbkb, .gbapp)...`);
        paths.forEach(e => {
          GBLog.info(`Looking in: ${e}...`);
          doIt(e);
        });

        // Deploys all .gbapp files first.

        const appPackagesProcessed = this.deployAppPackages(gbappPackages, core, appPackages);

        WaitUntil()
          .interval(1000)
          .times(5)
          .condition(cb => {
            GBLog.info(`Waiting for app package deployment...`);
            cb(appPackagesProcessed === gbappPackages.length);
          })
          .done(async () => {
            GBLog.info(`App Package deployment done.`);

            ({ generalPackages, totalPackages } = await this.deployDataPackages(

              core,
              botPackages,
              _this,
              generalPackages,
              server,
              reject,
              totalPackages,
              resolve
            ));
          });
      }
    );
  }

  /**
   * Deploys a bot to the storage.
   */

  public async deployBot(localPath: string, publicAddress: string): Promise<void> {
    const packageName = Path.basename(localPath);

    const service = new AzureDeployerService(this);
    let instance = await this.importer.importIfNotExistsBotPackage(undefined, packageName, localPath);

    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const group = GBConfigService.get('CLOUD_GROUP');
    const subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');
    const accessToken = await GBAdminService.getADALTokenFromUsername(username, password);

    if (await service.botExists(instance.botId, group)) {
      await service.updateBot(
        instance.botId,
        group,
        instance.title,
        instance.description,
        `${publicAddress}/api/messages/${instance.botId}`
      );

    } else {
      
      let botId = GBConfigService.get('BOT_ID');
      let bootInstance = await this.core.loadInstance(botId);

      instance.searchHost = bootInstance.searchHost;
      instance.searchIndex = bootInstance.searchIndex;
      instance.searchIndexer = bootInstance.searchIndexer;
      instance.searchKey = bootInstance.searchKey;
      instance.whatsappServiceKey = bootInstance.whatsappServiceKey;
      instance.whatsappBotKey = bootInstance.whatsappBotKey;
      instance.whatsappServiceNumber = bootInstance.whatsappServiceNumber;
      instance.whatsappServiceUrl = bootInstance.whatsappServiceUrl;
      instance.whatsappServiceWebhookUrl = bootInstance.whatsappServiceWebhookUrl;
             
      instance = await service.internalDeployBot(
        instance,
        accessToken,
        instance.botId,
        instance.title,
        group,
        instance.description,
        `${publicAddress}/api/messages/${instance.botId}`,
        'global',
        instance.nlpAppId,
        instance.nlpKey,
        instance.marketplaceId,
        instance.marketplacePassword,
        subscriptionId
      );

      await GBServer.globals.minService.mountBot(instance);
    }
    await this.core.saveInstance(instance);
    
  }


  /**
   * UndDeploys a bot to the storage.
   */

  public async undeployBot(botId: string, packageName: string): Promise<void> {
    const service = new AzureDeployerService(this);

    const group = GBConfigService.get('CLOUD_GROUP');

    if (await service.botExists(botId, group)) {

      await service.deleteBot(
        botId, group
      );

    }
    GBServer.globals.minService.unmountBot(botId);
    await this.core.deleteInstance(botId);
    const packageFolder =Path.join(process.env.PWD, 'work', packageName);
    rimraf.sync(packageFolder);
  }

  public async undeployTheme(packageName: string): Promise<void> {
    const packageFolder = Path.join(process.env.PWD, 'work', packageName);
    rimraf.sync(packageFolder);
  }

  public async deployPackageToStorage(instanceId: number, packageName: string): Promise<GuaribasPackage> {
    return GuaribasPackage.create({
      packageName: packageName,
      instanceId: instanceId
    });
  }

  public async deployFromSharePoint(instanceId: number) {
    const adminService = new GBAdminService(this.core);
    const accessToken = adminService.acquireElevatedToken(instanceId);

    // Initialize Graph client.

    const client = graph.Client.init({
      authProvider: done => {
        done(undefined, accessToken);
      }
    });
  }

  public async deployPackage(min: GBMinInstance, localPath: string) {
    const packageType = Path.extname(localPath);

    switch (packageType) {
      case '.gbot':
        await this.deployBot(localPath, GBServer.globals.publicAddress);
        break;

      case '.gbkb':
        const service = new KBService(this.core.sequelize);
        await service.deployKb(this.core, this, localPath);
        break;

      case '.gbdialog':
        const vm = new GBVMService();
        await vm.loadDialogPackage(localPath, min, this.core, this);
        break;
        
      default:
        const err = GBError.create(`Unhandled package type: ${packageType}.`);
        Promise.reject(err);
        break;
    }
  }

  public async undeployPackageFromLocalPath(instance: IGBInstance, localPath: string) {
    const packageType = Path.extname(localPath);
    const packageName = Path.basename(localPath);

    const p = await this.getPackageByName(instance.instanceId, packageName);


    switch (packageType) {
      case '.gbot':
        const packageObject = JSON.parse(Fs.readFileSync(urlJoin(localPath, 'package.json'), 'utf8'));
        await this.undeployBot(packageObject.botId, packageName);
        break;

      case '.gbkb':
        const service = new KBService(this.core.sequelize);
        return service.undeployKbFromStorage(instance, this, p.packageId);

      case '.gbui':

        break;

      case '.gbtheme':
        this.undeployTheme(packageName);
        break;

      case '.gbdialog':

        break;

      default:
        const err = GBError.create(`Unhandled package type: ${packageType}.`);
        Promise.reject(err);
        break;
    }
  }

  public async rebuildIndex(instance: IGBInstance, searchSchema: any) {
    const search = new AzureSearch(
      instance.searchKey,
      instance.searchHost,
      instance.searchIndex,
      instance.searchIndexer
    );

    const connectionString = GBDeployer.getConnectionStringFromInstance(instance);

    const dsName = 'gb';
    try {
      await search.deleteDataSource(dsName);
    } catch (err) {
      if (err.code !== 404) {
        // First time, nothing to delete.
        throw err;
      }
    }

    await search.createDataSource(dsName, dsName, 'GuaribasQuestion', 'azuresql', connectionString);

    try {
      await search.deleteIndex();
    } catch (err) {
      if (err.code !== 404) {
        // First time, nothing to delete.
        throw err;
      }
    }
    await search.createIndex(searchSchema, dsName);
  }

  public async getPackageByName(instanceId: number, packageName: string): Promise<GuaribasPackage> {
    const where = { packageName: packageName, instanceId: instanceId };

    return GuaribasPackage.findOne({
      where: where
    });
  }

  public runOnce() {
    const root = 'packages/default.gbui';
    const npm = urlJoin(process.env.PWD, 'node_modules', '.bin', 'npm');
    if (!Fs.existsSync(`${root}/build`)) {
      GBLog.info(`Preparing default.gbui (it may take some additional time for the first time)...`);
      Fs.writeFileSync(`${root}/.env`, 'SKIP_PREFLIGHT_CHECK=true');
      child_process.execSync(`${npm} install`, { cwd: root });
      child_process.execSync(`${npm} run build`, { cwd: root });
    }
  }

  private async deployDataPackages(

    core: IGBCoreService,
    botPackages: string[],
    _this: this,
    generalPackages: string[],
    server: any,
    reject: any,
    totalPackages: number,
    resolve: any
  ) {
    try {
      await core.syncDatabaseStructure();
    } catch (e) {
      throw e;
    }

    // Deploys all .gbot files first.

    botPackages.forEach(async (e) => {
      if (e !== 'packages\\boot.gbot') {
        GBLog.info(`Deploying bot: ${e}...`);
        await _this.deployBot(e, GBServer.globals.publicAddress);
        GBLog.info(`Bot: ${e} deployed...`);
      }
    });

    // Then all remaining generalPackages are loaded.

    generalPackages = generalPackages.filter(p => !p.endsWith('.git'));
    generalPackages.forEach(filename => {
      const filenameOnly = Path.basename(filename);
      GBLog.info(`Deploying package: ${filename}...`);

      // Handles apps for general bots - .gbapp must stay out of deploy folder.

      if (Path.extname(filename) === '.gbapp' || Path.extname(filename) === '.gblib') {
        // Themes for bots.
      } else if (Path.extname(filename) === '.gbtheme') {
        server.use(`/themes/${filenameOnly}`, express.static(filename));
        GBLog.info(`Theme (.gbtheme) assets accessible at: /themes/${filenameOnly}.`);
      } else if (Path.extname(filename) === '.gbkb') {
        server.use(`/kb/${filenameOnly}/subjects`, express.static(urlJoin(filename, 'subjects')));
        server.use(`/kb/${filenameOnly}/images`, express.static(urlJoin(filename, 'images')));
        GBLog.info(`KB (.gbkb) assets accessible at: /kb/${filenameOnly}.`);
      } else if (Path.extname(filename) === '.gbui') {
        // Already Handled
      } else if (Path.extname(filename) === '.gbdialog') {
        // Already Handled
      } else if (Path.extname(filename) === '.gbignore') {
        // Ignored
      } else {
        // Unknown package format.
        const err = new Error(`Package type not handled: ${filename}.`);
        reject(err);
      }
      totalPackages++;
    });

    WaitUntil()
      .interval(100)
      .times(5)
      .condition(cb => {
        GBLog.info(`Waiting for package deployment...`);
        cb(totalPackages === generalPackages.length);
      })
      .done(() => {
        if (botPackages.length === 0) {
          GBLog.info('Use ADDITIONAL_DEPLOY_PATH to point to a .gbai package folder (no external packages).');
        } else {
          GBLog.info(`Package deployment done.`);
        }
        resolve();
      });

    return { generalPackages, totalPackages };
  }

  private isSystemPackage(name: string): Boolean {
    const names = ['core.gbapp', 'admin.gbapp', 'azuredeployer.gbapp', 'customer-satisfaction.gbapp', 'kb.gbapp'];

    return names.indexOf(name) > -1;
  }

  private deployAppPackages(gbappPackages: string[], core: any, appPackages: any[]) {
    let appPackagesProcessed = 0;
    gbappPackages.forEach(e => {
      const filenameOnly = Path.basename(e);

      // Skips .gbapp inside deploy folder.
      if (this.isSystemPackage(filenameOnly) === false) {

        GBLog.info(`Deploying app: ${e}...`);
        let folder = Path.join(e, 'node_modules');
        if (!Fs.existsSync(folder)) {
          GBLog.info(`Installing modules for ${e}...`);
          child_process.execSync('npm install', { cwd: e });
        }

        folder = Path.join(e, 'dist');
        if (!Fs.existsSync()) {
          GBLog.info(`Compiling ${e}...`);

          try {
            child_process.execSync(Path.join(process.env.PWD, 'node_modules/.bin/tsc'), { cwd: e });
            import(e)
              .then(m => {
                const p = new m.Package();
                p.loadPackage(core, core.sequelize);
                appPackages.push(p);
                GBLog.info(`App (.gbapp) deployed: ${e}.`);
                appPackagesProcessed++;
              })
              .catch(err => {
                GBLog.error(`Error deploying .gbapp package: ${e}\n${err}`);
                appPackagesProcessed++;
              });
          } catch (error) {
            GBLog.error(`Error compiling .gbapp package ${e}:\n${error.stdout.toString()}`);
            appPackagesProcessed++;
          }
        }
      } else {
        appPackagesProcessed++;
      }
    });

    return appPackagesProcessed;
  }
}
