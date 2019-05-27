/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___  _   _    _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/ \ /`\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| |*| |( (_) )  |
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

import { GBLog, IGBCoreService, IGBInstallationDeployer, IGBInstance, IGBPackage } from 'botlib';
import * as fs from 'fs';
import { Sequelize } from 'sequelize-typescript';
import { GBAdminPackage } from '../../admin.gbapp/index';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import { GBAnalyticsPackage } from '../../analytics.gblib';
import { StartDialog } from '../../azuredeployer.gbapp/dialogs/StartDialog';
import { GBCorePackage } from '../../core.gbapp';
import { GBCustomerSatisfactionPackage } from '../../customer-satisfaction.gbapp';
import { GBKBPackage } from '../../kb.gbapp';
import { GBSecurityPackage } from '../../security.gblib';
import { GBWhatsappPackage } from '../../whatsapp.gblib/index';
import { GuaribasInstance } from '../models/GBModel';
import { GBConfigService } from './GBConfigService';

const opn = require('opn');

/**
 *  Core service layer.
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
   * Constructor retrieves default values.
   */
  constructor() {
    this.adminService = new GBAdminService(this);
  }

  /**
   * Gets database config and connect to storage.
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
      storage = GBConfigService.get('STORAGE_STORAGE');
    } else {
      throw new Error(`Unknown dialect: ${this.dialect}.`);
    }

    const logging: boolean | Function =
      GBConfigService.get('STORAGE_LOGGING') === 'true'
        ? (str: string): void => {
          GBLog.info(str);
        }
        : false;

    const encrypt: boolean = GBConfigService.get('STORAGE_ENCRYPT') === 'true';

    this.sequelize = new Sequelize({
      host: host,
      database: database,
      username: username,
      password: password,
      logging: logging,
      operatorsAliases: false,
      dialect: this.dialect,
      storage: storage,
      dialectOptions: {
        options: {
          encrypt: encrypt
        }
      }, pool: {
        max: 32,
        min: 8,
        idle: 40000,
        evict: 40000,
        acquire: 40000
      }
    });

    if (this.dialect === 'mssql') {
      this.queryGenerator = this.sequelize.getQueryInterface().QueryGenerator;
      // tslint:disable:no-unsafe-any
      this.createTableQuery = this.queryGenerator.createTableQuery;
      this.queryGenerator.createTableQuery = (tableName, attributes, options) =>
        this.createTableQueryOverride(tableName, attributes, options);
      this.changeColumnQuery = this.queryGenerator.changeColumnQuery;
      this.queryGenerator.changeColumnQuery = (tableName, attributes) =>
        this.changeColumnQueryOverride(tableName, attributes);
      // tslint:enable:no-unsafe-any
    }
  }

  public async checkStorage(installationDeployer: IGBInstallationDeployer) {
    try {
      await this.sequelize.authenticate();
    } catch (error) {
      GBLog.info('Opening storage firewall on infrastructure...');
      // tslint:disable:no-unsafe-any
      if (error.parent.code === 'ELOGIN') {
        await this.openStorageFrontier(installationDeployer);
      } else {
        throw error;
      }
      // tslint:ensable:no-unsafe-any
    }
  }

  public async syncDatabaseStructure() {
    if (GBConfigService.get('STORAGE_SYNC') === 'true') {
      const alter = GBConfigService.get('STORAGE_SYNC_ALTER') === 'true';
      GBLog.info('Syncing database...');

      return this.sequelize.sync({
        alter: alter,
        force: false // Keep it false this due to data loss danger.
      });
    } else {
      const msg = `Database synchronization is disabled.`;
      GBLog.info(msg);
    }
  }

  /**
   * Loads all items to start several listeners.
   */
  public async loadInstances(): Promise<IGBInstance[]> {
    return GuaribasInstance.findAll({});
  }

  /**
   * Loads just one Bot instance by its internal Id.
   */
  public async loadInstanceById(instanceId: number): Promise<IGBInstance> {
    const options = { where: { instanceId: instanceId } };

    return GuaribasInstance.findOne(options);
  }

  /**
   * Loads just one Bot instance.
   */
  public async loadInstance(botId: string): Promise<IGBInstance> {
    const options = { where: {} };
    options.where = { botId: botId };

    return await GuaribasInstance.findOne(options);
  }

  public async writeEnv(instance: IGBInstance) {
    const env = `ADDITIONAL_DEPLOY_PATH=
ADMIN_PASS=${instance.adminPass}
BOT_ID=${instance.botId}
CLOUD_SUBSCRIPTIONID=${instance.cloudSubscriptionId}
CLOUD_LOCATION=${instance.cloudLocation}
CLOUD_GROUP=${instance.botId}
CLOUD_USERNAME=${instance.cloudUsername}
CLOUD_PASSWORD=${instance.cloudPassword}
MARKETPLACE_ID=${instance.marketplaceId}
MARKETPLACE_SECRET=${instance.marketplacePassword}
NLP_AUTHORING_KEY=${instance.nlpAuthoringKey}
STORAGE_DIALECT=${instance.storageDialect}
STORAGE_SERVER=${instance.storageServer}.database.windows.net
STORAGE_NAME=${instance.storageName}
STORAGE_USERNAME=${instance.storageUsername}
STORAGE_PASSWORD=${instance.storagePassword}
STORAGE_SYNC=true
`;

    fs.writeFileSync('.env', env);
  }

  public async ensureProxy(port): Promise<string> {
    try {
      if (fs.existsSync('node_modules/ngrok/bin/ngrok.exe') ||
        fs.existsSync('node_modules/ngrok/bin/ngrok')) {
        const ngrok = require('ngrok');

        return await ngrok.connect({ port: port });
      } else {
        GBLog.warn('ngrok executable not found (only tested on Windows). Check installation or node_modules folder.');

        return 'https://localhost';
      }
    } catch (error) {
      // There are false positive from ngrok regarding to no memory, but it's just
      // lack of connection.
      GBLog.verbose(error);
      throw new Error('Error connecting to remote ngrok server, please check network connection.');
    }
  }

  public async saveInstance(fullInstance: any) {
    const options = { where: {} };
    options.where = { botId: fullInstance.botId };
    let instance = await GuaribasInstance.findOne(options);
    // tslint:disable-next-line:prefer-object-spread
    instance = Object.assign(instance, fullInstance);

    return await instance.save();
  }

  /**
   * Loads all bot instances from object storage, if it's formatted.
   *
   * @param core
   * @param azureDeployer
   * @param proxyAddress
   */
  public async loadAllInstances(
    core: IGBCoreService,
    installationDeployer: IGBInstallationDeployer,
    proxyAddress: string
  ) {
    GBLog.info(`Loading instances from storage...`);
    let instances: IGBInstance[];
    try {
      instances = await core.loadInstances();
      const instance = instances[0];
      if (process.env.NODE_ENV === 'development') {
        GBLog.info(`Updating bot endpoint to local reverse proxy (ngrok)...`);
        await installationDeployer.updateBotProxy(
          instance.botId,
          instance.botId,
          `${proxyAddress}/api/messages/${instance.botId}`
        );
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
            GBLog.info(`Storage is empty. After collecting storage structure from all .gbapps it will get synced.`);
          }
        } else {
          throw new Error(`Cannot connect to operating storage: ${error.message}.`);
        }
      }
    }

    return instances;
  }

  /**
   * If instances is undefined here it's because storage has been formatted.
   * Load all instances from .gbot found on deploy package directory.
   * @param instances
   * @param bootInstance
   * @param core
   */
  public async ensureInstances(instances: IGBInstance[], bootInstance: any, core: IGBCoreService) {
    if (instances === undefined) {
      const instance = new GuaribasInstance();
      await instance.save();
      instances = await core.loadInstances();
    }

    return instances;
  }

  public loadSysPackages(core: GBCoreService) {
    // NOTE: if there is any code before this line a semicolon
    // will be necessary before this line.
    // Loads all system packages.

    [
      GBAdminPackage,
      GBAnalyticsPackage,
      GBCorePackage,
      GBSecurityPackage,
      GBKBPackage,
      GBCustomerSatisfactionPackage,
      GBWhatsappPackage
    ].forEach(e => {
      GBLog.info(`Loading sys package: ${e.name}...`);
      const p = Object.create(e.prototype) as IGBPackage;
      p.loadPackage(core, core.sequelize);
    });
  }

  public ensureAdminIsSecured() {
    const password = GBConfigService.get('ADMIN_PASS');
    if (!GBAdminService.StrongRegex.test(password)) {
      throw new Error(
        'Please, define a really strong password in ADMIN_PASS environment variable before running the server.'
      );
    }
  }

  public async createBootInstance(
    core: GBCoreService,
    installationDeployer: IGBInstallationDeployer,
    proxyAddress: string
  ) {
    GBLog.info(`Deploying cognitive infrastructure (on the cloud / on premises)...`);
    try {
      const { instance, credentials, subscriptionId } = await StartDialog.createBaseInstance(installationDeployer);
      const changedInstance = await installationDeployer.deployFarm(
        proxyAddress,
        instance,
        credentials,
        subscriptionId
      );
      core.writeEnv(changedInstance);
      GBLog.info(`File .env written, starting General Bots...`);
      GBConfigService.init();

      return changedInstance;
    } catch (error) {
      GBLog.warn(
        `In case of error, please cleanup any infrastructure objects
            created during this procedure and .env before running again.`
      );
      throw error;
    }
  }

  public openBrowserInDevelopment() {
    if (process.env.NODE_ENV === 'development') {
      opn('http://localhost:4242');
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
      sql = sql.replace(
        re2,
        (match: string, ...args: any[]): string => {
          return `CONSTRAINT [${table}_pk] ${match}`;
        }
      );
      const re3 = /FOREIGN\s+KEY\s+\((\[[^\]]*\](?:,\s*\[[^\]]*\])*)\)/g;
      const re4 = /\[([^\]]*)\]/g;
      sql = sql.replace(
        re3,
        (match: string, ...args: any[]): string => {
          const fkcols = args[0];
          let fkname = table;
          let matches2 = re4.exec(fkcols);
          while (matches2 !== null) {
            fkname += `_${matches2[1]}`;
            matches2 = re4.exec(fkcols);
          }

          return `CONSTRAINT [${fkname}_fk] FOREIGN KEY (${fkcols})`;
        }
      );
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
      sql = sql.replace(
        re2,
        (match: string, ...args: any[]): string => {
          const fkcols = args[2];
          let fkname = table;
          let matches2 = re3.exec(fkcols);
          while (matches2 !== null) {
            fkname += `_${matches2[1]}`;
            matches2 = re3.exec(fkcols);
          }

          return `${args[0] ? args[0] : ''}CONSTRAINT [${fkname}_fk] FOREIGN KEY (${fkcols})`;
        }
      );
    }

    return sql;
  }

  /**
   * Opens storage firewall.
   *
   * @param azureDeployer Infrastructure Deployer instance.
   */
  private async openStorageFrontier(installationDeployer: IGBInstallationDeployer) {
    const group = GBConfigService.get('CLOUD_GROUP');
    const serverName = GBConfigService.get('STORAGE_SERVER').split('.database.windows.net')[0];
    await installationDeployer.openStorageFirewall(group, serverName);
  }
}
