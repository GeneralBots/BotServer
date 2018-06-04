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
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
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

const Path = require("path");
const Fs = require("fs");
const _ = require("lodash");
const Parse = require("csv-parse");
const Async = require("async");
const UrlJoin = require("url-join");
const Walk = require("fs-walk");
const logger = require("../../../src/logger");

import { Sequelize } from 'sequelize-typescript';
import { Promise } from "bluebird";
import { GBConfigService } from "./GBConfigService";
import { DataTypeUUIDv1 } from "sequelize";
import { UniversalBot } from "botbuilder";
import { GBServiceCallback, IGBInstance, IGBCoreService } from 'botlib';
import { GuaribasInstance } from "../models/GBModel";

/**
 *  Core service layer.
 */
export class GBCoreService implements IGBCoreService {

  public sequelize: Sequelize;

  private queryGenerator: any;
  private createTableQuery: (tableName, attributes, options) => string;
  private changeColumnQuery: (tableName, attributes) => string;

  /** Dialect used. Tested: mssql and sqlite.  */

  private dialect: string;

  constructor() {
    this.dialect = GBConfigService.get("DATABASE_DIALECT");
  }

  /** Get config and connect to storage. */
  initDatabase(cb) {

    let host: string | undefined;
    let database: string | undefined;
    let username: string | undefined;
    let password: string | undefined;
    let storage: string | undefined;

    if (this.dialect === "mssql") {
      host = GBConfigService.get("DATABASE_HOST");
      database = GBConfigService.get("DATABASE_NAME");
      username = GBConfigService.get("DATABASE_USERNAME");
      password = GBConfigService.get("DATABASE_PASSWORD");
    } else if (this.dialect === "sqlite") {
      storage = GBConfigService.get("DATABASE_STORAGE");
    }

    let logging = (GBConfigService.get("DATABASE_LOGGING") === "true")
      ? (str: string) => { logger.trace(str); }
      : false;

    let encrypt = (GBConfigService.get("DATABASE_ENCRYPT") === "true");

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
      },
    });

    if (this.dialect === "mssql") {
      this.queryGenerator = this.sequelize.getQueryInterface().QueryGenerator;
      this.createTableQuery = this.queryGenerator.createTableQuery;
      this.queryGenerator.createTableQuery = (tableName, attributes, options) =>
        this.createTableQueryOverride(tableName, attributes, options);
      this.changeColumnQuery = this.queryGenerator.changeColumnQuery;
      this.queryGenerator.changeColumnQuery = (tableName, attributes) =>
        this.changeColumnQueryOverride(tableName, attributes);
    }

    setImmediate(cb);
  }

  private createTableQueryOverride(tableName, attributes, options): string {
    let sql: string = this.createTableQuery.apply(this.queryGenerator,
      [tableName, attributes, options]);
    // let sql: string = '' +
    //   'IF OBJECT_ID(\'[UserGroup]\', \'U\') IS NULL\n' +
    //   'CREATE TABLE [UserGroup] (\n' +
    //   '  [id] INTEGER NOT NULL IDENTITY(1,1),\n' +
    //   '  [userId] INTEGER NULL,\n' +
    //   '  [groupId] INTEGER NULL,\n' +
    //   '  [instanceId] INTEGER NULL,\n' +
    //   '  PRIMARY KEY ([id1], [id2]),\n' +
    //   '  FOREIGN KEY ([userId1], [userId2], [userId3]) REFERENCES [User] ([userId1], [userId2], [userId3]) ON DELETE NO ACTION,\n' +
    //   '  FOREIGN KEY ([groupId1], [groupId2]) REFERENCES [Group] ([groupId1], [groupId1]) ON DELETE NO ACTION,\n' +
    //   '  FOREIGN KEY ([instanceId]) REFERENCES [Instance] ([instanceId]) ON DELETE NO ACTION);';
    const re1 = /CREATE\s+TABLE\s+\[([^\]]*)\]/;
    const matches = re1.exec(sql);
    if (matches) {
      const table = matches[1];
      const re2 = /PRIMARY\s+KEY\s+\(\[[^\]]*\](?:,\s*\[[^\]]*\])*\)/;
      sql = sql.replace(re2, (match: string, ...args: any[]): string => {
        return 'CONSTRAINT [' + table + '_pk] ' + match;
      });
      const re3 = /FOREIGN\s+KEY\s+\((\[[^\]]*\](?:,\s*\[[^\]]*\])*)\)/g;
      const re4 = /\[([^\]]*)\]/g;
      sql = sql.replace(re3, (match: string, ...args: any[]): string => {
        const fkcols = args[0];
        let fkname = table;
        let matches = re4.exec(fkcols);
        while (matches != null) {
          fkname += '_' + matches[1];
          matches = re4.exec(fkcols);
        }
        return 'CONSTRAINT [' + fkname + '_fk] FOREIGN KEY (' + fkcols + ')';
      });
    }
    return sql;
  }

  private changeColumnQueryOverride(tableName, attributes): string {
    let sql: string = this.changeColumnQuery.apply(this.queryGenerator,
      [tableName, attributes]);
    // let sql = '' +
    //   'ALTER TABLE [UserGroup]\n' +
    //   '  ADD CONSTRAINT [invalid1] FOREIGN KEY ([userId1], [userId2], [userId3]) REFERENCES [User] ([userId1], [userId2], [userId3]) ON DELETE NO ACTION,\n' +
    //   '      CONSTRAINT [invalid2] FOREIGN KEY ([groupId1], [groupId2]) REFERENCES [Group] ([groupId1], [groupId2]) ON DELETE NO ACTION, \n' +
    //   '      CONSTRAINT [invalid3] FOREIGN KEY ([instanceId1]) REFERENCES [Instance] ([instanceId1]) ON DELETE NO ACTION;\n';
    const re1 = /ALTER\s+TABLE\s+\[([^\]]*)\]/;
    const matches = re1.exec(sql);
    if (matches) {
      const table = matches[1];
      const re2 = /(ADD\s+)?CONSTRAINT\s+\[([^\]]*)\]\s+FOREIGN\s+KEY\s+\((\[[^\]]*\](?:,\s*\[[^\]]*\])*)\)/g;
      const re3 = /\[([^\]]*)\]/g;
      sql = sql.replace(re2, (match: string, ...args: any[]): string => {
        const fkcols = args[2];
        let fkname = table;
        let matches = re3.exec(fkcols);
        while (matches != null) {
          fkname += '_' + matches[1];
          matches = re3.exec(fkcols);
        }
        return (args[0] ? args[0] : '') + 'CONSTRAINT [' + fkname + '_fk] FOREIGN KEY (' + fkcols + ')';
      });
    }
    return sql;
  }

  syncDatabaseStructure(cb) {
    if (GBConfigService.get("DATABASE_SYNC") === "true") {
      const alter = (GBConfigService.get("DATABASE_SYNC_ALTER") === "true");
      const force = (GBConfigService.get("DATABASE_SYNC_FORCE") === "true");
      logger.trace("Syncing database...");
      this.sequelize.sync({
        alter: alter,
        force: force
      }).then(value => {
        logger.trace("Database synced.");
        cb();
      }, err => logger.error(err));
    } else {
      logger.trace("Database synchronization is disabled.");
      cb();
    }
  }

  /**
   * Loads all items to start several listeners.
   * @param cb Instances loaded or error info.
   */
  loadInstances(cb: GBServiceCallback<IGBInstance[]>) {
    GuaribasInstance.findAll({})
      .then((items: IGBInstance[]) => {
        if (!items) items = [];

        if (items.length == 0) {
          cb([], null);
        } else {
          cb(items, null);
        }
      })
      .catch(reason => {
        if (reason.message.indexOf("no such table: GuaribasInstance") != -1) {
          cb([], null);
        } else {
          logger.trace(`GuaribasServiceError: ${reason}`);
          cb(null, reason);
        }
      });
  }

  /**
   * Loads just one Bot instance.
   */
  loadInstance(botId: string, cb: GBServiceCallback<IGBInstance>) {
    let options = { where: {} };

    if (botId != "[default]") {
      options.where = { botId: botId };
    }

    GuaribasInstance.findOne(options)
      .then((instance: IGBInstance) => {
        if (instance) {
          cb(instance, null);
        } else {
          cb(null, null);
        }
      })
      .catch(err => {
        cb(null, err);
        logger.trace(`GuaribasServiceError: ${err}`);
      });
  }
}
