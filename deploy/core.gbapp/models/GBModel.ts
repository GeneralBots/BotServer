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

import {
  DataTypes,
  DataTypeUUIDv4,
  DataTypeDate,
  DataTypeDecimal
} from "sequelize";

import {
  Sequelize,
  Table,
  Column,
  Model,
  HasMany,
  BelongsTo,
  BelongsToMany,
  Length,
  ForeignKey,
  CreatedAt,
  UpdatedAt,
  DataType,
  IsUUID,
  PrimaryKey,
  AutoIncrement
} from "sequelize-typescript";

import { IGBInstance } from "botlib";

@Table
export class GuaribasInstance extends Model<GuaribasInstance> implements IGBInstance {

  @PrimaryKey
  @AutoIncrement
  @Column
  instanceId: number;

  @Column
  whoAmIVideo: string;

  @Column botId: string;

  @Column title: string;

  @Column description: string;

  @Column version: string;

  @Column enabledAdmin: boolean;

  /* Services section on bot.json */

  @Column engineName: string;

  @Column marketplaceId: string;

  @Column textAnalyticsKey: string;

  @Column marketplacePassword: string;

  @Column webchatKey: string;

  @Column whatsappBotKey: string;

  @Column whatsappServiceKey: string;

  @Column whatsappServiceNumber: string;

  @Column whatsappServiceUrl: string;

  @Column whatsappServiceWebhookUrl: string;

  @Column speechKey: string;

  @Column spellcheckerKey: string;

  @Column theme: string;

  @Column ui: string;

  @Column kb: string;
  
  @Column
  nlpAppId: string;
  
  @Column
  nlpSubscriptionKey: string;
  
  @Column
  @Column({ type: DataType.STRING(512) })
  nlpServerUrl: string;

  @Column searchHost: string;

  @Column searchKey: string;

  @Column searchIndex: string;

  @Column searchIndexer: string;

  /* Settings section of bot.json */

  @Column(DataType.FLOAT) nlpVsSearch: number;

  @Column(DataType.FLOAT) searchScore: number;

  @Column(DataType.FLOAT) nlpScore: number;

  @Column
  @CreatedAt
  createdAt: Date;

  @Column
  @UpdatedAt
  updatedAt: Date;
}

@Table
export class GuaribasPackage extends Model<GuaribasPackage> {

  @PrimaryKey
  @AutoIncrement
  @Column
  packageId: number;

  @Column
  packageName: string;

  @ForeignKey(() => GuaribasInstance)
  @Column
  instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance;

  @Column
  @CreatedAt
  createdAt: Date;

  @Column
  @UpdatedAt
  updatedAt: Date;
}

@Table
export class GuaribasChannel extends Model<GuaribasChannel> {

  @PrimaryKey
  @AutoIncrement
  @Column
  channelId: number;

  @Column title: string;

  @Column
  @CreatedAt
  createdAt: Date;

  @Column
  @UpdatedAt
  updatedAt: Date;
}

@Table
export class GuaribasException extends Model<GuaribasException> {

  @PrimaryKey
  @AutoIncrement
  @Column
  exceptionId: number;

  @Column message: string;

  @ForeignKey(() => GuaribasInstance)
  @Column
  instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance;

  @Column
  @CreatedAt
  createdAt: Date;

  @Column
  @UpdatedAt
  updatedAt: Date;
}
