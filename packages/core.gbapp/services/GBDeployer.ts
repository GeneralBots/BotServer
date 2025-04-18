/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import path from 'path';
import express from 'express';
import child_process from 'child_process';
import { rimraf } from 'rimraf';
import urlJoin from 'url-join';
import { Client } from 'minio';

import fs from 'fs/promises';
import { GBError, GBLog, GBMinInstance, IGBCoreService, IGBDeployer, IGBInstance, IGBPackage } from 'botlib';
import { AzureSearch } from 'pragmatismo-io-framework';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBServer } from '../../../src/app.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import Excel from 'exceljs';
import asyncPromise from 'async-promises';
import { GuaribasInstance, GuaribasPackage } from '../models/GBModel.js';
import { GBAdminService } from './../../admin.gbapp/services/GBAdminService.js';
import { AzureDeployerService } from './../../azuredeployer.gbapp/services/AzureDeployerService.js';
import { KBService } from './../../kb.gbapp/services/KBService.js';
import { GBConfigService } from './GBConfigService.js';
import { GBImporter } from './GBImporterService.js';
import { TeamsService } from '../../teams.gblib/services/TeamsService.js';
import MicrosoftGraph from '@microsoft/microsoft-graph-client';
import { GBLogEx } from './GBLogEx.js';
import { GBUtil } from '../../../src/util.js';
import { HNSWLib } from '@langchain/community/vectorstores/hnswlib';
import { OpenAIEmbeddings } from '@langchain/openai';
import { GBMinService } from './GBMinService.js';

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

    async function scanPackageDirectory(directory) {
      // Gets all directories.

      const isDirectory = async source => (await fs.lstat(source)).isDirectory();
      const getDirectories = async source =>
        (await fs.readdir(source)).map(name => path.join(source, name)).filter(isDirectory);
      const dirs = await getDirectories(directory);
      await CollectionUtil.asyncForEach(dirs, async element => {
        // For each folder, checks its extensions looking for valid packages.

        if (element === '.') {
          GBLogEx.info(0, `Ignoring ${element}...`);
        } else {
          const name = path.basename(element);

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

    if (GBConfigService.get('GB_MODE') === 'legacy') {
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

    instance.adminPass =await  GBUtil.hashPassword( GBAdminService.getRndPassword());
    instance.title = botId;
    instance.activationCode = instance.botId.substring(0, 15);
    instance.state = 'active';
    instance.nlpScore = 0.8;
    instance.searchScore = 0.25;
    instance.params = JSON.stringify({ 'Can Publish': mobile, 'Admin Notify E-mail': email });

    // Saves bot information to the store.

    await this.core.saveInstance(instance);
    if (GBConfigService.get('GB_MODE') === 'legacy') {
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

    if (GBConfigService.get('GB_MODE') !== 'legacy') {
      const where = { botId: botId };

      return await GuaribasInstance.findOne({
        where: where
      }) !== null;
  
    }
    else {

      const service = await AzureDeployerService.createInstance(this);

      return await service.botExists(botId);
    
    }
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
    } catch (e) {
      GBLogEx.info(min, `Creating new store...`);
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
    const packageName = path.basename(localPath);
    const instance = await this.importer.importIfNotExistsBotPackage(undefined, packageName, localPath);
    await this.deployBotOnAzure(instance, publicAddress);
  }

  /**
   * Loads all para from tabular file Config.xlsx.
   */

  public async loadParamsFromTabular(min: GBMinInstance, filePath: string): Promise<any> {
    const xls = path.join(filePath, 'Config.xlsx');
    const csv = path.join(filePath, 'config.csv');

    let rows: any[] = [];
    let obj: any = {};

    const workbook = new Excel.Workbook();

    if (await GBUtil.exists(xls)) {
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
    } else if (await GBUtil.exists(csv)) {
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
        const key = line[1];
        let value = line[2];


        if (key && value) {
          if (value.text) { value = value.text };
          obj[key] = value;
        }
      }
    });

    GBLogEx.info(min, `Processing ${rows.length} rows from ${path.basename(filePath)}...`);
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
    const storageMode = process.env.GB_MODE;

    if (storageMode === 'gbcluster') {
      const minioClient = new Client({
        endPoint: process.env.DRIVE_SERVER || 'localhost',
        port: parseInt(process.env.DRIVE_PORT || '9000', 10),
        useSSL: process.env.DRIVE_USE_SSL === 'true',
        accessKey: process.env.DRIVE_ACCESSKEY,
        secretKey: process.env.DRIVE_SECRET,
      });

      const bucketName =  (process.env.DRIVE_ORG_PREFIX + min.botId + '.gbai').toLowerCase();

      if (!(await GBUtil.exists(localPath))) {
        await fs.mkdir(localPath, { recursive: true });
      }

      const objectsStream = minioClient.listObjects(bucketName, remotePath, true);
      for await (const obj of objectsStream) {
        const itemPath = path.join(localPath, obj.name);

        if (obj.name.endsWith('/')) {
          if (!(await GBUtil.exists(itemPath))) {
            await fs.mkdir(itemPath, { recursive: true });
          }
        } else {
          let download = true;

          if (await GBUtil.exists(itemPath)) {
            const stats = await fs.stat(itemPath);
            if (stats.mtime >= new Date(obj.lastModified)) {
              download = false;
            }
          }

          if (download) {
            await minioClient.fGetObject(bucketName, obj.name, itemPath);
            await fs.utimes(itemPath, new Date(), new Date(obj.lastModified));
          }
        }
      }
    } else {
      if (!baseUrl) {
        const { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

        remotePath = remotePath.replace(/\\/gi, '/');
        const parts = remotePath.split('/');

        let pathBase = localPath;
        if (!(await GBUtil.exists(pathBase))) {
          await fs.mkdir(pathBase, { recursive: true });
        }

        await CollectionUtil.asyncForEach(parts, async (item) => {
          pathBase = path.join(pathBase, item);
          if (!(await GBUtil.exists(pathBase))) {
            await fs.mkdir(pathBase, { recursive: true });
          }
        });

        let packagePath = GBUtil.getGBAIPath(min.botId);
        packagePath = urlJoin(packagePath, remotePath);
        let url = `${baseUrl}/drive/root:/${packagePath}:/children`;

        let documents;

        try {
          const res = await client.api(url).get();
          documents = res.value;
        } catch (error) {
          GBLogEx.info(min, `Error downloading: ${error.toString()}`);
        }

        if (documents === undefined || documents.length === 0) {
          return null;
        }

        await CollectionUtil.asyncForEach(documents, async (item) => {
          const itemPath = path.join(localPath, remotePath, item.name);

          if (item.folder) {
            if (!(await GBUtil.exists(itemPath))) {
              await fs.mkdir(itemPath, { recursive: true });
            }
            const nextFolder = urlJoin(remotePath, item.name);
            await this.downloadFolder(min, localPath, nextFolder);
          } else {
            let download = true;

            if (await GBUtil.exists(itemPath)) {
              const stats = await fs.stat(itemPath);
              if (new Date(stats.mtime) >= new Date(item.lastModifiedDateTime)) {
                download = false;
              }
            }

            if (download) {
              const url = item['@microsoft.graph.downloadUrl'];

              const response = await fetch(url);
              await fs.writeFile(itemPath, new Uint8Array(await response.arrayBuffer()), { encoding: null });
              await fs.utimes(itemPath, new Date(), new Date(item.lastModifiedDateTime));
            }
          }
        });
      }
    }

  }

  /**
   * Undeploys a bot to the storage.
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
  public async deployPackage2(min: GBMinInstance, user, packageWorkFolder: string, download = false) {
    const packageName = path.basename(packageWorkFolder);
    const packageType = path.extname(packageWorkFolder);
    let handled = false;
    let pck = null;

    const gbai = GBUtil.getGBAIPath(min.instance.botId);

    if (download) {
      if (packageType === '.gbkb' || packageType === '.gbtheme') {
        await this.cleanupPackage(min.instance, packageName);
      }

      if (GBConfigService.get('GB_MODE') === 'local') {
        const filePath = path.join(GBConfigService.get('STORAGE_LIBRARY'), gbai, packageName);
        await GBUtil.copyIfNewerRecursive(filePath, packageWorkFolder);
      } else {
        await this.downloadFolder(min, path.join('work', `${gbai}`), packageName);
      }
    }

    // Asks for each .gbapp if it will handle the package publishing.

    const _this = this;
    await CollectionUtil.asyncForEach(min.appPackages, async (e: IGBPackage) => {
      // If it will be handled, create a temporary service layer to be
      // called by .gbapp and manage the associated package row.

      if (
        (pck = await e.onExchangeData(min, 'handlePackage', {
          name: packageWorkFolder,
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

        min.instance.params = await this.loadParamsFromTabular(min, packageWorkFolder);
        if (min.instance.params) {
          let connections = [];

          // Find all tokens in .gbot Config.
          const strFind = ' Driver';
          const conns = await min.core['findParam'](min.instance, strFind);
          await CollectionUtil.asyncForEach(conns, async t => {
            const connectionName = t.replace(strFind, '').trim();
            let con = {};
            con['name'] = connectionName;
            con['storageDriver'] = min.core.getParam<string>(min.instance, `${connectionName} Driver`, null);
            con['storageTables'] = min.core.getParam<string>(min.instance, `${connectionName} Tables`, null);
            const storageName = min.core.getParam<string>(min.instance, `${connectionName} Name`, null);

            let file = min.core.getParam<string>(min.instance, `${connectionName} File`, null);

            if (storageName) {
              con['storageName'] = storageName.trim();
              con['storageServer'] = min.core.getParam<string>(min.instance, `${connectionName} Server`, null);
              con['storageUsername'] = min.core.getParam<string>(min.instance, `${connectionName} Username`, null);
              con['storagePort'] = min.core.getParam<string>(min.instance, `${connectionName} Port`, null);
              con['storagePassword'] = min.core.getParam<string>(min.instance, `${connectionName} Password`, null);
            } else if (file) {
              const packagePath = GBUtil.getGBAIPath(min.botId, 'gbdata');
              con['storageFile'] = path.join(GBConfigService.get('STORAGE_LIBRARY'), packagePath, file);
            } else {
              GBLogEx.debug(min, `No storage information found for ${connectionName}, missing storage name or file.`);
            }
            connections.push(con);
          });

          const packagePath = GBUtil.getGBAIPath(min.botId, null);
          const localFolder = path.join('work', packagePath, 'connections.json');
          await fs.writeFile(localFolder, JSON.stringify(connections), { encoding: null });

          // Updates instance object.

          await this.core.saveInstance(min.instance);
          GBServer.globals.minService.unmountBot(min.botId);
          GBServer.globals.minService.mountBot(min.instance);

          GBLogEx.info(min, `Bot ${min.botId} reloaded.`);
        }
        break;

      case '.gbkb':
        // Deploys .gbkb into the storage.

        const service = new KBService(this.core.sequelize);
        await service.deployKb(this.core, this, packageWorkFolder, min);
        break;

      case '.gbdialog':
        // Compiles files from .gbdialog into work folder and deploys
        // it to the VM.

        const vm = new GBVMService();
        await vm.loadDialogPackage(packageWorkFolder, min, this.core, this);
        GBLogEx.verbose(min, `Dialogs (.gbdialog) for ${min.botId} loaded.`);
        break;

      case '.gbtheme':
        // Updates server listeners to serve theme files in .gbtheme.
        const filePath = path.join(process.env.PWD, 'templates', 'default.gbai', 'default.gbtheme');
        GBServer.globals.server.use('/' + urlJoin('themes', packageName), express.static(filePath));
        GBLogEx.verbose(min, `Theme (.gbtheme) assets accessible at: /themes/${packageName}.`);

        break;

      case '.gbapp':
        // Dynamically compiles and loads .gbapp packages (Node.js packages).

        await this.callGBAppCompiler(packageWorkFolder, this.core);
        break;

      case '.gblib':
        // Dynamically compiles and loads .gblib packages (Node.js packages).

        await this.callGBAppCompiler(packageWorkFolder, this.core);
        break;

      default:
        throw GBError.create(`Unhandled package type: ${packageType}.`);
    }
  }

  /**
   * Removes the package local files from cache.
   */
  public async cleanupPackage(instance: IGBInstance, packageName: string) {
    const packagePath = GBUtil.getGBAIPath(instance.botId, null, packageName);
    const localFolder = path.join('work', packagePath);
    rimraf.sync(localFolder);
  }

  /**
   * Removes the package from the storage and local work folders.
   */
  public async undeployPackageFromPackageName(instance: IGBInstance, packageName: string) {
    // Gets information about the package.

    const p = await this.getStoragePackageByName(instance.instanceId, packageName);

    const packagePath = GBUtil.getGBAIPath(instance.botId, null, packageName);
    const localFolder = path.join('work', packagePath);

    return await this.undeployPackageFromLocalPath(instance, localFolder);
  }

  /**
   * Removes the package from the storage and local work folders.
   */
  public async undeployPackageFromLocalPath(instance: IGBInstance, localPath: string) {
    // Gets information about the package.

    const packageType = path.extname(localPath);
    const packageName = path.basename(localPath);
    const p = await this.getStoragePackageByName(instance.instanceId, packageName);

    // Removes objects from storage, cloud resources and local files if any.

    switch (packageType) {
      case '.gbot':
        const packageObject = JSON.parse(await fs.readFile(urlJoin(localPath, 'package.json'), 'utf8'));
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
    } catch (error) {
      // If it is a 404 there is nothing to delete as it is the first creation.

      if (error.code !== 404) {
        throw error;
      }
    }

    // Removes the index.

    try {
      await search.deleteIndex();
    } catch (error) {
      // If it is a 404 there is nothing to delete as it is the first creation.

      if (error.code !== 404 && error.code !== 'OperationNotAllowed') {
        throw error;
      }
    }

    // Creates the data source and index on the cloud.

    try {
      await search.createDataSource(dsName, dsName, 'GuaribasQuestion', 'azuresql', connectionString);
    } catch (error) {
      GBLog.error(error);
      throw error;
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
  public async setupDefaultGBUI() {
    // Setups paths.

    const root = 'packages/default.gbui';
    const npm = urlJoin(process.env.PWD, 'node_modules', '.bin', 'npm');

    // Checks if .gbapp compiliation is enabled.

    if (!(await GBUtil.exists(`${root}/build`)) && process.env.DISABLE_WEB !== 'true') {
      // Write a .env required to fix some bungs in create-react-app tool.

      await fs.writeFile(`${root}/.env`, 'SKIP_PREFLIGHT_CHECK=true');

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
    const gbaiName = GBUtil.getGBAIPath(botId);

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

    // FEAT-A7B1F6

    GBServer.globals.server.use(
      `/${gbaiName}/${botId}.gbdrive/public`,
      express.static(urlJoin('work', gbaiName, `${botId}.gbdrive`, 'public'))
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

    GBLogEx.info(0, `Deploying General Bots Application (.gbapp) or Library (.gblib): ${path.basename(gbappPath)}...`);
    let folder = path.join(gbappPath, 'node_modules');
    if (process.env.GBAPP_DISABLE_COMPILE !== 'true') {
      if (!(await GBUtil.exists(folder))) {
        GBLogEx.info(0, `Installing modules for ${path.basename(gbappPath)}...`);
        child_process.execSync('npm install', { cwd: gbappPath });
      }
    }

    folder = path.join(gbappPath, 'dist');
    try {
      // Runs TSC in .gbapp folder.

      if (process.env.GBAPP_DISABLE_COMPILE !== 'true') {
        GBLogEx.info(0, `Compiling: ${path.basename(gbappPath)}.`);
        child_process.execSync(path.join(process.env.PWD, 'node_modules/.bin/tsc'), { cwd: gbappPath });
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
      'llm.gblib',
      'saas.gbapp'
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
      const filenameOnly = path.basename(e);

      // Skips .gbapp inside deploy folder.

      if (this.isSystemPackage(filenameOnly) === false) {
        appPackagesProcessed = await this.callGBAppCompiler(e, core, appPackages, appPackagesProcessed);
      }
    });

    return appPackagesProcessed;
  }
}
