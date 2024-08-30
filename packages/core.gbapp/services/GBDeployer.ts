/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import Path from 'path';
import express from 'express';
import child_process from 'child_process';
import { rimraf } from 'rimraf';
import urlJoin from 'url-join';
import Fs from 'fs';
import { GBError, GBLog, GBMinInstance, IGBCoreService, IGBDeployer, IGBInstance, IGBPackage } from 'botlib';
import { AzureSearch } from 'pragmatismo-io-framework';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBServer } from '../../../src/app.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import Excel from 'exceljs';
import asyncPromise from 'async-promises';
import { GuaribasPackage } from '../models/GBModel.js';
import { GBAdminService } from './../../admin.gbapp/services/GBAdminService.js';
import { AzureDeployerService } from './../../azuredeployer.gbapp/services/AzureDeployerService.js';
import { KBService } from './../../kb.gbapp/services/KBService.js';
import { GBConfigService } from './GBConfigService.js';
import { GBImporter } from './GBImporterService.js';
import { TeamsService } from '../../teams.gblib/services/TeamsService.js';
import MicrosoftGraph from '@microsoft/microsoft-graph-client';
import { GBLogEx } from './GBLogEx.js';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { GBUtil } from '../../../src/util.js';
import { HNSWLib } from '@langchain/community/vectorstores/hnswlib';
import { OpenAIEmbeddings } from '@langchain/openai';

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
    return `Server=tcp:${GBConfigService.get('STORAGE_SERVER')},1433;Database=${GBConfigService.get(
      'STORAGE_NAME'
    )};User ID=${GBConfigService.get('STORAGE_USERNAME')};Password=${GBConfigService.get(
      'STORAGE_PASSWORD'
    )};Trusted_Connection=False;Encrypt=True;Connection Timeout=30;`;
  }

  /**
   * Retrives token and initialize drive client API.
   */
  public static async internalGetDriveClient(min: GBMinInstance) {
    let token;

    // Get token as root only if the bot does not have
    // an custom tenant for retrieving packages.

    token = await (min.adminService as any)['acquireElevatedToken'](
      min.instance.instanceId,
      min.instance.authenticatorTenant ? false : true
    );

    const siteId = process.env.STORAGE_SITE_ID;
    const libraryId = GBConfigService.get('STORAGE_LIBRARY');

    const client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });
    const baseUrl = `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}`;
    min['cacheToken'] = { baseUrl, client };

    return { baseUrl, client };
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
          GBLogEx.info(0, `Ignoring ${element}...`);
        } else {
          const name = Path.basename(element);

          // Skips what does not need to be loaded.

          if (
            process.env.GBAPP_SKIP &&
            (process.env.GBAPP_SKIP.toLowerCase().indexOf(name) !== -1 || process.env.GBAPP_SKIP === 'true')
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

    GBLogEx.info(0, `Deploying Application packages...`);
    await CollectionUtil.asyncForEach(paths, async e => {
      GBLogEx.info(0, `Looking in: ${e}...`);
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

    GBLogEx.info(0, `App Package deployment done.`);
  }

  /**
   * Deploys a new blank bot to the database, cognitive services and other services.
   */
  public async deployBlankBot(botId: string, mobile: string, email: string) {
    // Creates a new row on the GuaribasInstance table.
    const instance = await this.importer.createBotInstance(botId);
    const bootInstance = GBServer.globals.bootInstance;

    if (GBConfigService.get('STORAGE_NAME')) {
      // Gets the access token to perform service operations.

      const accessToken = await (GBServer.globals.minBoot.adminService as any)['acquireElevatedToken'](
        bootInstance.instanceId,
        true
      );

      // Creates the MSFT application that will be associated to the bot.

      const service = await AzureDeployerService.createInstance(this);
      const application = await service.createApplication(accessToken, botId);
      // Fills new instance base information and get App secret.

      instance.marketplaceId = (application as any).appId;
      instance.marketplacePassword = await service.createApplicationSecret(accessToken, (application as any).id);
    }

    instance.adminPass = GBAdminService.getRndPassword();
    instance.title = botId;
    instance.activationCode = instance.botId.substring(0, 15);
    instance.state = 'active';
    instance.nlpScore = 0.8;
    instance.searchScore = 0.25;
    instance.params = JSON.stringify({ 'Can Publish': mobile, 'Admin Notify E-mail': email });

    // Saves bot information to the store.

    await this.core.saveInstance(instance);
    if (GBConfigService.get('STORAGE_NAME')) {
      await this.deployBotOnAzure(instance, GBServer.globals.publicAddress);
    }

    // Makes available bot to the channels and .gbui interfaces.

    await GBServer.globals.minService.mountBot(instance);

    // Creates remaining objects on the cloud and updates instance information.

    return instance;
  }

  /**
   * Verifies if bot exists on bot catalog.
   */
  public async botExists(botId: string): Promise<boolean> {
    const service = await AzureDeployerService.createInstance(this);

    return await service.botExists(botId);
  }

  /**
   * Performs all tasks of deploying a new bot on the cloud.
   */
  public async deployBotOnAzure(instance: IGBInstance, publicAddress: string): Promise<IGBInstance> {
    // Reads base configuration from environent file.

    const service = await AzureDeployerService.createInstance(this);
    const username = GBConfigService.get('CLOUD_USERNAME');
    const password = GBConfigService.get('CLOUD_PASSWORD');
    const accessToken = await GBAdminService.getADALTokenFromUsername(username, password);
    const group = GBConfigService.get('CLOUD_GROUP') ?? GBConfigService.get('BOT_ID');
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
    }

    // Saves final instance object and returns it.

    return await this.core.saveInstance(instance);
  }

  public async loadOrCreateEmptyVectorStore(min: GBMinInstance): Promise<HNSWLib> {
    let vectorStore: HNSWLib;

    const azureOpenAIKey = await (min.core as any)['getParam'](min.instance, 'Azure Open AI Key', null, true);
    const azureOpenAIVersion = await (min.core as any)['getParam'](min.instance, 'Azure Open AI Version', null, true);
    const azureOpenAIApiInstanceName = await (min.core as any)['getParam'](
      min.instance,
      'Azure Open AI Instance',
      null,
      true
    );
    const azureOpenAIEmbeddingModel = await (min.core as any)['getParam'](
      min.instance,
      'Azure Open AI Embedding Model',
      null,
      true
    );

    let embedding;
    if (!azureOpenAIEmbeddingModel) {
      return;
    }

    embedding = new OpenAIEmbeddings({
      maxConcurrency: 5,
      azureOpenAIApiKey: azureOpenAIKey,
      azureOpenAIApiDeploymentName: azureOpenAIEmbeddingModel,
      azureOpenAIApiVersion: azureOpenAIVersion,
      azureOpenAIApiInstanceName: azureOpenAIApiInstanceName
    });

    try {
      vectorStore = await HNSWLib.load(min['vectorStorePath'], embedding);
    } catch {
      vectorStore = new HNSWLib(embedding, {
        space: 'cosine'
      });
    }
    return vectorStore;
  }

  /**
   * Performs the NLP publishing process on remote service.
   */
  public async publishNLP(instance: IGBInstance): Promise<void> {
    const service = await AzureDeployerService.createInstance(this);
    const res = await service.publishNLP(instance.cloudLocation, instance.nlpAppId, instance.nlpAuthoringKey);
    if (res.status !== 200 && res.status !== 201) {
      throw res.bodyAsText;
    }
  }

  /**
   * Trains NLP on the remote service.
   */
  public async trainNLP(instance: IGBInstance): Promise<void> {
    const service = await AzureDeployerService.createInstance(this);
    const res = await service.trainNLP(instance.cloudLocation, instance.nlpAppId, instance.nlpAuthoringKey);
    if (res.status !== 200 && res.status !== 202) {
      throw res.bodyAsText;
    }
    GBUtil.sleep(5000);
  }

  /**
   * Return a zip file for importing bot in apps, currently MS Teams.
   */
  public async getBotManifest(instance: IGBInstance): Promise<Buffer> {
    const s = new TeamsService();
    const manifest = await s.getManifest(
      instance.marketplaceId,
      instance.botId,
      instance.description,
      GBAdminService.generateUuid(),
      instance.botId,
      'General Bots'
    );

    return await s.getAppFile(manifest);
  }

  /**
   * Refreshes NLP entities on the remote service.
   */
  public async refreshNLPEntity(instance: IGBInstance, listName, listData): Promise<void> {
    const service = await AzureDeployerService.createInstance(this);
    const res = await service.refreshEntityList(
      instance.cloudLocation,
      instance.nlpAppId,
      listName,
      instance.nlpAuthoringKey,
      listData
    );
    if (res.status !== 200) {
      throw res.bodyAsText;
    }
  }

  /**
   * Deploys a bot to the storage from a .gbot folder.
   */
  public async deployBotFromLocalPath(localPath: string, publicAddress: string): Promise<void> {
    const packageName = Path.basename(localPath);
    const instance = await this.importer.importIfNotExistsBotPackage(undefined, packageName, localPath);
    await this.deployBotOnAzure(instance, publicAddress);
  }

  /**
   * Loads all para from tabular file Config.xlsx.
   */

  public async loadParamsFromTabular(min: GBMinInstance, filePath: string): Promise<any> {
    const xls = Path.join(filePath, 'Config.xlsx');
    const csv = Path.join(filePath, 'config.csv');

    let rows: any[] = [];
    let obj: any = {};

    const workbook = new Excel.Workbook();

    if (Fs.existsSync(xls)) {
      await workbook.xlsx.readFile(xls);
      let worksheet: any;
      for (let t = 0; t < workbook.worksheets.length; t++) {
        worksheet = workbook.worksheets[t];
        if (worksheet) {
          break;
        }
      }
      rows = worksheet.getSheetValues();

      // Skips the header lines.
      for (let index = 0; index < 6; index++) {
        rows.shift();
      }
    } else if (Fs.existsSync(csv)) {
      await workbook.csv.readFile(csv);
      let worksheet = workbook.worksheets[0]; // Assuming the CSV file has only one sheet
      rows = worksheet.getSheetValues();

      // Skips the header lines.

      rows.shift();
    } else {
      return [];
    }
    await asyncPromise.eachSeries(rows, async (line: any) => {
      if (line && line.length > 0) {
        obj[line[1]] = line[2];
      }
    });

    GBLogEx.info(min, `Processing ${rows.length} rows from ${filePath}...`);
    rows = null;
    return obj;
  }

  /**
   */
  public async downloadFolder(
    min: GBMinInstance,
    localPath: string,
    remotePath: string,
    baseUrl: string = null,
    client = null
  ): Promise<any> {
    GBLogEx.info(min, `downloadFolder: localPath=${localPath}, remotePath=${remotePath}, baseUrl=${baseUrl}`);

    if (!baseUrl) {
      let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

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

      let path = DialogKeywords.getGBAIPath(min.botId);
      path = urlJoin(path, remotePath);
      let url = `${baseUrl}/drive/root:/${path}:/children`;

      GBLogEx.info(min, `Download URL: ${url}`);

      const res = await client.api(url).get();
      const documents = res.value;
      if (documents === undefined || documents.length === 0) {
        GBLogEx.info(min, `${remotePath} is an empty folder.`);
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
            if (new Date(dt.mtime) >= new Date(item.lastModifiedDateTime)) {
              download = false;
            }
          }

          if (download) {
            GBLogEx.verbose(min, `Downloading ${itemPath}...`);
            const url = item['@microsoft.graph.downloadUrl'];

            const response = await fetch(url);
            Fs.writeFileSync(itemPath, Buffer.from(await response.arrayBuffer()), { encoding: null });
            Fs.utimesSync(itemPath, new Date(), new Date(item.lastModifiedDateTime));
          } else {
            GBLogEx.info(min, `Local is up to date: ${itemPath}...`);
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

    const service = await AzureDeployerService.createInstance(this);
    const group = GBConfigService.get('BOT_ID');
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
  public async deployPackage(min: GBMinInstance, localPath: string) {
    // TODO:  Adjust interface mismatch.
  }
  /**
   * Deploys a folder into the bot storage.
   */
  public async deployPackage2(min: GBMinInstance, user, localPath: string) {
    const packageType = Path.extname(localPath);
    let handled = false;
    let pck = null;

    // Asks for each .gbapp if it will handle the package publishing.

    const _this = this;
    await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
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
    });

    if (handled) {
      return pck;
    }

    // Deploy platform packages here accordingly to their extension.

    switch (packageType) {
      case '.gbot':
        // Extracts configuration information from .gbot files.

        min.instance.params = await this.loadParamsFromTabular(min, localPath);
        if (min.instance.params) {
          let connections = [];

          // Find all tokens in .gbot Config.
          const strFind = ' Driver';
          const conns = await min.core['findParam'](min.instance, strFind);
          await CollectionUtil.asyncForEach(conns, async t => {
            const connectionName = t.replace(strFind, '');
            let con = {};
            con['name'] = connectionName;
            con['storageDriver'] = min.core.getParam<string>(min.instance, `${connectionName} Driver`, null);
            const storageName = min.core.getParam<string>(min.instance, `${connectionName} Name`, null);

            let file = min.core.getParam<string>(min.instance, `${connectionName} File`, null);

            if (storageName) {
              con['storageName'] = storageName;
              con['storageServer'] = min.core.getParam<string>(min.instance, `${connectionName} Server`, null);
              con['storageUsername'] = min.core.getParam<string>(min.instance, `${connectionName} Username`, null);
              con['storagePort'] = min.core.getParam<string>(min.instance, `${connectionName} Port`, null);
              con['storagePassword'] = min.core.getParam<string>(min.instance, `${connectionName} Password`, null);
            } else if (file) {
              const path = DialogKeywords.getGBAIPath(min.botId, 'gbdata');
              con['storageFile'] = Path.join(GBConfigService.get('STORAGE_LIBRARY'), path, file);
            } else {
              GBLogEx.debug(min, `No storage information found for ${connectionName}, missing storage name or file.`);
            }
            connections.push(con);
          });

          const path = DialogKeywords.getGBAIPath(min.botId, null);
          const localFolder = Path.join('work', path, 'connections.json');
          Fs.writeFileSync(localFolder, JSON.stringify(connections), { encoding: null });

          // Updates instance object.

          await this.core.saveInstance(min.instance);
          GBLogEx.info(min, `Reloading bot ${min.botId}...`);
          GBServer.globals.minService.unmountBot(min.botId);
          GBServer.globals.minService.mountBot(min.instance);
          GBLogEx.info(min, `Bot ${min.botId} reloaded.`);
        }
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
        GBLogEx.verbose(min, `Dialogs (.gbdialog) for ${min.botId} loaded.`);
        break;

      case '.gbtheme':
        // Updates server listeners to serve theme files in .gbtheme.

        const packageName = Path.basename(localPath);
        GBServer.globals.server.use(`/themes/${packageName}`, express.static(localPath));
        GBLogEx.verbose(min, `Theme (.gbtheme) assets accessible at: /themes/${packageName}.`);

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
        throw GBError.create(`Unhandled package type: ${packageType}.`);
    }
  }

  /**
   * Removes the package local files from cache.
   */
  public async cleanupPackage(instance: IGBInstance, packageName: string) {
    const path = DialogKeywords.getGBAIPath(instance.botId, null, packageName);
    const localFolder = Path.join('work', path);
    rimraf.sync(localFolder);
  }

  /**
   * Removes the package from the storage and local work folders.
   */
  public async undeployPackageFromPackageName(instance: IGBInstance, packageName: string) {
    // Gets information about the package.

    const packageType = Path.extname(packageName);
    const p = await this.getStoragePackageByName(instance.instanceId, packageName);

    const path = DialogKeywords.getGBAIPath(instance.botId, null, packageName);
    const localFolder = Path.join('work', path);

    return await this.undeployPackageFromLocalPath(instance, localFolder);
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

        if (p) {
          await service.undeployKbFromStorage(instance, this, p.packageId);
        }

        return;
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
    const key = instance.searchKey ? instance.searchKey : GBServer.globals.minBoot.instance.searchKey;
    GBLogEx.info(instance.instanceId, `rebuildIndex running...`);

    if (!key) {
      return;
    }
    const searchIndex = instance.searchIndex ? instance.searchIndex : GBServer.globals.minBoot.instance.searchIndex;
    const searchIndexer = instance.searchIndexer
      ? instance.searchIndexer
      : GBServer.globals.minBoot.instance.searchIndexer;
    const host = instance.searchHost ? instance.searchHost : GBServer.globals.minBoot.instance.searchHost;

    // Prepares search.

    const search = new AzureSearch(key, host, searchIndex, searchIndexer);
    const connectionString = GBDeployer.getConnectionStringFromInstance(GBServer.globals.minBoot.instance);
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

      if (err.code !== 404 && err.code !== 'OperationNotAllowed') {
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

    GBLogEx.info(instance.instanceId, `Released rebuildIndex mutex.`);
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

      GBLogEx.info(0, `Installing modules default.gbui (It may take a few minutes)...`);

      child_process.execSync(`${npm} install`, { cwd: root });

      GBLogEx.info(0, `Transpiling default.gbui...`);
      child_process.execSync(`${npm} run build`, { cwd: root });
    }
  }

  /**
   * Servers bot storage assets to be used by web, WhatsApp and other channels.
   */
  public static mountGBKBAssets(packageName: any, botId: string, filename: string) {
    const gbaiName = DialogKeywords.getGBAIPath(botId);

    // Servers menu assets.

    GBServer.globals.server.use(
      `/kb/${gbaiName}/${packageName}/subjects`,
      express.static(urlJoin(filename, 'subjects'))
    );

    // Servers all other assets in .gbkb folders.

    GBServer.globals.server.use(
      `/kb/${gbaiName}/${packageName}/assets`,
      express.static(urlJoin('work', gbaiName, filename, 'assets'))
    );
    GBServer.globals.server.use(
      `/kb/${gbaiName}/${packageName}/images`,
      express.static(urlJoin('work', gbaiName, filename, 'images'))
    );
    GBServer.globals.server.use(
      `/kb/${gbaiName}/${packageName}/docs`,
      express.static(urlJoin('work', gbaiName, filename, 'docs'))
    );
    GBServer.globals.server.use(
      `/kb/${gbaiName}/${packageName}/audios`,
      express.static(urlJoin('work', gbaiName, filename, 'audios'))
    );
    GBServer.globals.server.use(
      `/kb/${gbaiName}/${packageName}/videos`,
      express.static(urlJoin('work', gbaiName, filename, 'videos'))
    );
    GBServer.globals.server.use(`/${botId}/cache`, express.static(urlJoin('work', gbaiName, 'cache')));
    GBServer.globals.server.use(
      `/${gbaiName}/${botId}.gbdrive/public`,
      express.static(urlJoin('work', gbaiName, `${botId}.gbdata`, 'public'))
    );

    GBLog.verbose(`KB (.gbkb) assets accessible at: /kb/${gbaiName}/${packageName}.`);
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

    GBLogEx.info(0, `Deploying General Bots Application (.gbapp) or Library (.gblib): ${Path.basename(gbappPath)}...`);
    let folder = Path.join(gbappPath, 'node_modules');
    if (process.env.GBAPP_DISABLE_COMPILE !== 'true') {
      if (!Fs.existsSync(folder)) {
        GBLogEx.info(0, `Installing modules for ${gbappPath}...`);
        child_process.execSync('npm install', { cwd: gbappPath });
      }
    }

    folder = Path.join(gbappPath, 'dist');
    try {
      // Runs TSC in .gbapp folder.

      if (process.env.GBAPP_DISABLE_COMPILE !== 'true') {
        GBLogEx.info(0, `Compiling: ${gbappPath}.`);
        child_process.execSync(Path.join(process.env.PWD, 'node_modules/.bin/tsc'), { cwd: gbappPath });
      }

      // After compiled, adds the .gbapp to the current server VM context.

      if (gbappPath.endsWith('.gbapp') || gbappPath.endsWith('.gblib')) {
        const m = await import(`file://${gbappPath}/dist/index.js`);
        if (m.Package) {
          const p = new m.Package();

          // Adds a name property to the list of loaded .gbapp packages.

          p['name'] = gbappPath;
          await p.loadPackage(core, core.sequelize);
          if (appPackages !== undefined) {
            appPackages.push(p);
          }
        }
      }
      GBLogEx.info(0, `.gbapp or .gblib deployed: ${gbappPath}.`);
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
      'hubspot.gblib',
      'llm.gblib'
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
