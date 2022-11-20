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

import {
  AutoIncrement,
  BelongsTo,
  Column,
  CreatedAt,
  DataType,
  ForeignKey,
  Model,
  PrimaryKey,
  Table,
  UpdatedAt
} from 'sequelize-typescript';

import { IGBInstance } from 'botlib';

/**
 * Base instance data for a bot.
 */
@Table
export class GuaribasInstance extends Model<GuaribasInstance> implements IGBInstance {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  instanceId: number;

  @Column(DataType.STRING(255))
  botEndpoint: string;

  @Column(DataType.STRING(255))
  whoAmIVideo: string;

  @Column(DataType.STRING(255))
  botId: string;

  @Column(DataType.STRING(255))
  title: string;

  @Column({ type: DataType.STRING(16) })
  activationCode: string;

  @Column(DataType.STRING(255))
  description: string;

  @Column({ type: DataType.STRING(16) })
  state: string;

  version: string;

  @Column(DataType.STRING(64))
  botKey: string;

  @Column(DataType.STRING(255))
  enabledAdmin: boolean;

  @Column(DataType.STRING(255))
  engineName: string;

  @Column(DataType.STRING(255))
  marketplaceId: string;

  @Column(DataType.STRING(255))
  textAnalyticsKey: string;

  @Column(DataType.STRING(255))
  textAnalyticsEndpoint: string;

  @Column({ type: DataType.STRING(64) })
  translatorKey: string;

  @Column({ type: DataType.STRING(128) })
  translatorEndpoint: string;

  @Column(DataType.STRING(255))
  marketplacePassword: string;

  @Column(DataType.STRING(255))
  webchatKey: string;

  @Column(DataType.STRING(255))
  authenticatorTenant: string;

  @Column(DataType.STRING(255))
  authenticatorAuthorityHostUrl: string;

  @Column(DataType.STRING(255))
  cloudSubscriptionId: string;

  @Column(DataType.STRING(255))
  cloudUsername: string;

  @Column(DataType.STRING(255))
  cloudPassword: string;

  @Column(DataType.STRING(255))
  cloudLocation: string;

  @Column(DataType.STRING(255))
  googleBotKey: string;

  @Column(DataType.STRING(255))
  googleChatApiKey: string;

  @Column(DataType.STRING(255))
  googleChatSubscriptionName: string;

  @Column(DataType.STRING(255))
  googleClientEmail: string;

  @Column({ type: DataType.STRING(4000) })
  googlePrivateKey: string;

  @Column(DataType.STRING(255))
  googleProjectId: string;

  @Column({ type: DataType.STRING(255) })
  facebookWorkplaceVerifyToken: string;

  @Column({ type: DataType.STRING(255) })
  facebookWorkplaceAppSecret: string;

  @Column({ type: DataType.STRING(512) })
  facebookWorkplaceAccessToken: string;

  @Column(DataType.STRING(255))
  whatsappBotKey: string;

  @Column(DataType.STRING(255))
  whatsappServiceKey: string;

  @Column(DataType.STRING(255))
  whatsappServiceNumber: string;

  @Column(DataType.STRING(255))
  whatsappServiceUrl: string;

  @Column(DataType.STRING(255))
  smsKey: string;

  @Column(DataType.STRING(255))
  smsSecret: string;

  @Column(DataType.STRING(255))
  smsServiceNumber: string;

  @Column(DataType.STRING(255))
  speechKey: string;

  @Column(DataType.STRING(255))
  speechEndpoint: string;

  @Column(DataType.STRING(255))
  spellcheckerKey: string;

  @Column(DataType.STRING(255))
  spellcheckerEndpoint: string;

  @Column(DataType.STRING(255))
  theme: string;

  @Column(DataType.STRING(255))
  ui: string;

  @Column(DataType.STRING(255))
  kb: string;

  @Column(DataType.STRING(255))
  nlpAppId: string;

  @Column(DataType.STRING(255))
  nlpKey: string;

  @Column({ type: DataType.STRING(512) })
  nlpEndpoint: string;

  @Column(DataType.STRING(255))
  nlpAuthoringKey: string;

  @Column(DataType.STRING(255))
  deploymentPaths: string;

  @Column(DataType.STRING(255))
  searchHost: string;

  @Column(DataType.STRING(255))
  searchKey: string;

  @Column(DataType.STRING(255))
  searchIndex: string;

  @Column(DataType.STRING(255))
  searchIndexer: string;

  @Column(DataType.STRING(255))
  storageUsername: string;

  @Column(DataType.STRING(255))
  storagePassword: string;

  @Column(DataType.STRING(255))
  storageName: string;

  @Column(DataType.STRING(255))
  storageServer: string;

  @Column(DataType.STRING(255))
  storageDialect: string;

  @Column(DataType.STRING(255))
  storagePath: string;

  @Column(DataType.STRING(255))
  adminPass: string;

  @Column(DataType.FLOAT)
  nlpVsSearch: number; // TODO: Remove field.

  @Column(DataType.FLOAT)
  searchScore: number;

  @Column(DataType.FLOAT)
  nlpScore: number;

  @Column(DataType.DATE)
  @CreatedAt
  createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  updatedAt: Date;

  @Column(DataType.STRING(4000))
  params: string;
}

/**
 * Each packaged listed for use in a bot instance.
 */
@Table
export class GuaribasPackage extends Model<GuaribasPackage> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  packageId: number;

  @Column(DataType.STRING(255))
  packageName: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  updatedAt: Date;

  @Column({ type: DataType.STRING(512) })
  custom: string;
}

/**
 * A bot channel.
 */
@Table
export class GuaribasChannel extends Model<GuaribasChannel> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  channelId: number;

  @Column(DataType.STRING(255))
  title: string;

  @Column(DataType.DATE)
  @CreatedAt
  createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  updatedAt: Date;
}

/**
 * An exception that has been thrown.
 */
@Table
//tslint:disable-next-line:max-classes-per-file
export class GuaribasException extends Model<GuaribasException> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  exceptionId: number;

  @Column(DataType.STRING(255))
  message: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  updatedAt: Date;
}

@Table
//tslint:disable-next-line:max-classes-per-file
export class GuaribasApplications extends Model<GuaribasApplications> {
  @Column(DataType.STRING(255))
  name: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  updatedAt: Date;
}

@Table
//tslint:disable-next-line:max-classes-per-file
export class GuaribasSchedule extends Model<GuaribasSchedule> {
  @Column(DataType.STRING(255))
  name: string;

  @Column(DataType.STRING(255))
  schedule: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  updatedAt: Date;
}
