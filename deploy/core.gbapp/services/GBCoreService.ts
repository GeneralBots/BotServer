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
import { Sequelize } from "sequelize-typescript";
import * as fs from "fs";
import { GBConfigService } from "./GBConfigService";
import { IGBInstance, IGBCoreService } from "botlib";
import { GuaribasInstance } from "../models/GBModel";
import { GBAdminService } from "../../admin.gbapp/services/GBAdminService";
const processExists = require("process-exists");
const TextDecoder = require("util").TextDecoder;

/**
 *  Core service layer.
 */
export class GBCoreService implements IGBCoreService {
  isCloudSetup() {
    return GBConfigService.tryGet("STORAGE_DIALECT");
  }

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
  private createTableQuery: (tableName, attributes, options) => string;

  /**
   * Custom change column query.
   */
  private changeColumnQuery: (tableName, attributes) => string;

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
  async initDatabase() {
    return new Promise((resolve, reject) => {
      try {
        this.dialect = GBConfigService.get("STORAGE_DIALECT");

        let host: string | undefined;
        let database: string | undefined;
        let username: string | undefined;
        let password: string | undefined;
        let storage: string | undefined;

        if (this.dialect === "mssql") {
          host = GBConfigService.get("STORAGE_SERVER");
          database = GBConfigService.get("STORAGE_NAME");
          username = GBConfigService.get("STORAGE_USERNAME");
          password = GBConfigService.get("STORAGE_PASSWORD");
        } else if (this.dialect === "sqlite") {
          storage = GBConfigService.get("STORAGE_STORAGE");
        } else {
          reject(`Unknown dialect: ${this.dialect}.`);
        }

        let logging =
          GBConfigService.get("STORAGE_LOGGING") === "true"
            ? (str: string) => {
                logger.info(str);
              }
            : false;

        let encrypt = GBConfigService.get("STORAGE_ENCRYPT") === "true";

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
            encrypt: encrypt
          },
          pool: {
            max: 32,
            min: 8,
            idle: 40000,
            evict: 40000,
            acquire: 40000
          }
        });

        if (this.dialect === "mssql") {
          this.queryGenerator = this.sequelize.getQueryInterface().QueryGenerator;
          this.createTableQuery = this.queryGenerator.createTableQuery;
          this.queryGenerator.createTableQuery = (
            tableName,
            attributes,
            options
          ) => this.createTableQueryOverride(tableName, attributes, options);
          this.changeColumnQuery = this.queryGenerator.changeColumnQuery;
          this.queryGenerator.changeColumnQuery = (tableName, attributes) =>
            this.changeColumnQueryOverride(tableName, attributes);
        }
        resolve();
      } catch (error) {
        reject(error);
      }
    });
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
      options
    ]);
    const re1 = /CREATE\s+TABLE\s+\[([^\]]*)\]/;
    const matches = re1.exec(sql);
    if (matches) {
      const table = matches[1];
      const re2 = /PRIMARY\s+KEY\s+\(\[[^\]]*\](?:,\s*\[[^\]]*\])*\)/;
      sql = sql.replace(
        re2,
        (match: string, ...args: any[]): string => {
          return "CONSTRAINT [" + table + "_pk] " + match;
        }
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
            fkname += "_" + matches[1];
            matches = re4.exec(fkcols);
          }
          return "CONSTRAINT [" + fkname + "_fk] FOREIGN KEY (" + fkcols + ")";
        }
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
      attributes
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
            fkname += "_" + matches[1];
            matches = re3.exec(fkcols);
          }
          return (
            (args[0] ? args[0] : "") +
            "CONSTRAINT [" +
            fkname +
            "_fk] FOREIGN KEY (" +
            fkcols +
            ")"
          );
        }
      );
    }
    return sql;
  }

  async syncDatabaseStructure() {
    if (GBConfigService.get("STORAGE_SYNC") === "true") {
      const alter = GBConfigService.get("STORAGE_SYNC_ALTER") === "true";
      const force = GBConfigService.get("STORAGE_SYNC_FORCE") === "true";
      logger.info("Syncing database...");
      return this.sequelize.sync({
        alter: alter,
        force: force
      });
    } else {
      let msg = "Database synchronization is disabled.";
      logger.info(msg);
    }
  }

  /**
   * Loads all items to start several listeners.
   */
  async loadInstances(): Promise<IGBInstance> {
    return GuaribasInstance.findAll({});
  }

  /**
   * Loads just one Bot instance by its internal Id.
   */
  async loadInstanceById(instanceId: string): Promise<IGBInstance> {
    let options = { where: { instanceId: instanceId } };
    return GuaribasInstance.findOne(options);
  }

  /**
   * Loads just one Bot instance.
   */
  async loadInstance(botId: string): Promise<IGBInstance> {
    let options = { where: {} };

    if (botId != "[default]") {
      options.where = { botId: botId };
    }

    return GuaribasInstance.findOne(options);
  }

  public async writeEnv(instance: IGBInstance) {
    let env =
      `ADMIN_PASS=${instance.adminPass}\n` +
      `ADDITIONAL_DEPLOY_PATH=\n` +
      `STORAGE_DIALECT=${instance.storageDialect}\n` +
      `STORAGE_SERVER=${instance.storageServer}.database.windows.net\n` +
      `STORAGE_NAME=${instance.storageName}\n` +
      `STORAGE_USERNAME=${instance.storageUsername}\n` +
      `STORAGE_PASSWORD=${instance.storagePassword}\n` +
      `STORAGE_SYNC=true\n` +
      `CLOUD_USERNAME=${instance.cloudUsername}\n` +
      `CLOUD_PASSWORD=${instance.cloudPassword}\n` +
      `CLOUD_SUBSCRIPTIONID=${instance.cloudSubscriptionId}\n` +
      `CLOUD_LOCATION=${instance.cloudLocation}\n` +
      `CLOUD_GROUP=${instance.botId}\n` +
      `NLP_AUTHORING_KEY=${instance.nlpAuthoringKey}`;

    fs.writeFileSync(".env", env);
  }

  public async ensureProxy(port): Promise<string> {
    let proxyAddress: string;
    const ngrok = require("ngrok");
    return await ngrok.connect({port:port});
  }
}
