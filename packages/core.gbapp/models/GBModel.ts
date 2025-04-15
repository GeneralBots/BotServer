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
  declare instanceId: number;

  @Column(DataType.STRING(255))
  declare  botEndpoint: string;

  @Column(DataType.STRING(255))
  declare  whoAmIVideo: string;

  @Column(DataType.STRING(255))
  declare  botId: string;

  @Column(DataType.STRING(255))
  declare title: string;

  @Column({ type: DataType.STRING(16) })
  declare activationCode: string;

  @Column(DataType.STRING(255))
  declare description: string;

  @Column({ type: DataType.STRING(16) })
  declare  state: string;

  declare  version: string;

  @Column(DataType.STRING(64))
  declare  botKey: string;

  @Column(DataType.STRING(255))
  declare enabledAdmin: boolean;

  @Column(DataType.STRING(255))
  declare  engineName: string;

  @Column(DataType.STRING(255))
  declare  marketplaceId: string;

  @Column(DataType.STRING(255))
  declare  textAnalyticsKey: string;

  @Column(DataType.STRING(255))
  declare  textAnalyticsEndpoint: string;

  @Column({ type: DataType.STRING(64) })
  declare  translatorKey: string;

  @Column({ type: DataType.STRING(128) })
  declare   translatorEndpoint: string;

  @Column(DataType.STRING(255))
  declare  marketplacePassword: string;

  @Column(DataType.STRING(255))
  declare  webchatKey: string;

  @Column(DataType.STRING(255))
  declare  authenticatorTenant: string;

  @Column(DataType.STRING(255))
  declare authenticatorAuthorityHostUrl: string;

  @Column(DataType.STRING(255))
  declare  cloudSubscriptionId: string;

  @Column(DataType.STRING(255))
  declare cloudUsername: string;

  @Column(DataType.STRING(255))
  declare cloudPassword: string;

  @Column(DataType.STRING(255))
  declare cloudLocation: string;

  @Column(DataType.STRING(255))
  declare  googleBotKey: string;

  @Column(DataType.STRING(255))
  declare googleChatApiKey: string;

  @Column(DataType.STRING(255))
  declare  googleChatSubscriptionName: string;

  @Column(DataType.STRING(255))
  declare googleClientEmail: string;

  @Column({ type: DataType.STRING(4000) })
  declare googlePrivateKey: string;

  @Column(DataType.STRING(255))
  declare googleProjectId: string;

  @Column({ type: DataType.STRING(255) })
  declare   facebookWorkplaceVerifyToken: string;

  @Column({ type: DataType.STRING(255) })
  declare  facebookWorkplaceAppSecret: string;

  @Column({ type: DataType.STRING(512) })
  declare  facebookWorkplaceAccessToken: string;

  @Column(DataType.STRING(255))
  declare   whatsappBotKey: string;

  @Column(DataType.STRING(255))
  declare  whatsappServiceKey: string;

  @Column(DataType.STRING(255))
  declare  whatsappServiceNumber: string;

  @Column(DataType.STRING(255))
  declare  whatsappServiceUrl: string;

  @Column(DataType.STRING(255))
  declare   smsKey: string;

  @Column(DataType.STRING(255))
  declare  smsSecret: string;

  @Column(DataType.STRING(255))
  declare  smsServiceNumber: string;

  @Column(DataType.STRING(255))
  declare  speechKey: string;

  @Column(DataType.STRING(255))
  declare  speechEndpoint: string;

  @Column(DataType.STRING(255))
  declare  spellcheckerKey: string;

  @Column(DataType.STRING(255))
  declare  spellcheckerEndpoint: string;

  @Column(DataType.STRING(255))
  declare theme: string;

  @Column(DataType.STRING(255))
  declare  ui: string;

  @Column(DataType.STRING(255))
  declare   kb: string;

  @Column(DataType.STRING(255))
  declare  nlpAppId: string;

  @Column(DataType.STRING(255))
  declare  nlpKey: string;

  @Column({ type: DataType.STRING(512) })
  declare  nlpEndpoint: string;

  @Column(DataType.STRING(255))
  declare  nlpAuthoringKey: string;

  @Column(DataType.STRING(255))
  declare  deploymentPaths: string;

  @Column(DataType.STRING(255))
  declare   searchHost: string;

  @Column(DataType.STRING(255))
  declare   searchKey: string;

  @Column(DataType.STRING(255))
  declare  searchIndex: string;

  @Column(DataType.STRING(255))
  declare   searchIndexer: string;

  @Column(DataType.STRING(255))
  declare  storageUsername: string;

  @Column(DataType.STRING(255))
  declare   storagePassword: string;

  @Column(DataType.STRING(255))
  declare  storageName: string;

  @Column(DataType.STRING(255))
  declare   storageServer: string;

  @Column(DataType.STRING(255))
  declare   storageDialect: string;

  @Column(DataType.STRING(255))
  declare   storagePath: string;

  @Column(DataType.STRING(255))
  declare  adminPass: string;

  @Column(DataType.FLOAT)
  declare   searchScore: number;

  @Column(DataType.FLOAT)
  declare   nlpScore: number;

  @Column(DataType.DATE)
  @CreatedAt
  declare createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  declare updatedAt: Date;

  @Column(DataType.STRING(4000))
  declare  params: string;
}

/**
 * Each packaged listed for use in a bot instance.
 */
@Table
export class GuaribasPackage extends Model<GuaribasPackage> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  declare packageId: number;

  @Column(DataType.STRING(255))
  declare packageName: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  declare instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  declare instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  declare createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  declare  updatedAt: Date;

  @Column({ type: DataType.STRING(512) })
  declare custom: string;
}

/**
 * A bot channel.
 */
@Table
export class GuaribasChannel extends Model<GuaribasChannel> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  declare channelId: number;

  @Column(DataType.STRING(255))
  declare title: string;

  @Column(DataType.DATE)
  @CreatedAt
  declare createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  declare updatedAt: Date;
}

/**
 * An exception that has been thrown.
 */
@Table
//tslint:disable-next-line:max-classes-per-file
export class GuaribasLog extends Model<GuaribasLog> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  declare  logId: number;

  @Column(DataType.STRING(1024))
  declare  message: string;

  @Column(DataType.STRING(1))
  declare  kind: string;
  
  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  declare  instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  declare  instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  declare   createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  declare  updatedAt: Date;
}

@Table
//tslint:disable-next-line:max-classes-per-file
export class GuaribasApplications extends Model<GuaribasApplications> {
  @Column(DataType.STRING(255))
  declare name: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  declare instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  declare instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  declare createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  declare updatedAt: Date;
}

@Table
//tslint:disable-next-line:max-classes-per-file
export class GuaribasSchedule extends Model<GuaribasSchedule> {
  @Column(DataType.STRING(255))
  declare name: string;

  @Column(DataType.STRING(255))
  declare schedule: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  declare instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  declare instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  declare createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  declare updatedAt: Date;
}
