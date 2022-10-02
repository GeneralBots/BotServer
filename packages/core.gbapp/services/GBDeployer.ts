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
const urlJoin = require('url-join');
const Fs = require('fs');
const express = require('express');
const child_process = require('child_process');
const rimraf = require('rimraf');
const request = require('request-promise-native');
const  vhost = require('vhost')
import { GBError, GBLog, GBMinInstance, IGBCoreService, IGBDeployer, IGBInstance, IGBPackage } from 'botlib';
import { AzureSearch } from 'pragmatismo-io-framework';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBServer } from '../../../src/app';
import { GBVMService } from '../../basic.gblib/services/GBVMService';
import { GuaribasPackage } from '../models/GBModel';
import { GBAdminService } from './../../admin.gbapp/services/GBAdminService';
import { AzureDeployerService } from './../../azuredeployer.gbapp/services/AzureDeployerService';
import { KBService } from './../../kb.gbapp/services/KBService';
import { GBConfigService } from './GBConfigService';
import { GBImporter } from './GBImporterService';
import { TeamsService } from '../../teams.gblib/services/TeamsService';
const MicrosoftGraph = require('@microsoft/microsoft-graph-client');


/**
 * Deployer service for bots, themes, ai and more.
 */
export class GBDeployer implements IGBDeployer {

  /**
   * Where should deployer look into for general packages.
   */
  public static deployFolder = 'packages';

  /**
   * The work folder used to download artifacts from bot storage.
   */
  public static workFolder = 'work';

  /**
   * Reference to the core service.
   */
  public core: IGBCoreService;

  /**
   * Reference to the importer service.
   */
  public importer: GBImporter;

  /**
   * Deployer needs core and importer to be created.
   */
  constructor(core: IGBCoreService, importer: GBImporter) {
    this.core = core;
    this.importer = importer;
  }

  /**
   * Builds a connection string text to be used in direct
   * use to database like the Indexer (Azure Search).
   */
  public static getConnectionStringFromInstance(instance: IGBInstance) {
    return `Server=tcp:${instance.storageServer},1433;Database=${instance.storageName};User ID=${instance.storageUsername};Password=${instance.storagePassword};Trusted_Connection=False;Encrypt=True;Connection Timeout=30;`;
  }

  /**
   * Retrives token and initialize drive client API.
   */
  public static async internalGetDriveClient(min: GBMinInstance) {
    let token = await min.adminService.acquireElevatedToken(min.instance.instanceId);
    let siteId = process.env.STORAGE_SITE_ID;
    let libraryId = process.env.STORAGE_LIBRARY;

    let client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });
    const baseUrl = `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}`;
    return [baseUrl, client];
  }

  /**
   * Performs package deployment in all .gbai or default.
   */
  public async deployPackages(core: IGBCoreService, server: any, appPackages: IGBPackage[]) {

    // Builds lists of paths to search for packages.

    let paths = [urlJoin(process.env.PWD, GBDeployer.deployFolder), urlJoin(process.env.PWD, GBDeployer.workFolder)];
    const additionalPath = GBConfigService.get('ADDITIONAL_DEPLOY_PATH');
    if (additionalPath !== undefined && additionalPath !== '') {
      paths = paths.concat(additionalPath.split(';'));
    }
    const botPackages: string[] = [];
    const gbappPackages: string[] = [];
    const generalPackages: string[] = [];

    async function scanPackageDirectory(path) {

      // Gets all directories.

      const isDirectory = source => Fs.lstatSync(source).isDirectory();
      const getDirectories = source =>
        Fs.readdirSync(source)
          .map(name => Path.join(source, name))
          .filter(isDirectory);
      const dirs = getDirectories(path);
      await CollectionUtil.asyncForEach(dirs, async element => {

        // For each folder, checks its extensions looking for valid packages.

        if (element === '.') {
          GBLog.info(`Ignoring ${element}...`);
        } else {
          const name = Path.basename(element);

          // Skips what does not need to be loaded.

          if (process.env.GBAPP_SKIP && (process.env.GBAPP_SKIP.toLowerCase().indexOf(name) !== -1
            || process.env.GBAPP_SKIP === 'true')
          ) {
            return;
          }

          // Put it in corresponding collections.

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

    // Start the process of searching.

    GBLog.info(`Starting looking for packages (.gbot, .gbtheme, .gbkb, .gbapp)...`);
    await CollectionUtil.asyncForEach(paths, async e => {
      GBLog.info(`Looking in: ${e}...`);
      await scanPackageDirectory(e);
    });

    // Deploys all .gblib files first.

    const list = [];
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
  }

  /**
   * Deploys a new blank bot to the database, cognitive services and other services.
   */
  public async deployBlankBot(botId: string, mobile: string, email: string) {

    // Creates a new row on the GuaribasInstance table.

    const instance = await this.importer.createBotInstance(botId);
    const bootInstance = GBServer.globals.bootInstance;

    // Gets the access token to perform service operations.

    const accessToken = await GBServer.globals.minBoot.adminService.acquireElevatedToken(bootInstance.instanceId);

    // Creates the MSFT application that will be associated to the bot.

    const service = new AzureDeployerService(this);
    const application = await service.createApplication(accessToken, botId);

    // Fills new instance base information and get App secret.

    instance.marketplaceId = (application as any).appId;
    instance.marketplacePassword = await service.createApplicationSecret(accessToken, (application as any).id);
    instance.adminPass = GBAdminService.getRndPassword();
    instance.title = botId;
    instance.activationCode = instance.botId;
    instance.state = 'active';
    instance.nlpScore = 0.8;
    instance.searchScore = 0.45;
    instance.whatsappServiceKey = bootInstance.whatsappServiceKey;
    instance.whatsappServiceNumber = bootInstance.whatsappServiceNumber;
    instance.whatsappServiceUrl = bootInstance.whatsappServiceUrl;
    instance.params = JSON.stringify({ 'Can Publish': mobile, 'Admin Notify E-mail': email });

    // Saves bot information to the store.

    await this.core.saveInstance(instance);

    // Creates remaining objects on the cloud and updates instance information.

    return await this.deployBotFull(instance, GBServer.globals.publicAddress);
  }

  /**
   * Verifies if bot exists on bot catalog.
   */
  public async botExists(botId: string): Promise<boolean> {
    const service = new AzureDeployerService(this);

    return await service.botExists(botId);
  }

  /**
   * Performs all tasks of deploying a new bot on the cloud.
   */
  public async deployBotFull(instance: IGBInstance, publicAddress: string): Promise<IGBInstance> {

    // Reads base configuration from environent file.

    const service = new AzureDeployerService(this);
    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const accessToken = await GBAdminService.getADALTokenFromUsername(username, password);
    const group = GBConfigService.get('CLOUD_GROUP');
    const subscriptionId = GBConfigService.get('CLOUD_SUBSCRIPTIONID');

    // If the bot already exists, just update the endpoint.

    if (await service.botExists(instance.botId)) {
      await service.updateBot(
        instance.botId,
        group,
        instance.title,
        instance.description,
        `${publicAddress}/api/messages/${instance.botId}`
      );
    } else {
      const botId = GBConfigService.get('BOT_ID');
      const bootInstance = await this.core.loadInstanceByBotId(botId);

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

      // Internally create resources on cloud provider.

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

      // Makes available bot to the channels and .gbui interfaces.

      await GBServer.globals.minService.mountBot(instance);
    }

    // Saves final instance object and returns it.

    return await this.core.saveInstance(instance);
  }

  /**
   * Performs the NLP publishing process on remote service.
   */
  public async publishNLP(instance: IGBInstance): Promise<void> {
    const service = new AzureDeployerService(this);
    const res = await service.publishNLP(instance.cloudLocation, instance.nlpAppId,
      instance.nlpAuthoringKey);
    if (res.status !== 200 && res.status !== 201) { throw res.bodyAsText; }
  }

  /**
   * Trains NLP on the remote service.
   */
  public async trainNLP(instance: IGBInstance): Promise<void> {
    const service = new AzureDeployerService(this);
    const res = await service.trainNLP(instance.cloudLocation, instance.nlpAppId, instance.nlpAuthoringKey);
    if (res.status !== 200 && res.status !== 202) { throw res.bodyAsText; }
    const sleep = ms => {
      return new Promise(resolve => {
        setTimeout(resolve, ms);
      });
    };
    sleep(5000);
  }

  /**
   * Return a zip file for importing bot in apps, currently MS Teams.
   */
  public async getBotManifest(instance: IGBInstance): Promise<Buffer> {
    const s = new TeamsService();
    const manifest = await s.getManifest(instance.marketplaceId, instance.title, instance.description,
      GBAdminService.generateUuid(), instance.botId, "General Bots");

    return await s.getAppFile(manifest);
  }

  /**
   * Refreshes NLP entities on the remote service.
   */
  public async refreshNLPEntity(instance: IGBInstance, listName, listData): Promise<void> {
    const service = new AzureDeployerService(this);
    const res = await service.refreshEntityList(
      instance.cloudLocation,
      instance.nlpAppId,
      listName,
      instance.nlpAuthoringKey,
      listData
    );
    if (res.status !== 200) { throw res.bodyAsText; }
  }

  /**
   * Deploys a bot to the storage from a .gbot folder.
   */
  public async deployBotFromLocalPath(localPath: string, publicAddress: string): Promise<void> {
    const packageName = Path.basename(localPath);
    const instance = await this.importer.importIfNotExistsBotPackage(undefined, packageName, localPath);
    await this.deployBotFull(instance, publicAddress);
  }

  /**
   * Loads all para from tabular file Config.xlsx.
   */
  public async loadParamsFromTabular(min: GBMinInstance): Promise<any> {
    const siteId = process.env.STORAGE_SITE_ID;
    const libraryId = process.env.STORAGE_LIBRARY;

    GBLog.info(`Connecting to Config.xslx (siteId: ${siteId}, libraryId: ${libraryId})...`);

    // Connects to MSFT storage.

    const token = await min.adminService.acquireElevatedToken(min.instance.instanceId);
    const client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });

    // Retrieves all files in .bot folder.

    const botId = min.instance.botId;
    const path = `/${botId}.gbai/${botId}.gbot`;
    let url = `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/root:${path}:/children`;

    GBLog.info(`Loading .gbot from Excel: ${url}`);
    const res = await client
      .api(url)
      .get();

    // Finds Config.xlsx.

    const document = res.value.filter(m => {
      return m.name === 'Config.xlsx';
    });
    if (document === undefined || document.length === 0) {
      GBLog.info(`Config.xlsx not found on .bot folder, check the package.`);

      return null;
    }

    // Reads all rows in Config.xlsx that contains a pair of name/value
    // and fills an object that is returned to be saved in params instance field.

    const results = await client
      .api(
        `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/items/${document[0].id}/workbook/worksheets('General')/range(address='A7:B100')`
      )
      .get();
    let index = 0, obj = {};
    for (; index < results.text.length; index++) {
      if (results.text[index][0] === '') {
        return obj;
      }
      obj[results.text[index][0]] = results.text[index][1];
    }

    return obj;
  }

  /**
   * Loads all para from tabular file Config.xlsx.
   */
  public async downloadFolder(min: GBMinInstance, localPath: string, remotePath: string,
    baseUrl: string = null, client = null): Promise<any> {


    GBLog.info(`downloadFolder: localPath=${localPath}, remotePath=${remotePath}, baseUrl=${baseUrl}`);

    if (!baseUrl) {
      [baseUrl, client] = await GBDeployer.internalGetDriveClient(min);

      remotePath = remotePath.replace(/\\/gi, '/');
      const parts = remotePath.split('/');

      // Creates each subfolder.

      let pathBase = localPath;
      if (!Fs.existsSync(pathBase)) {
        Fs.mkdirSync(pathBase);
      }

      await CollectionUtil.asyncForEach(parts, async item => {
        pathBase = Path.join(pathBase, item);
        if (!Fs.existsSync(pathBase)) {
          Fs.mkdirSync(pathBase);
        }
      });

      // Retrieves all files in remote folder.

      const botId = min.instance.botId;
      const path = urlJoin(`/${botId}.gbai`, remotePath);
      let url = `${baseUrl}/drive/root:${path}:/children`;

      GBLog.info(`Download URL: ${url}`);

      const res = await client
        .api(url)
        .get();
      const documents = res.value;
      if (documents === undefined || documents.length === 0) {
        GBLog.info(`${remotePath} is an empty folder.`);
        return null;
      }

      // Download files or navigate to directory to recurse.

      await CollectionUtil.asyncForEach(documents, async item => {

        const itemPath = Path.join(localPath, remotePath, item.name);

        if (item.folder) {
          if (!Fs.existsSync(itemPath)) {
            Fs.mkdirSync(itemPath);
          }
          const nextFolder = urlJoin(remotePath, item.name);
          await this.downloadFolder(min, localPath, nextFolder);
        } else {
          let download = true;

          if (Fs.existsSync(itemPath)) {
            const dt = Fs.statSync(itemPath);
            if (new Date(dt.mtime) > new Date(item.lastModifiedDateTime)) {
              download = false;
            }
          }

          if (download) {
            GBLog.info(`Downloading ${itemPath}...`);
            const url = item['@microsoft.graph.downloadUrl'];

            const response = await request({ uri: url, encoding: null });
            Fs.writeFileSync(itemPath, response, { encoding: null });
            Fs.utimesSync(itemPath,
              new Date(), new Date(item.lastModifiedDateTime));
          }
          else {
            GBLog.info(`Local is up to date: ${itemPath}...`);
          }
        }
      });
    }
  }
  /**
   * UndDeploys a bot to the storage.
   */
  public async undeployBot(botId: string, packageName: string): Promise<void> {

    // Deletes Bot registration on cloud.

    const service = new AzureDeployerService(this);
    const group = GBConfigService.get('CLOUD_GROUP');
    if (await service.botExists(botId)) {
      await service.deleteBot(botId, group);
    }

    // Unbinds resources and listeners.

    GBServer.globals.minService.unmountBot(botId);

    // Removes the bot from the storage.

    await this.core.deleteInstance(botId);
  }

  /**
   * Deploys a new package to the database storage (just a group).
   */
  public async deployPackageToStorage(instanceId: number, packageName: string): Promise<GuaribasPackage> {
    return await GuaribasPackage.create(<GuaribasPackage>{
      packageName: packageName,
      instanceId: instanceId
    });
  }

  /**
   * Deploys a folder into the bot storage.
   */
  public async deployPackage(min: GBMinInstance, localPath: string) {

    const packageType = Path.extname(localPath);
    let handled = false;
    let pck = null;

    // Asks for each .gbapp if it will handle the package publishing.

    const _this = this;
    await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
      try {

        // If it will be handled, create a temporary service layer to be
        // called by .gbapp and manage the associated package row.

        if (
          (pck = await e.onExchangeData(min, 'handlePackage', {
            name: localPath,
            createPackage: async packageName => {
              return await _this.deployPackageToStorage(min.instance.instanceId, packageName);
            },
            updatePackage: async (p: GuaribasPackage) => {
              p.save();
            },
            existsPackage: async (packageName: string) => {
              return await _this.getStoragePackageByName(min.instance.instanceId, packageName);
            }
          }))
        ) {
          handled = true;
        }
      } catch (error) {
        GBLog.error(error);
      }
    });

    if (handled) {
      return pck;
    }

    // Deploy platform packages here accordingly to their extension.

    switch (packageType) {
      case '.gbot':

        // Extracts configuration information from .gbot files.

        if (process.env.ENABLE_PARAMS_ONLINE === 'false') {
          if (Fs.existsSync(localPath)) {
            GBLog.info(`Loading .gbot from ${localPath}.`);
            await this.deployBotFromLocalPath(localPath, GBServer.globals.publicAddress);
          }
        } else {
          min.instance.params = await this.loadParamsFromTabular(min);
        }

        // Updates instance object.

        await this.core.saveInstance(min.instance);

        break;

      case '.gbkb':

        // Deploys .gbkb into the storage.

        const service = new KBService(this.core.sequelize);
        await service.deployKb(this.core, this, localPath, min);
        break;

      case '.gbdialog':

        // Compiles files from .gbdialog into work folder and deploys
        // it to the VM.

        const vm = new GBVMService();
        await vm.loadDialogPackage(localPath, min, this.core, this);
        GBLog.info(`Dialogs (.gbdialog) for ${min.botId} loaded.`);
        break;

      case '.gbtheme':

        // Updates server listeners to serve theme files in .gbtheme.

        const packageName = Path.basename(localPath);
        GBServer.globals.server.use(`/themes/${packageName}`, express.static(localPath));
        GBLog.info(`Theme (.gbtheme) assets accessible at: /themes/${packageName}.`);

        break;

      case '.gbapp':

        // Dynamically compiles and loads .gbapp packages (Node.js packages).

        await this.callGBAppCompiler(localPath, this.core);
        break;

      case '.gblib':

        // Dynamically compiles and loads .gblib packages (Node.js packages).

        await this.callGBAppCompiler(localPath, this.core);
        break;

      default:

        const err = GBError.create(`Unhandled package type: ${packageType}.`);
        Promise.reject(err);
        break;
    }
  }

  /**
   * Removes the package from the storage and local work folders.
   */
  public async undeployPackageFromLocalPath(instance: IGBInstance, localPath: string) {

    // Gets information about the package.

    const packageType = Path.extname(localPath);
    const packageName = Path.basename(localPath);
    const p = await this.getStoragePackageByName(instance.instanceId, packageName);

    // Removes objects from storage, cloud resources and local files if any.

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
        break;

      case '.gbdialog':
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

  /**
   * Performs automation of the Indexer (Azure Search) and rebuild
   * its index based on .gbkb structure.
   */
  public async rebuildIndex(instance: IGBInstance, searchSchema: any) {

    // Prepares search.

    const search = new AzureSearch(
      instance.searchKey,
      instance.searchHost,
      instance.searchIndex,
      instance.searchIndexer
    );
    const connectionString = GBDeployer.getConnectionStringFromInstance(instance);
    const dsName = 'gb';

    // Removes any previous index.

    try {
      await search.deleteDataSource(dsName);
    } catch (err) {

      // If it is a 404 there is nothing to delete as it is the first creation.

      if (err.code !== 404) {

        throw err;
      }
    }

    // Removes the index.

    try {
      await search.deleteIndex();
    } catch (err) {

      // If it is a 404 there is nothing to delete as it is the first creation.

      if (err.code !== 404 && err.code !== "OperationNotAllowed") {
        throw err;
      }
    }

    // Creates the data source and index on the cloud.

    try {
      await search.createDataSource(dsName, dsName, 'GuaribasQuestion', 'azuresql', connectionString);
    } catch (err) {
      GBLog.error(err);
      throw err;
    }
    await search.createIndex(searchSchema, dsName);
  }

  /**
   * Finds a storage package by using package name.
   */
  public async getStoragePackageByName(instanceId: number, packageName: string): Promise<GuaribasPackage> {
    const where = { packageName: packageName, instanceId: instanceId };

    return await GuaribasPackage.findOne({
      where: where
    });
  }

  /**
   * Prepares the React application inside default.gbui folder and
   * makes this web application available as default web front-end.
   */
  public setupDefaultGBUI() {

    // Setups paths.

    const root = 'packages/default.gbui';
    const npm = urlJoin(process.env.PWD, 'node_modules', '.bin', 'npm');

    // Checks if .gbapp compiliation is enabled.

    if (!Fs.existsSync(`${root}/build`) && process.env.DISABLE_WEB !== 'true') {

      // Write a .env required to fix some bungs in create-react-app tool.

      Fs.writeFileSync(`${root}/.env`, 'SKIP_PREFLIGHT_CHECK=true');

      // Install modules and compiles the web app.

      GBLog.info(`Installing modules default.gbui (It may take a few minutes)...`);

      child_process.execSync(`${npm} install`, { cwd: root });

      GBLog.info(`Transpiling default.gbui...`);
      child_process.execSync(`${npm} run build`, { cwd: root });
    }
  }

  /**
   * Servers bot storage assets to be used by web, WhatsApp and other channels.
   */
  public static mountGBKBAssets(packageName: any, botId: string, filename: string) {

    // Servers menu assets.

    GBServer.globals.server.use(
      `/kb/${botId}.gbai/${packageName}/subjects`,
      express.static(urlJoin(filename, 'subjects'))
    );

    // Servers all other assets in .gbkb folders.

    const gbaiName = `${botId}.gbai`;
    GBServer.globals.server.use(`/kb/${gbaiName}/${packageName}/assets`,
      express.static(urlJoin('work', gbaiName, filename, 'assets')));
    GBServer.globals.server.use(`/kb/${gbaiName}/${packageName}/images`,
      express.static(urlJoin('work', gbaiName, filename, 'images')));
    GBServer.globals.server.use(`/kb/${gbaiName}/${packageName}/audios`,
      express.static(urlJoin('work', gbaiName, filename, 'audios')));
    GBServer.globals.server.use(`/kb/${gbaiName}/${packageName}/videos`,
      express.static(urlJoin('work', gbaiName, filename, 'videos')));
    GBServer.globals.server.use(`/${botId}/cache`,
      express.static(urlJoin('work', gbaiName, 'cache')));
    GBServer.globals.server.use(`/${gbaiName}/${botId}.gbdata/public`,
      express.static(urlJoin('work', gbaiName, `${botId}.gbdata`, 'public')));



    GBLog.info(`KB (.gbkb) assets accessible at: /kb/${botId}.gbai/${packageName}.`);
  }

  /**
   * Invokes Type Script compiler for a given .gbapp package (Node.js based).
   */
  public async callGBAppCompiler(
    gbappPath: string,
    core: IGBCoreService,
    appPackages: any[] = undefined,
    appPackagesProcessed: number = 0
  ) {

    // Runs `npm install` for the package.

    GBLog.info(`Deploying General Bots Application (.gbapp) or Library (.gblib): ${Path.basename(gbappPath)}...`);
    let folder = Path.join(gbappPath, 'node_modules');
    if (process.env.GBAPP_DISABLE_COMPILE !== 'true') {
      if (!Fs.existsSync(folder)) {
        GBLog.info(`Installing modules for ${gbappPath}...`);
        child_process.execSync('npm install', { cwd: gbappPath });
      }
    }

    folder = Path.join(gbappPath, 'dist');
    try {

      // Runs TSC in .gbapp folder.

      if (process.env.GBAPP_DISABLE_COMPILE !== 'true') {
        GBLog.info(`Compiling: ${gbappPath}.`);
        child_process.execSync(Path.join(process.env.PWD, 'node_modules/.bin/tsc'), { cwd: gbappPath });
      }

      // After compiled, adds the .gbapp to the current server VM context.

      if (gbappPath.endsWith('.gbapp') || gbappPath.endsWith('.gblib')) {
        const m = await import(gbappPath);
        if (m.Package) {
          const p = new m.Package();
          await p.loadPackage(core, core.sequelize);
          if (appPackages !== undefined) {
            appPackages.push(p);
          }
        }
      }
      GBLog.info(`.gbapp or .gblib deployed: ${gbappPath}.`);
      appPackagesProcessed++;

    } catch (error) {
      GBLog.error(`Error compiling package, message:  ${error.message}\n${error.stack}`);
      if (error.stdout) {
        GBLog.error(`.gbapp stdout: ${gbappPath}:\n${error.stdout.toString()}`);
      }
      appPackagesProcessed++;
    }

    return appPackagesProcessed;
  }

  /**
   * Determines if a given package is of system kind.
   */
  private isSystemPackage(name: string): Boolean {
    const names = [
      'analytics.gblib',
      'console.gblib',
      'security.gbapp',
      'whatsapp.gblib',
      'sharepoint.gblib',
      'core.gbapp',
      'basic.gblib',
      'admin.gbapp',
      'azuredeployer.gbapp',
      'customer-satisfaction.gbapp',
      'kb.gbapp',
      'google-chat.gblib',
      'teams.gblib',
      'hubspot.gblib'
    ];

    return names.indexOf(name) > -1;
  }

  /**
   * Performs the process of compiling all .gbapp folders.
   */
  private async deployAppPackages(gbappPackages: string[], core: any, appPackages: any[]) {

    // Loops through all ready to load .gbapp packages.

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
}
