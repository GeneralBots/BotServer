/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
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
const express = require('express');
const child_process = require('child_process');
const graph = require('@microsoft/microsoft-graph-client');
const rimraf = require('rimraf');

import { GBError, GBLog, GBMinInstance, IGBCoreService, IGBInstance, IGBPackage, IGBDeployer } from 'botlib';
import { AzureSearch } from 'pragmatismo-io-framework';
import { GBServer } from '../../../src/app';
import { GuaribasPackage } from '../models/GBModel';
import { GBAdminService } from './../../admin.gbapp/services/GBAdminService';
import { AzureDeployerService } from './../../azuredeployer.gbapp/services/AzureDeployerService';
import { KBService } from './../../kb.gbapp/services/KBService';
import { GBConfigService } from './GBConfigService';
import { GBImporter } from './GBImporterService';
import { GBVMService } from './GBVMService';
import { CollectionUtil } from 'pragmatismo-io-framework';
const MicrosoftGraph = require("@microsoft/microsoft-graph-client");

/**
 *
 * Deployer service for bots, themes, ai and more.
 */

export class GBDeployer implements IGBDeployer {
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

    let paths = [urlJoin(process.env.PWD, GBDeployer.deployFolder), urlJoin(process.env.PWD, GBDeployer.workFolder)];
    const additionalPath = GBConfigService.get('ADDITIONAL_DEPLOY_PATH');
    if (additionalPath !== undefined && additionalPath !== '') {
      paths = paths.concat(additionalPath.toLowerCase().split(';'));
    }
    const botPackages: string[] = [];
    const gbappPackages: string[] = [];
    let generalPackages: string[] = [];

    async function scanPackageDirectory(path) {
      const isDirectory = source => Fs.lstatSync(source).isDirectory();
      const getDirectories = source =>
        Fs.readdirSync(source)
          .map(name => Path.join(source, name))
          .filter(isDirectory);

      const dirs = getDirectories(path);
      await CollectionUtil.asyncForEach(dirs, async element => {
        element = element.toLowerCase();
        if (element === '.') {
          GBLog.info(`Ignoring ${element}...`);
        } else {
          if (element.endsWith('.gbot')) {
            botPackages.push(element);
          } else if (element.endsWith('.gbapp') || element.endsWith('.gblib')) {
            gbappPackages.push(element);
          } else {
            generalPackages.push(element);
          }
        }
      });
    }

    GBLog.info(`Starting looking for packages (.gbot, .gbtheme, .gbkb, .gbapp)...`);
    await CollectionUtil.asyncForEach(paths, async e => {
      GBLog.info(`Looking in: ${e}...`);
      await scanPackageDirectory(e);
    });

    // Deploys all .gblib files first.

    let list = [];
    for (let index = 0; index < gbappPackages.length; index++) {
      const element = gbappPackages[index];
      if (element.endsWith('.gblib')) {
        list.push(element);
        gbappPackages.splice(index, 1);
      }
    }
    for (let index = 0; index < gbappPackages.length; index++) {
      const element = gbappPackages[index];
      list.push(element);
    }
    await this.deployAppPackages(list, core, appPackages);

    GBLog.info(`App Package deployment done.`);

    ({ generalPackages } = await this.deployDataPackages(

      core,
      botPackages,
      _this,
      generalPackages
    ));

  }

  public async deployBlankBot(botId: string) {
    let instance = await this.importer.createBotInstance(botId);

    const bootInstance = GBServer.globals.bootInstance;
    const accessToken = await GBServer.globals.minBoot.adminService
      .acquireElevatedToken(bootInstance.instanceId);

    const service = new AzureDeployerService(this);
    let application = await service.createApplication(accessToken, botId);

    instance.marketplaceId = (application as any).appId;
    instance.marketplacePassword = await service.createApplicationSecret(
      accessToken, (application as any).id);
    instance.adminPass = GBAdminService.getRndPassword();
    instance.title = botId;
    instance.activationCode = instance.botId;
    instance.state = 'active';
    instance.nlpScore = 0.80; // TODO: Migrate to Excel Config.xlsx. 
    instance.searchScore = 0.45;
    instance.whatsappServiceKey = bootInstance.whatsappServiceKey;
    instance.whatsappServiceNumber = bootInstance.whatsappServiceNumber;
    instance.whatsappServiceUrl = bootInstance.whatsappServiceUrl;

    await this.core.saveInstance(instance);

    return await this.deployBotFull(instance, GBServer.globals.publicAddress);
  }

  public async botExists(botId: string): Promise<boolean> {
    const service = new AzureDeployerService(this);
    return await service.botExists(botId);
  }
  /**
   * Deploys a bot to the storage.
   */

  public async deployBotFull(instance: IGBInstance, publicAddress: string): Promise<IGBInstance> {

    const service = new AzureDeployerService(this);
    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const accessToken = await GBAdminService.getADALTokenFromUsername(username, password);
    const group = GBConfigService.get('CLOUD_GROUP');
    const subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');

    if (await service.botExists(instance.botId)) {
      await service.updateBot(
        instance.botId,
        group,
        instance.title,
        instance.description,
        `${publicAddress}/api/messages/${instance.botId}`
      );

    } else {

      let botId = GBConfigService.get('BOT_ID');
      let bootInstance = await this.core.loadInstanceByBotId(botId);

      instance.searchHost = bootInstance.searchHost;
      instance.searchIndex = bootInstance.searchIndex;
      instance.searchIndexer = bootInstance.searchIndexer;
      instance.searchKey = bootInstance.searchKey;
      instance.whatsappServiceKey = bootInstance.whatsappServiceKey;
      instance.whatsappServiceNumber = bootInstance.whatsappServiceNumber;
      instance.whatsappServiceUrl = bootInstance.whatsappServiceUrl;
      instance.storageServer = bootInstance.storageServer;
      instance.storageName = bootInstance.storageName;
      instance.storageUsername = bootInstance.storageUsername;
      instance.storagePassword = bootInstance.storagePassword;
      instance.cloudLocation = bootInstance.cloudLocation;
      instance.speechEndpoint = bootInstance.speechEndpoint;
      instance.speechKey = bootInstance.speechKey;

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
    return await this.core.saveInstance(instance);

  }

  /**
   * Deploys a bot to the storage from a .gbot folder.
   */

  public async deployBotFromLocalPath(localPath: string, publicAddress: string): Promise<void> {

    const packageName = Path.basename(localPath);
    let instance = await this.importer.importIfNotExistsBotPackage(undefined, packageName, localPath);
    this.deployBotFull(instance, publicAddress);
  }

  public async loadParamsFromExcel(min: GBMinInstance): Promise<any> {

    let token =
      await min.adminService.acquireElevatedToken(min.instance.instanceId);

    let siteId = process.env.STORAGE_SITE_ID;
    let libraryId = process.env.STORAGE_LIBRARY;

    let client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });
    const botId = min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbot`;

    let res = await client.api(
      `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/root:${path}:/children`)
      .get();


    // Performs validation.

    let document = res.value.filter(m => {
      return m.name === "Config.xlsx"
    });
    if (document === undefined || document.length === 0) {
      GBLog.info(`Config.xlsx not found on .bot folder, check the package.`);
      return null;
    }

    // Creates workbook session that will be discarded.

    let results = await client.api(
      `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/items/${document[0].id}/workbook/worksheets('General')/range(address='A7:B100')`)
      .get();

    let index = 0;
    let obj = {};
    for (; index < results.text.length; index++) {
      if (results.text[index][0] === "") {
        return obj;
      }
      obj[results.text[index][0]] = results.text[index][1];
    }
    return obj;
  }


  /**
   * UndDeploys a bot to the storage.
   */

  public async undeployBot(botId: string, packageName: string): Promise<void> {
    const service = new AzureDeployerService(this);

    const group = GBConfigService.get('CLOUD_GROUP');

    if (await service.botExists(botId)) {

      await service.deleteBot(
        botId, group
      );

    }
    GBServer.globals.minService.unmountBot(botId);
    await this.core.deleteInstance(botId);
    const packageFolder = Path.join(process.env.PWD, 'work', `${botId}.gbai`, packageName);
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

    // TODO: Today a download only approach is used. 
  }

  public async deployPackage(min: GBMinInstance, localPath: string) {
    const packageType = Path.extname(localPath);

    const _this = this;
    let handled = false;
    let pck = null;

    // .gbapp package or platform package checking.

    await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
      if (pck = await e.onExchangeData(min, "handlePackage", {
        name: localPath,
        createPackage: async (packageName) => {
          return await _this.deployPackageToStorage(min.instance.instanceId, packageName);
        }, updatePackage: async (p: GuaribasPackage) => {
          p.save();
        }
      })) {
        handled = true;
      }
    });

    if (handled) {

      return pck;
    }

    // Deploy platform packages here.

    switch (packageType) {
      case '.gbot':
        if (Fs.existsSync(localPath)) {
          await this.deployBotFromLocalPath(localPath, GBServer.globals.publicAddress);
        }
        min.instance.params = await this.loadParamsFromExcel(min);
        await this.core.saveInstance(min.instance);

        break;

      case '.gbkb':
        const service = new KBService(this.core.sequelize);
        await service.deployKb(this.core, this, localPath, min);
        break;

      case '.gbdialog':
        const vm = new GBVMService();
        await vm.loadDialogPackage(localPath, min, this.core, this);
        break;

      case '.gbtheme':
        const packageName = Path.basename(localPath);
        GBServer.globals.server.use(`/themes/${packageName}`, express.static(localPath));
        GBLog.info(`Theme (.gbtheme) assets accessible at: /themes/${packageName}.`);

        break;

      case '.gbapp':
        await this.callGBAppCompiler(localPath, this.core);
        break;

      case '.gblib':
        await this.callGBAppCompiler(localPath, this.core);
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

    const p = await this.getStoragePackageByName(instance.instanceId, packageName);

    switch (packageType) {
      case '.gbot':
        const packageObject = JSON.parse(Fs.readFileSync(urlJoin(localPath, 'package.json'), 'utf8'));
        await this.undeployBot(packageObject.botId, packageName);
        break;

      case '.gbkb':
        const service = new KBService(this.core.sequelize);
        rimraf.sync(localPath);
        return await service.undeployKbFromStorage(instance, this, p.packageId);

      case '.gbui':

        break;

      case '.gbtheme':
        rimraf.sync(localPath);
        break;

      case '.gbdialog':
        rimraf.sync(localPath);
        break;

      case '.gblib':

        break;
      case '.gbapp':

        break;
      default:
        const err = GBError.create(`Unhandled package type: ${packageType}.`);
        Promise.reject(err);
        break;
    }
    rimraf.sync(localPath);
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

    // TODO: Use temporary names for index for exchanging them after the new one is created.

    try {
      await search.deleteIndex();
    } catch (err) {
      if (err.code !== 404) {
        // First time, nothing to delete.
        throw err;
      }
    }

    try {
      await search.createDataSource(dsName, dsName, 'GuaribasQuestion', 'azuresql', connectionString);
    } catch (err) {
      GBLog.error(err);
      throw err;
    }

    await search.createIndex(searchSchema, dsName);
  }

  public async getStoragePackageByName(instanceId: number, packageName: string): Promise<GuaribasPackage> {
    const where = { packageName: packageName, instanceId: instanceId };

    return await GuaribasPackage.findOne({
      where: where
    });
  }

  public setupDefaultGBUI() {
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
    generalPackages: string[]
  ) {
    try {
      await core.syncDatabaseStructure();
    } catch (e) {
      throw e;
    }

    // Deploys all .gbot files first.

    await CollectionUtil.asyncForEach(botPackages, async e => {
      if (e !== 'packages\\boot.gbot') {
        GBLog.info(`Deploying bot: ${e}...`);
        await _this.deployBotFromLocalPath(e, GBServer.globals.publicAddress);
        GBLog.info(`√ Bot: ${e} deployed...`);
      }
    });

    // Then all remaining generalPackages are loaded.

    const instances = core.loadInstances();
    await CollectionUtil.asyncForEach(instances, async instance => {

      this.mountGBKBAssets(`{instance.botId}.gbkb`, instance.botId, `{instance.botId}.gbkb`);
    });

    GBLog.info(`Package deployment done.`);
    return { generalPackages};
  }

  public mountGBKBAssets(packageName: any, botId: string, filename: string) {
    GBServer.globals.server.use(`/kb/${botId}.gbai/${packageName}/subjects`, express.static(urlJoin(filename, 'subjects')));
    GBServer.globals.server.use(`/kb/${botId}.gbai/${packageName}/assets`, express.static(urlJoin(filename, 'assets')));
    GBServer.globals.server.use(`/kb/${botId}.gbai/${packageName}/images`, express.static(urlJoin(filename, 'images')));
    GBServer.globals.server.use(`/kb/${botId}.gbai/${packageName}/audios`, express.static(urlJoin(filename, 'audios')));
    GBServer.globals.server.use(`/kb/${botId}.gbai/${packageName}/videos`, express.static(urlJoin(filename, 'videos')));
    GBLog.info(`KB (.gbkb) assets accessible at: /kb/${botId}.gbai/${packageName}.`);
  }

  private isSystemPackage(name: string): Boolean {
    const names = ['analytics.gblib', 'console.gblib', 'security.gbapp', 'whatsapp.gblib', 'sharepoint.gblib', 'core.gbapp', 'admin.gbapp', 'azuredeployer.gbapp', 'customer-satisfaction.gbapp', 'kb.gbapp'];

    return names.indexOf(name) > -1;
  }

  private async deployAppPackages(gbappPackages: string[], core: any, appPackages: any[]) {
    let appPackagesProcessed = 0;
    await CollectionUtil.asyncForEach(gbappPackages, async e => {
      const filenameOnly = Path.basename(e);

      // Skips .gbapp inside deploy folder.
      if (this.isSystemPackage(filenameOnly) === false) {

        appPackagesProcessed = await this.callGBAppCompiler(e, core, appPackages, appPackagesProcessed);
      }
    });

    return appPackagesProcessed;
  }

  public async callGBAppCompiler(gbappPath: string, core: IGBCoreService,
    appPackages: any[] = undefined, appPackagesProcessed: number = 0) {
    GBLog.info(`Deploying General Bots Application (.gbapp) or Library (.gblib): ${Path.basename(gbappPath)}...`);
    let folder = Path.join(gbappPath, 'node_modules');
    if (process.env.GBAPP_DISABLE_COMPILE !== "true") {
      if (!Fs.existsSync(folder)) {
        GBLog.info(`Installing modules for ${gbappPath}...`);
        child_process.execSync('npm install', { cwd: gbappPath });
      }
    }
    folder = Path.join(gbappPath, 'dist');
    try {
      if (process.env.GBAPP_DISABLE_COMPILE !== "true") {
        GBLog.info(`Compiling: ${gbappPath}.`);
        child_process.execSync(Path.join(process.env.PWD, 'node_modules/.bin/tsc'), { cwd: gbappPath });
      }

      if (gbappPath.endsWith('.gbapp')) {
        const m = await import(gbappPath);
        const p = new m.Package();
        await p.loadPackage(core, core.sequelize);
        if (appPackages !== undefined) {
          appPackages.push(p);
        }
      }

      GBLog.info(`.gbapp or .gblib deployed: ${gbappPath}.`);
      appPackagesProcessed++;
    }
    catch (error) {
      GBLog.error(`Error compiling package, message:  ${error.message}\n${error.stack}`);
      if (error.stdout) {
        GBLog.error(`Error compiling package, stdout: ${gbappPath}:\n${error.stdout.toString()}`);
      }
      appPackagesProcessed++;
    }
    return appPackagesProcessed;
  }
}
