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

import { GBLog, GBMinInstance, IGBCoreService, IGBInstallationDeployer, IGBInstance, IGBPackage } from 'botlib';
import fs from 'fs/promises'; 
import { Sequelize, SequelizeOptions } from 'sequelize-typescript';
import { Op, Dialect } from 'sequelize';
import { GBServer } from '../../../src/app.js';
import { GBAdminPackage } from '../../admin.gbapp/index.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { GBAnalyticsPackage } from '../../analytics.gblib/index.js';
import { StartDialog } from '../../azuredeployer.gbapp/dialogs/StartDialog.js';
import { GBCorePackage } from '../../core.gbapp/index.js';
import { GBCustomerSatisfactionPackage } from '../../customer-satisfaction.gbapp/index.js';
import { GBKBPackage } from '../../kb.gbapp/index.js';
import { GBSecurityPackage } from '../../security.gbapp/index.js';
import { v2 as webdav } from 'webdav-server';
import { GBWhatsappPackage } from '../../whatsapp.gblib/index.js';
import { GuaribasApplications, GuaribasInstance, GuaribasLog } from '../models/GBModel.js';
import { GBConfigService } from './GBConfigService.js';
import mkdirp from 'mkdirp';
import { GBAzureDeployerPackage } from '../../azuredeployer.gbapp/index.js';
import { GBSharePointPackage } from '../../sharepoint.gblib/index.js';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBBasicPackage } from '../../basic.gblib/index.js';
import { GBGoogleChatPackage } from '../../google-chat.gblib/index.js';
import { GBHubSpotPackage } from '../../hubspot.gblib/index.js';
import open from 'open';
import ngrok from 'ngrok';
import path from 'path';
import { GBUtil } from '../../../src/util.js';
import { GBLogEx } from './GBLogEx.js';
import { GBDeployer } from './GBDeployer.js';
import { SystemKeywords } from '../../basic.gblib/services/SystemKeywords.js';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import csvdb from 'csv-database';

/**
 * GBCoreService contains main logic for handling storage services related
 * to instance handling. When the server starts a instance is needed and
 * if no instance is found a boot instance is created. After that high-level
 * instance management methods can be created.
 * Core scheduling, base network services are also handled in this service.
 */
export class GBCoreService implements IGBCoreService {
  /**
   * Data access layer instance.
   */
  public sequelize: Sequelize;

  /**
   * Administrative services.
   */
  public adminService: GBAdminService;

  /**
   * Allows filtering on SQL generated before send to the database.
   */
  private queryGenerator: any;

  /**
   * Custom create table query.
   */
  private createTableQuery: (tableName: string, attributes: any, options: any) => string;

  /**
   * Custom change column query.
   */
  private changeColumnQuery: (tableName: string, attributes: any) => string;

  /**
   * Dialect used. Tested: mssql and sqlite.
   */
  private dialect: string;

  /**
   *
   */
  constructor() {
    this.adminService = new GBAdminService(this);
  }
  public async ensureInstances(instances: IGBInstance[], bootInstance: any, core: IGBCoreService) {}

  /**
   * Gets database config and connect to storage. Currently two databases
   * are available: SQL Server and SQLite.
   */
  public async initStorage(): Promise<any> {
    this.dialect = GBConfigService.get('STORAGE_DIALECT');

    let host: string | undefined;
    let database: string | undefined;
    let username: string | undefined;
    let password: string | undefined;
    let storage: string | undefined;

    if (this.dialect === 'mssql') {
      host = GBConfigService.get('STORAGE_SERVER');
      database = GBConfigService.get('STORAGE_NAME');
      username = GBConfigService.get('STORAGE_USERNAME');
      password = GBConfigService.get('STORAGE_PASSWORD');
    } else if (this.dialect === 'sqlite') {
      storage = GBConfigService.get('STORAGE_FILE');

      if (!await GBUtil.exists(storage)) {
        process.env.STORAGE_SYNC = 'true';
      }
    } else {
      throw new Error(`Unknown dialect: ${this.dialect}.`);
    }

    const logging: boolean | Function =
      GBConfigService.get('STORAGE_LOGGING') === 'true'
        ? (str: string): void => {
            GBLogEx.info(0, str);
          }
        : false;

    const encrypt: boolean = GBConfigService.get('STORAGE_ENCRYPT') === 'true';

    const acquire = parseInt(GBConfigService.get('STORAGE_ACQUIRE_TIMEOUT'));
    const sequelizeOptions: SequelizeOptions = {
      define: {
        freezeTableName: true,
        timestamps: false
      },
      host: host,
      logging: logging as boolean,
      dialect: this.dialect as Dialect,
      storage: storage,
      quoteIdentifiers: false, // set case-insensitive
      dialectOptions: {
        options: {
          trustServerCertificate: true,
          encrypt: encrypt
        }
      },
      pool: {
        max: 5,
        min: 0,
        idle: 10000,
        evict: 10000,
        acquire: acquire
      }
    };

    this.sequelize = new Sequelize(database, username, password, sequelizeOptions);
  }

  /**
   * Checks wheather storage is acessible or not and opens firewall
   * in case of any connection block.
   */
  public async checkStorage(installationDeployer: IGBInstallationDeployer) {
    try {
      await this.sequelize.authenticate();
    } catch (error) {
      GBLogEx.info(0, 'Opening storage firewall on infrastructure...');
      // tslint:disable:no-unsafe-any
      if (error.parent.code === 'ELOGIN') {
        await this.openStorageFrontier(installationDeployer);
      } else {
        throw error;
      }
      // tslint:ensable:no-unsafe-any
    }
  }

  /**
   * Syncronizes structure between model and tables in storage.
   */
  public async syncDatabaseStructure() {
    if (GBConfigService.get('STORAGE_SYNC') === 'true') {
      const alter = GBConfigService.get('STORAGE_SYNC_ALTER') === 'true';
      GBLogEx.info(0, 'Syncing database...');

      return await this.sequelize.sync({
        alter: alter,
        force: false // Keep it false due to data loss danger.
      });
    } else {
      const msg = `Database synchronization is disabled.`;
      GBLogEx.info(0, msg);
    }
  }

  /**
   * Loads all items to start several listeners.
   */
  public async getLatestLogs(instanceId: number): Promise<string> {
    const options = {
      where: {
        instanceId: instanceId,
        state: 'active',
        created: {
          [Op.gt]: new Date(Date.now() - 60 * 60 * 1000 * 48) // Latest 48 hours.
        }
      }
    };
    const list = await GuaribasLog.findAll(options);
    let out = 'General Bots Log\n';
    await CollectionUtil.asyncForEach(list, async e => {
      out = `${out}\n${e.createdAt} - ${e.message}`;
    });
    return out;
  }

  /**
   * Loads all items to start several listeners.
   */
  public async loadInstances(): Promise<IGBInstance[]> {
    if (process.env.LOAD_ONLY) {
      const bots = process.env.LOAD_ONLY.split(`;`);
      const and = [];
      await CollectionUtil.asyncForEach(bots, async e => {
        and.push({ botId: e });
      });

      const options = {
        where: {
          [Op.or]: and
        }
      };
      return await GuaribasInstance.findAll(options);
    } else {
      const options = { where: { state: 'active' } };
      return await GuaribasInstance.findAll(options);
    }
  }

  /**
   * Loads just one Bot instance by its internal Id.
   */
  public async loadInstanceById(instanceId: number): Promise<IGBInstance> {
    const options = { where: { instanceId: instanceId, state: 'active' } };

    return await GuaribasInstance.findOne(options);
  }
  /**
   * Loads just one Bot instance.
   */
  public async loadInstanceByActivationCode(code: string): Promise<IGBInstance> {
    let options = { where: { activationCode: code, state: 'active' } };

    return await GuaribasInstance.findOne(options);
  }
  /**
   * Loads just one Bot instance.
   */
  public async loadInstanceByBotId(botId: string): Promise<IGBInstance> {
    const options = { where: {} };
    options.where = { botId: botId, state: 'active' };

    return await GuaribasInstance.findOne(options);
  }

  /**
   * Writes .env required to start the full server. Used during
   * first startup, when user is asked some questions to create the
   * full base environment.
   */
  public async writeEnv(instance: IGBInstance) {
    const env = `
ADDITIONAL_DEPLOY_PATH=
ADMIN_PASS=${instance.adminPass}
BOT_ID=${instance.botId}
CLOUD_SUBSCRIPTIONID=${instance.cloudSubscriptionId}
CLOUD_LOCATION=${instance.cloudLocation}
CLOUD_USERNAME=${instance.cloudUsername}
CLOUD_PASSWORD=${instance.cloudPassword}
MARKETPLACE_ID=${instance.marketplaceId}
MARKETPLACE_SECRET=${instance.marketplacePassword}
STORAGE_DIALECT=${instance.storageDialect}
STORAGE_SERVER=${instance.storageServer}
STORAGE_NAME=${instance.storageName}
STORAGE_USERNAME=${instance.storageUsername}
STORAGE_PASSWORD=${instance.storagePassword}
STORAGE_SYNC=true
STORAGE_SYNC_ALTER=true
ENDPOINT_UPDATE=true
`;

    fs.writeFile('.env', env);
  }

  /**
   * Certifies that network servers will reach back the development machine
   * when calling back from web services. This ensures that reverse proxy is
   * established.
   */
  public async ensureProxy(port): Promise<string> {
    try {
      if (await GBUtil.exists('node_modules/ngrok/bin/ngrok.exe') || await GBUtil.exists('node_modules/.bin/ngrok')) {
        return await ngrok.connect({ port: port });
      } else {
        GBLog.warn('ngrok executable not found. Check installation or node_modules folder.');

        return 'https://localhost';
      }
    } catch (error) {
      // There are false positive from ngrok regarding to no memory, but it's just
      // lack of connection.

      throw new Error(`Error connecting to remote ngrok server, please check network connection. ${error.msg}`);
    }
  }

  /**
   * Setup generic web hooks so .gbapps can expose application logic
   * and get called on demand.
   */
  public installWebHook(isGet: boolean, url: string, callback: any) {
    if (isGet) {
      GBServer.globals.server.get(url, (req, res) => {
        callback(req, res);
      });
    } else {
      GBServer.globals.server.post(url, (req, res) => {
        callback(req, res);
      });
    }
  }

  /**
   * Defines the entry point dialog to be called whenever a user
   * starts talking to the bot.
   */
  public setEntryPointDialog(dialogName: string) {
    GBServer.globals.entryPointDialog = dialogName;
  }

  /**
   * Replaces the default web application root path used to start the GB
   * with a custom home page.
   */
  public setWWWRoot(localPath: string) {
    GBServer.globals.wwwroot = localPath;
  }

  /**
   * Removes a bot instance from storage.
   */
  public async deleteInstance(botId: string) {
    const options = { where: {} };
    options.where = { botId: botId };
    await GuaribasInstance.destroy(options);
  }

  /**
   * Saves a bot instance object to the storage handling
   * multi-column JSON based store 'params' field.
   */
  public async saveInstance(fullInstance: any) {
    const options = { where: {} };
    options.where = { botId: fullInstance.botId };
    let instance = await GuaribasInstance.findOne(options);
    // tslint:disable-next-line:prefer-object-spread
    if (instance) {
      instance = Object.assign(instance, fullInstance);
    } else {
      instance = Object.assign(new GuaribasInstance(), fullInstance);
    }
    try {
      instance.params = JSON.stringify(JSON.parse(instance.params));
    } catch (err) {
      instance.params = JSON.stringify(instance.params);
    }
    return await instance.save();
  }

  /**
   * Loads all bot instances from object storage, if it's formatted.
   */
  public async getApplicationsByInstanceId(appPackages, instanceId: number) {
    const options = { where: { instanceId: instanceId } };
    const apps = await GuaribasApplications.findAll(options);

    let matchingAppPackages = [];
    await CollectionUtil.asyncForEach(appPackages, async appPackage => {
      const filenameOnly = path.basename(appPackage.name);
      const matchedApp = apps.find(app => app.name === filenameOnly);
      if (matchedApp || filenameOnly.endsWith('.gblib')) {
        matchingAppPackages.push(appPackage);
      }
    });

    return matchingAppPackages;
  }

  /**
   * Loads all bot instances from object storage, if it's formatted.
   */
  public async loadAllInstances(
    core: IGBCoreService,
    installationDeployer: IGBInstallationDeployer,
    proxyAddress: string
  ) {
    GBLogEx.info(0, `Loading instances from storage...`);
    let instances: IGBInstance[];
    try {
      instances = await core.loadInstances();
      if (process.env.ENDPOINT_UPDATE === 'true') {
        const group = GBConfigService.get('CLOUD_GROUP') ?? GBConfigService.get('BOT_ID');
        await CollectionUtil.asyncForEach(instances, async instance => {
          GBLogEx.info(instance.instanceId, `Updating bot endpoint for ${instance.botId}...`);
          try {
            await installationDeployer.updateBotProxy(
              instance.botId,
              group,
              `${proxyAddress}/api/messages/${instance.botId}`
            );
          } catch (error) {
            if (error.code === 'ResourceNotFound') {
              GBLog.warn(`Bot ${instance.botId} not found on resource group ${GBConfigService.get('BOT_ID')}.`);
            } else {
              throw new Error(`Error updating bot proxy, details: ${error}.`);
            }
          }
        });
      }
    } catch (error) {
      if (error.parent === undefined) {
        throw new Error(`Cannot connect to operating storage: ${error.message}.`);
      } else {
        // Check if storage is empty and needs formatting.
        const isInvalidObject = error.parent.number === 208 || error.parent.errno === 1; // MSSQL or SQLITE.
        if (isInvalidObject) {
          if (GBConfigService.get('STORAGE_SYNC') !== 'true') {
            throw new Error(
              `Operating storage is out of sync or there is a storage connection error.
            Try setting STORAGE_SYNC to true in .env file. Error: ${error.message}.`
            );
          } else {
            GBLogEx.info(
              0,
              `Storage is empty. After collecting storage structure from all .gbapps it will get synced.`
            );
          }
        } else {
          throw new Error(`Cannot connect to operating storage: ${error.message}.`);
        }
      }
    }

    return instances;
  }

  /**
   * Loads all system packages from 'packages' folder.
   */
  public async loadSysPackages(core: GBCoreService): Promise<IGBPackage[]> {
    // NOTE: if there is any code before this line a semicolon
    // will be necessary before this line.
    // Loads all system packages.
    const sysPackages: IGBPackage[] = [];

    await CollectionUtil.asyncForEach(
      [
        GBAdminPackage,
        GBCorePackage,
        GBSecurityPackage,
        GBKBPackage,
        GBCustomerSatisfactionPackage,
        GBAnalyticsPackage,
        GBWhatsappPackage,
        GBAzureDeployerPackage,
        GBSharePointPackage,
        GBGoogleChatPackage,
        GBBasicPackage,
        GBHubSpotPackage
      ],
      async e => {
        GBLogEx.info(0, `Loading sys package: ${e.name}...`);

        const p = Object.create(e.prototype) as IGBPackage;
        sysPackages.push(p);

        await p.loadPackage(core, core.sequelize);
      }
    );

    return sysPackages;
  }

  /**
   * Verifies that an complex global password has been specified
   * before starting the server.
   */
  public ensureAdminIsSecured() {}

  public async createBootInstance(
    core: GBCoreService,
    installationDeployer: IGBInstallationDeployer,
    proxyAddress: string
  ) {
    return await this.createBootInstanceEx(
      core,
      installationDeployer,
      proxyAddress,
      null,
      GBConfigService.get('FREE_TIER')
    );
  }
  /**
   * Creates the first bot instance (boot instance) used to "boot" the server.
   * At least one bot is required to perform conversational administrative tasks.
   * So a base main bot is always deployed and will act as root bot for
   * configuration tree with three levels: .env > root bot > all other bots.
   */
  public async createBootInstanceEx(
    core: GBCoreService,
    installationDeployer: IGBInstallationDeployer,
    proxyAddress: string,
    deployer,
    freeTier
  ) {
    GBLogEx.info(0, `Deploying cognitive infrastructure (on the cloud / on premises)...`);
    try {
      const { instance, credentials, subscriptionId, installationDeployer } = await StartDialog.createBaseInstance(
        deployer,
        freeTier
      );
      installationDeployer['core'] = this;
      const changedInstance = await installationDeployer['deployFarm2'](
        proxyAddress,
        instance,
        credentials,
        subscriptionId
      );
      await this.writeEnv(changedInstance);
      GBConfigService.init();

      GBLogEx.info(0, `File .env written. Preparing storage and search for the first time...`);
      await this.openStorageFrontier(installationDeployer);
      await this.initStorage();

      return [changedInstance, installationDeployer];
    } catch (error) {
      GBLog.warn(
        `There is an error being thrown, so please cleanup any infrastructure objects
            created during this procedure and .env before running again.`
      );
      throw error;
    }
  }

  /**
   * Helper to get the web browser onpened in UI interfaces.
   */
  public openBrowserInDevelopment() {
    if (process.env.NODE_ENV === 'development') {
      open('http://localhost:4242');
    }
  }

  /**
   * SQL:
   *
   * // let sql: string = '' +
   * //   'IF OBJECT_ID(\'[UserGroup]\', \'U\') IS NULL' +
   * //   'CREATE TABLE [UserGroup] (' +
   * //   '  [id] INTEGER NOT NULL IDENTITY(1,1),' +
   * //   '  [userId] INTEGER NULL,' +
   * //   '  [groupId] INTEGER NULL,' +
   * //   '  [instanceId] INTEGER NULL,' +
   * //   '  PRIMARY KEY ([id1], [id2]),' +
   * //   '  FOREIGN KEY ([userId1], [userId2], [userId3]) REFERENCES [User] ([userId1], [userId2], [userId3]) ON DELETE NO ACTION,' +
   * //   '  FOREIGN KEY ([groupId1], [groupId2]) REFERENCES [Group] ([groupId1], [groupId1]) ON DELETE NO ACTION,' +
   * //   '  FOREIGN KEY ([instanceId]) REFERENCES [Instance] ([instanceId]) ON DELETE NO ACTION)'
   */
  private createTableQueryOverride(tableName, attributes, options): string {
    let sql: string = this.createTableQuery.apply(this.queryGenerator, [tableName, attributes, options]);
    const re1 = /CREATE\s+TABLE\s+\[([^\]]*)\]/;
    const matches = re1.exec(sql);
    if (matches !== null) {
      const table = matches[1];
      const re2 = /PRIMARY\s+KEY\s+\(\[[^\]]*\](?:,\s*\[[^\]]*\])*\)/;
      sql = sql.replace(re2, (match: string, ...args: any[]): string => {
        return `CONSTRAINT [${table}_pk] ${match}`;
      });
      const re3 = /FOREIGN\s+KEY\s+\((\[[^\]]*\](?:,\s*\[[^\]]*\])*)\)/g;
      const re4 = /\[([^\]]*)\]/g;
      sql = sql.replace(re3, (match: string, ...args: any[]): string => {
        const fkcols = args[0];
        let fkname = table;
        let matches2 = re4.exec(fkcols);
        while (matches2 !== null) {
          fkname += `_${matches2[1]}`;
          matches2 = re4.exec(fkcols);
        }

        return `CONSTRAINT [${fkname}_fk] FOREIGN KEY (${fkcols})`;
      });
    }

    return sql;
  }

  /**
   * SQL:
   * let sql = '' +
   * 'ALTER TABLE [UserGroup]' +
   * '  ADD CONSTRAINT [invalid1] FOREIGN KEY ([userId1], [userId2], [userId3]) REFERENCES [User] ([userId1], [userId2], [userId3]) ON DELETE NO ACTION,' +
   * '      CONSTRAINT [invalid2] FOREIGN KEY ([groupId1], [groupId2]) REFERENCES [Group] ([groupId1], [groupId2]) ON DELETE NO ACTION, ' +
   * '      CONSTRAINT [invalid3] FOREIGN KEY ([instanceId1]) REFERENCES [Instance] ([instanceId1]) ON DELETE NO ACTION'
   */
  private changeColumnQueryOverride(tableName, attributes): string {
    let sql: string = this.changeColumnQuery.apply(this.queryGenerator, [tableName, attributes]);
    const re1 = /ALTER\s+TABLE\s+\[([^\]]*)\]/;
    const matches = re1.exec(sql);
    if (matches !== null) {
      const table = matches[1];
      const re2 = /(ADD\s+)?CONSTRAINT\s+\[([^\]]*)\]\s+FOREIGN\s+KEY\s+\((\[[^\]]*\](?:,\s*\[[^\]]*\])*)\)/g;
      const re3 = /\[([^\]]*)\]/g;
      sql = sql.replace(re2, (match: string, ...args: any[]): string => {
        const fkcols = args[2];
        let fkname = table;
        let matches2 = re3.exec(fkcols);
        while (matches2 !== null) {
          fkname += `_${matches2[1]}`;
          matches2 = re3.exec(fkcols);
        }

        return `${args[0] ? args[0] : ''}CONSTRAINT [${fkname}_fk] FOREIGN KEY (${fkcols})`;
      });
    }

    return sql;
  }

  /**
   * Opens storage firewall used by the server when starting to get root bot instance.
   */
  private async openStorageFrontier(installationDeployer: IGBInstallationDeployer) {
    const group = GBConfigService.get('BOT_ID');
    const serverName = GBConfigService.get('STORAGE_SERVER').split('.database.windows.net')[0];
    await installationDeployer.openStorageFirewall(group, serverName);
  }

  public async setConfig(min, name: string, value: any): Promise<any> {
    if (GBConfigService.get('STORAGE_NAME')) {
      // Handles calls for BASIC persistence on sheet files.

      GBLog.info(`Defining Config.xlsx variable ${name}= '${value}'...`);

      let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

      const maxLines = 512;
      const file = 'Config.xlsx';
      const packagePath = GBUtil.getGBAIPath(min.botId, `gbot`);

      let document = await new SystemKeywords().internalGetDocument(client, baseUrl, packagePath, file);

      // Creates book session that will be discarded.

      let sheets = await client.api(`${baseUrl}/drive/items/${document.id}/workbook/worksheets`).get();

      let results = await client
        .api(
          `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='A1:A${maxLines}')`
        )
        .get();

      const rows = results.text;
      let address = '';

      // Fills the row variable.

      for (let i = 1; i <= rows.length; i++) {
        let result = rows[i - 1][0];
        if (result && result.toLowerCase() === name.toLowerCase()) {
          address = `B${i}:B${i}`;
          break;
        }
      }

      let body = { values: [[]] };
      body.values[0][0] = value;

      await client
        .api(
          `${baseUrl}/drive/items/${document.id}/workbook/worksheets('${sheets.value[0].name}')/range(address='${address}')`
        )
        .patch(body);
    } else {
      let packagePath = GBUtil.getGBAIPath(min.botId, `gbot`);
      const config = path.join(GBConfigService.get('STORAGE_LIBRARY'), packagePath, 'config.csv');

      const db = await csvdb(config, ['name', 'value'], ',');
      if (await db.get({ name: name })) {
        await db.edit({ name: name }, { name, value });
      } else {
        await db.add({ name, value });
      }
    }
  }

  /**
   * Get a dynamic param from instance. Dynamic params are defined in Config.xlsx
   * and loaded into the work folder from   comida command.
   *
   * @param name Name of param to get from instance.
   * @param defaultValue Value returned when no param is defined in Config.xlsx.
   */
  public getParam<T>(instance: IGBInstance, name: string, defaultValue?: T, platform = false): any {
    let value = null;
    let params;
    name = name.trim();

    // Gets .gbot Params from specified bot.

    if (instance.params) {
      params = typeof instance.params === 'object' ? instance.params : JSON.parse(instance.params);
      params = GBUtil.caseInsensitive(params);
      value = params ? params[name] : defaultValue;
    }

    // Gets specified bot instance values.

    params = GBUtil.caseInsensitive(instance['dataValues']);

    if (params && !value) {
      // Retrieves the value from specified bot instance (no params collection).

      value = instance['dataValues'][name];

      // If still not found, get from boot bot params.

      const minBoot = GBServer.globals.minBoot as any;

      if (minBoot.instance && !value && instance.botId != minBoot.instance.botId) {
        instance = minBoot.instance;

        if (instance.params) {
          params = typeof instance.params === 'object' ? instance.params : JSON.parse(instance.params);
          params = GBUtil.caseInsensitive(params);
          value = params ? params[name] : defaultValue;
        }

        // If still did not found in boot bot params, try instance fields.

        if (!value) {
          value = instance['dataValues'][name];
        }
        if (!value) {
          value = instance[name];
        }
      }
    }

    if (value === undefined) {
      value = null;
    }

    if (!value && platform) {
      value = process.env[name.replace(/ /g, '_').toUpperCase()];
    }

    if (value && typeof defaultValue === 'boolean') {
      return new Boolean(value ? value.toString().toLowerCase() === 'true' : defaultValue).valueOf();
    }
    if (value && typeof defaultValue === 'string') {
      return value ? value : defaultValue;
    }
    if (value && typeof defaultValue === 'number') {
      return new Number(value ? value : defaultValue ? defaultValue : 0).valueOf();
    }

    const ret = value ?? defaultValue;
    return ret;
  }

  /**
   * Finds a dynamic param from instance.    *
   */
  public async findParam<T>(instance: IGBInstance, criteria: string) {
    let params = null;
    const list = [];
    if (instance.params) {
      params = typeof instance.params === 'object' ? instance.params : JSON.parse(instance.params);
    }

    Object.keys(params).forEach(e => {
      if (e.toLowerCase().indexOf(criteria.toLowerCase()) !== -1) {
        list.push(e);
      }
    });

    return list;
  }

  public async ensureFolders(instances, deployer: GBDeployer) {
    let libraryPath = GBConfigService.get('STORAGE_LIBRARY');

    if (!await GBUtil.exists(libraryPath)) {
      mkdirp.sync(libraryPath);
    }

    await this.syncBotStorage(instances, 'default', deployer, libraryPath);

    const files = fs.readdir(libraryPath);
    await CollectionUtil.asyncForEach(files, async file => {
      if (file.trim().toLowerCase() !== 'default.gbai') {
        let botId = file.replace(/\.gbai/, '');

        await this.syncBotStorage(instances, botId, deployer, libraryPath);
      }
    });
  }

  private async syncBotStorage(instances: any, botId: any, deployer: GBDeployer, libraryPath: string) {
    let instance = instances.find(p => p.botId.toLowerCase().trim() === botId.toLowerCase().trim());

    if (!instance) {
      GBLog.info(`Importing package ${botId}...`);

      // Creates a bot.

      let mobile = null,
        email = null;

      instance = await deployer.deployBlankBot(botId, mobile, email);
      const gbaiPath = path.join(libraryPath, `${botId}.gbai`);

      if (!await GBUtil.exists(gbaiPath)) {
        fs.mkdir(gbaiPath, { recursive: true });

        const base = path.join(process.env.PWD, 'templates', 'default.gbai');

        fs.cp(path.join(base, `default.gbkb`), path.join(gbaiPath, `default.gbkb`), {
          errorOnExist: false,
          force: true,
          recursive: true
        });
        fs.cp(path.join(base, `default.gbot`), path.join(gbaiPath, `default.gbot`), {
          errorOnExist: false,
          force: true,
          recursive: true
        });
        fs.cp(path.join(base, `default.gbtheme`), path.join(gbaiPath, `default.gbtheme`), {
          errorOnExist: false,
          force: true,
          recursive: true
        });
        fs.cp(path.join(base, `default.gbdata`), path.join(gbaiPath, `default.gbdata`), {
          errorOnExist: false,
          force: true,
          recursive: true
        });
        fs.cp(path.join(base, `default.gbdialog`), path.join(gbaiPath, `default.gbdialog`), {
          errorOnExist: false,
          force: true,
          recursive: true
        });
        fs.cp(path.join(base, `default.gbdrive`), path.join(gbaiPath, `default.gbdrive`), {
          errorOnExist: false,
          force: true,
          recursive: true
        });
      }
    }
  }

  public static async createWebDavServer(minInstances: GBMinInstance[]) {
    const userManager = new webdav.SimpleUserManager();
    const privilegeManager = new webdav.SimplePathPrivilegeManager();

    // Create the WebDAV server
    const server = new webdav.WebDAVServer({
      port: 1900,
      httpAuthentication: new webdav.HTTPDigestAuthentication(userManager, 'Default realm'),
      privilegeManager: privilegeManager
    });
    GBServer.globals.webDavServer = server;

    minInstances.forEach(min => {
      const user = min.core.getParam(min.instance, 'WebDav Username', GBConfigService.get('WEBDAV_USERNAME'));
      const pass = min.core.getParam(min.instance, 'WebDav Password', GBConfigService.get('WEBDAV_PASSWORD'));

      if (user && pass) {
        const objUser = userManager.addUser(user, pass);

        const virtualPath = '/' + min.botId;
        let path = GBUtil.getGBAIPath(min.botId, null);
        const gbaiRoot = path.join(GBConfigService.get('STORAGE_LIBRARY'), path);

        server.setFileSystem(virtualPath, new webdav.PhysicalFileSystem(gbaiRoot), successed => {
          GBLogEx.info(min.instance.instanceId, `WebDav online for ${min.botId}...`);
        });
        privilegeManager.setRights(objUser, virtualPath, ['all']);
      }
    });
    server.start(1900);
  }
}
