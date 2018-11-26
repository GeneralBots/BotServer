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

import { IGBCoreService, IGBInstance, IGBPackage } from 'botlib';
import * as fs from 'fs';
import { Sequelize } from 'sequelize-typescript';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import { GuaribasInstance } from '../models/GBModel';
import { GBConfigService } from './GBConfigService';
import { AzureDeployerService } from 'packages/azuredeployer.gbapp/services/AzureDeployerService';
import { GBAnalyticsPackage } from 'packages/analytics.gblib';
import { GBAdminPackage } from 'packages/admin.gbapp/index';
import { GBCorePackage } from 'packages/core.gbapp';
import { GBCustomerSatisfactionPackage } from 'packages/customer-satisfaction.gbapp';
import { GBKBPackage } from 'packages/kb.gbapp';
import { GBSecurityPackage } from 'packages/security.gblib';
import { GBWhatsappPackage } from 'packages/whatsapp.gblib/index';

const logger = require('../../../src/logger');
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
  private createTableQuery: (
    tableName: string,
    attributes: any,
    options: any,
  ) => string;

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
  public async initDatabase(): Promise<any> {
    return new Promise(
      (resolve: any, reject: any): any => {
        try {
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
            reject(`Unknown dialect: ${this.dialect}.`);
          }

          const logging: any =
            GBConfigService.get('STORAGE_LOGGING') === 'true'
              ? (str: string): void => {
                  logger.info(str);
                }
              : false;

          const encrypt: boolean =
            GBConfigService.get('STORAGE_ENCRYPT') === 'true';

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
              encrypt: encrypt,
            },
            pool: {
              max: 32,
              min: 8,
              idle: 40000,
              evict: 40000,
              acquire: 40000,
            },
          });

          if (this.dialect === 'mssql') {
            this.queryGenerator = this.sequelize.getQueryInterface().QueryGenerator;
            this.createTableQuery = this.queryGenerator.createTableQuery;
            this.queryGenerator.createTableQuery = (
              tableName,
              attributes,
              options,
            ) => this.createTableQueryOverride(tableName, attributes, options);
            this.changeColumnQuery = this.queryGenerator.changeColumnQuery;
            this.queryGenerator.changeColumnQuery = (tableName, attributes) =>
              this.changeColumnQueryOverride(tableName, attributes);
          }
          resolve();
        } catch (error) {
          reject(error);
        }
      },
    );
  }

  public async syncDatabaseStructure() {
    if (GBConfigService.get('STORAGE_SYNC') === 'true') {
      const alter = GBConfigService.get('STORAGE_SYNC_ALTER') === 'true';
      const force = GBConfigService.get('STORAGE_SYNC_FORCE') === 'true';
      logger.info('Syncing database...');
      return this.sequelize.sync({
        alter: alter,
        force: force,
      });
    } else {
      const msg = 'Database synchronization is disabled.';
      logger.info(msg);
    }
  }

  /**
   * Loads all items to start several listeners.
   */
  public async loadInstances(): Promise<IGBInstance> {
    return GuaribasInstance.findAll({});
  }

  /**
   * Loads just one Bot instance by its internal Id.
   */
  public async loadInstanceById(instanceId: string): Promise<IGBInstance> {
    const options = { where: { instanceId: instanceId } };
    return GuaribasInstance.findOne(options);
  }

  /**
   * Loads just one Bot instance.
   */
  public async loadInstance(botId: string): Promise<IGBInstance> {
    const options = { where: {} };

    if (botId !== '[default]') {
      options.where = { botId: botId };
    }

    return GuaribasInstance.findOne(options);
  }

  public async writeEnv(instance: IGBInstance) {
    const env =
      `ADDITIONAL_DEPLOY_PATH=\n` +
      `ADMIN_PASS=${instance.adminPass}\n` +
      `CLOUD_SUBSCRIPTIONID=${instance.cloudSubscriptionId}\n` +
      `CLOUD_LOCATION=${instance.cloudLocation}\n` +
      `CLOUD_GROUP=${instance.botId}\n` +
      `CLOUD_USERNAME=${instance.cloudUsername}\n` +
      `CLOUD_PASSWORD=${instance.cloudPassword}\n` +
      `MARKETPLACE_ID=${instance.marketplaceId}\n` +
      `MARKETPLACE_SECRET=${instance.marketplacePassword}\n` +
      `NLP_AUTHORING_KEY=${instance.nlpAuthoringKey}\n` +
      `STORAGE_DIALECT=${instance.storageDialect}\n` +
      `STORAGE_SERVER=${instance.storageServer}.database.windows.net\n` +
      `STORAGE_NAME=${instance.storageName}\n` +
      `STORAGE_USERNAME=${instance.storageUsername}\n` +
      `STORAGE_PASSWORD=${instance.storagePassword}\n` +
      `STORAGE_SYNC=true\n`;

    fs.writeFileSync('.env', env);
  }

  public async ensureProxy(port): Promise<string> {
    let proxyAddress: string;
    const ngrok = require('ngrok');
    return await ngrok.connect({ port: port });
  }

  /**
   * SQL:
   *
   * // let sql: string = '' +
   * //   'IF OBJECT_ID(\'[UserGroup]\', \'U\') IS NULL\n' +
   * //   'CREATE TABLE [UserGroup] (\n' +
   * //   '  [id] INTEGER NOT NULL IDENTITY(1,1),\n' +
   * //   '  [userId] INTEGER NULL,\n' +
   * //   '  [groupId] INTEGER NULL,\n' +
   * //   '  [instanceId] INTEGER NULL,\n' +
   * //   '  PRIMARY KEY ([id1], [id2]),\n' +
   * //   '  FOREIGN KEY ([userId1], [userId2], [userId3]) REFERENCES [User] ([userId1], [userId2], [userId3]) ON DELETE NO ACTION,\n' +
   * //   '  FOREIGN KEY ([groupId1], [groupId2]) REFERENCES [Group] ([groupId1], [groupId1]) ON DELETE NO ACTION,\n' +
   * //   '  FOREIGN KEY ([instanceId]) REFERENCES [Instance] ([instanceId]) ON DELETE NO ACTION)'
   */
  private createTableQueryOverride(tableName, attributes, options): string {
    let sql: string = this.createTableQuery.apply(this.queryGenerator, [
      tableName,
      attributes,
      options,
    ]);
    const re1 = /CREATE\s+TABLE\s+\[([^\]]*)\]/;
    const matches = re1.exec(sql);
    if (matches) {
      const table = matches[1];
      const re2 = /PRIMARY\s+KEY\s+\(\[[^\]]*\](?:,\s*\[[^\]]*\])*\)/;
      sql = sql.replace(
        re2,
        (match: string, ...args: any[]): string => {
          return 'CONSTRAINT [' + table + '_pk] ' + match;
        },
      );
      const re3 = /FOREIGN\s+KEY\s+\((\[[^\]]*\](?:,\s*\[[^\]]*\])*)\)/g;
      const re4 = /\[([^\]]*)\]/g;
      sql = sql.replace(
        re3,
        (match: string, ...args: any[]): string => {
          const fkcols = args[0];
          let fkname = table;
          let matches = re4.exec(fkcols);
          while (matches != null) {
            fkname += '_' + matches[1];
            matches = re4.exec(fkcols);
          }
          return 'CONSTRAINT [' + fkname + '_fk] FOREIGN KEY (' + fkcols + ')';
        },
      );
    }
    return sql;
  }

  /**
   * SQL:
   * let sql = '' +
   * 'ALTER TABLE [UserGroup]\n' +
   * '  ADD CONSTRAINT [invalid1] FOREIGN KEY ([userId1], [userId2], [userId3]) REFERENCES [User] ([userId1], [userId2], [userId3]) ON DELETE NO ACTION,\n' +
   * '      CONSTRAINT [invalid2] FOREIGN KEY ([groupId1], [groupId2]) REFERENCES [Group] ([groupId1], [groupId2]) ON DELETE NO ACTION, \n' +
   * '      CONSTRAINT [invalid3] FOREIGN KEY ([instanceId1]) REFERENCES [Instance] ([instanceId1]) ON DELETE NO ACTION\n'
   */
  private changeColumnQueryOverride(tableName, attributes): string {
    let sql: string = this.changeColumnQuery.apply(this.queryGenerator, [
      tableName,
      attributes,
    ]);
    const re1 = /ALTER\s+TABLE\s+\[([^\]]*)\]/;
    const matches = re1.exec(sql);
    if (matches) {
      const table = matches[1];
      const re2 = /(ADD\s+)?CONSTRAINT\s+\[([^\]]*)\]\s+FOREIGN\s+KEY\s+\((\[[^\]]*\](?:,\s*\[[^\]]*\])*)\)/g;
      const re3 = /\[([^\]]*)\]/g;
      sql = sql.replace(
        re2,
        (match: string, ...args: any[]): string => {
          const fkcols = args[2];
          let fkname = table;
          let matches = re3.exec(fkcols);
          while (matches != null) {
            fkname += '_' + matches[1];
            matches = re3.exec(fkcols);
          }
          return (
            (args[0] ? args[0] : '') +
            'CONSTRAINT [' +
            fkname +
            '_fk] FOREIGN KEY (' +
            fkcols +
            ')'
          );
        },
      );
    }
    return sql;
  }

  /**
   * Loads all bot instances from object storage, if it's formatted.
   *
   * @param core
   * @param azureDeployer
   * @param proxyAddress
   */
  public async loadAllInstances(
    core: GBCoreService,
    azureDeployer: AzureDeployerService,
    proxyAddress: string,
  ) {
    logger.info(`Loading instances from storage...`);
    let instances: GuaribasInstance[];
    try {
      instances = await core.loadInstances();
      const instance = instances[0];
      if (process.env.NODE_ENV === 'development') {
        logger.info(`Updating bot endpoint to local reverse proxy (ngrok)...`);
        await azureDeployer.updateBotProxy(
          instance.botId,
          instance.botId,
          `${proxyAddress}/api/messages/${instance.botId}`,
        );
      }
    } catch (error) {
      if (error.parent.code === 'ELOGIN') {
        const group = GBConfigService.get('CLOUD_GROUP');
        const serverName = GBConfigService.get('STORAGE_SERVER').split(
          '.database.windows.net',
        )[0];
        await azureDeployer.openStorageFirewall(group, serverName);
      } else {
        // Check if storage is empty and needs formatting.
        const isInvalidObject =
          error.parent.number == 208 || error.parent.errno == 1; // MSSQL or SQLITE.
        if (isInvalidObject) {
          if (GBConfigService.get('STORAGE_SYNC') != 'true') {
            throw new Error(
              `Operating storage is out of sync or there is a storage connection error. Try setting STORAGE_SYNC to true in .env file. Error: ${
                error.message
              }.`,
            );
          } else {
            logger.info(
              `Storage is empty. After collecting storage structure from all .gbapps it will get synced.`,
            );
          }
        } else {
          throw new Error(
            `Cannot connect to operating storage: ${error.message}.`,
          );
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
  public async ensureInstances(
    instances: GuaribasInstance[],
    bootInstance: any,
    core: GBCoreService,
  ) {
    if (!instances) {
      const saveInstance = new GuaribasInstance(bootInstance);
      await saveInstance.save();
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
      GBWhatsappPackage,
    ].forEach(e => {
      logger.info(`Loading sys package: ${e.name}...`);
      const p = Object.create(e.prototype) as IGBPackage;
      p.loadPackage(core, core.sequelize);
    });
  }

  public ensureAdminIsSecured() {
    const password = GBConfigService.get('ADMIN_PASS');
    if (!GBAdminService.StrongRegex.test(password)) {
      throw new Error(
        'Please, define a really strong password in ADMIN_PASS environment variable before running the server.',
      );
    }
  }

  public async createBootInstance(
    core: GBCoreService,
    azureDeployer: AzureDeployerService,
    proxyAddress: string,
  ) {
    let bootInstance: IGBInstance;
    try {
      await core.initDatabase();
    } catch (error) {
      logger.info(
        `Deploying cognitive infrastructure (on the cloud / on premises)...`,
      );
      try {
        bootInstance = await azureDeployer.deployFarm(proxyAddress);
      } catch (error) {
        logger.warn(
          'In case of error, please cleanup any infrastructure objects created during this procedure and .env before running again.',
        );
        throw error;
      }
      core.writeEnv(bootInstance);
      logger.info(`File .env written, starting General Bots...`);
      GBConfigService.init();
      await core.initDatabase();
    }
    return bootInstance;
  }

  public openBrowserInDevelopment() {
    if (process.env.NODE_ENV === 'development') {
      opn('http://localhost:4242');
    }
  }

}
