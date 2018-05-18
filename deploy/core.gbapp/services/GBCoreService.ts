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

import { Sequelize } from "sequelize-typescript";
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

  /** Dialect used. Tested: mssql and sqlite.  */

  dialect: string;

  constructor() {
    this.dialect = GBConfigService.get("DATABASE_DIALECT");
  }

  /** Get config and connect to storage. */
  initDatabase(cb) {

    let host = "";
    let database = "";
    let username = "";
    let password = "";
    let storage = "";

    if (this.dialect === "mssql") {
      host = GBConfigService.get("DATABASE_HOST");
      database = GBConfigService.get("DATABASE_NAME");
      username = GBConfigService.get("DATABASE_USERNAME");
      password = GBConfigService.get("DATABASE_PASSWORD");
    } else if (this.dialect === "sqlite") {
      storage = GBConfigService.get("DATABASE_STORAGE");
    }

    this.sequelize = new Sequelize({
      host: host,
      database: database,
      username: username,
      password: password,
      logging: false,
      operatorsAliases: false,
      dialect: this.dialect,
      storage: storage,

      dialectOptions: {
        encrypt: true
      },
      pool: {
        max: 32,
        min: 8,
        idle: 40000,
        evict: 40000,
        acquire: 40000
      }
    });
    cb();
  }

  syncDatabaseStructure(cb) {
    if (GBConfigService.get("DATABASE_SYNC")) {
      logger.trace("Syncing database...");
      this.sequelize.sync().then(value => {
        logger.trace("Database synced.");
        cb();
      });
    }
    else{
      logger.trace("Database synchronization is disabled.");
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
          cb(null, reason);
          logger.trace(`GuaribasServiceError: ${reason}`);
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
