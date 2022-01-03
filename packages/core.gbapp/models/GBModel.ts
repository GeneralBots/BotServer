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
export class GuaribasInstance extends Model<GuaribasInstance>
  implements IGBInstance {

  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  public instanceId: number;

  @Column(DataType.STRING(255))
  public botEndpoint: string;

  @Column(DataType.STRING(255))
  public whoAmIVideo: string;

  @Column(DataType.STRING(255))
  public botId: string;

  @Column(DataType.STRING(255))
  public title: string;

  @Column({ type: DataType.STRING(16) })
  public activationCode: string;

  @Column(DataType.STRING(255))
  public description: string;

  @Column({ type: DataType.STRING(16) })
  public state: string;

  public version: string;

  @Column(DataType.STRING(255))
  public enabledAdmin: boolean;

  @Column(DataType.STRING(255))
  public engineName: string;

  @Column(DataType.STRING(255))
  public marketplaceId: string;

  @Column(DataType.STRING(255))
  public textAnalyticsKey: string;

  @Column(DataType.STRING(255))
  public textAnalyticsEndpoint: string;

  @Column({ type: DataType.STRING(64) })
  public translatorKey: string;

  @Column(DataType.STRING(255))
  @Column({ type: DataType.STRING(128) })
  public translatorEndpoint: string;

  @Column(DataType.STRING(255))
  public marketplacePassword: string;

  @Column(DataType.STRING(255))
  public webchatKey: string;

  @Column(DataType.STRING(255))
  public authenticatorTenant: string;

  @Column(DataType.STRING(255))
  public authenticatorAuthorityHostUrl: string;

  @Column(DataType.STRING(255))
  public cloudSubscriptionId: string;

  @Column(DataType.STRING(255))
  public cloudUsername: string;

  @Column(DataType.STRING(255))
  public cloudPassword: string;

  @Column(DataType.STRING(255))
  public cloudLocation: string;

  @Column(DataType.STRING(255))
  public googleBotKey: string;

  @Column(DataType.STRING(255))
  public googleChatApiKey: string;

  @Column(DataType.STRING(255))
  public googleChatSubscriptionName: string;

  @Column(DataType.STRING(255))
  public googleClientEmail: string;

  @Column({ type: DataType.STRING(4000) })
  public googlePrivateKey: string;
  
  @Column(DataType.STRING(255))
  public googleProjectId: string;
  
  @Column({ type: DataType.STRING(255) })
  facebookWorkplaceVerifyToken: string;

  @Column({ type: DataType.STRING(255) })
  facebookWorkplaceAppSecret: string;

  @Column({ type: DataType.STRING(512) })
  facebookWorkplaceAccessToken: string;
  
  @Column(DataType.STRING(255))
  public whatsappBotKey: string;

  @Column(DataType.STRING(255))
  public whatsappServiceKey: string;

  @Column(DataType.STRING(255))
  public whatsappServiceNumber: string;

  @Column(DataType.STRING(255))
  public whatsappServiceUrl: string;

  @Column(DataType.STRING(255))
  public smsKey: string;

  @Column(DataType.STRING(255))
  public smsSecret: string;

  @Column(DataType.STRING(255))
  public smsServiceNumber: string;

  @Column(DataType.STRING(255))
  public speechKey: string;

  @Column(DataType.STRING(255))
  public speechEndpoint: string;

  @Column(DataType.STRING(255))
  public spellcheckerKey: string;

  @Column(DataType.STRING(255))
  public spellcheckerEndpoint: string;

  @Column(DataType.STRING(255))
  public theme: string;

  @Column(DataType.STRING(255))
  public ui: string;

  @Column(DataType.STRING(255))
  public kb: string;

  @Column(DataType.STRING(255))
  public nlpAppId: string;

  @Column(DataType.STRING(255))
  public nlpKey: string;

  @Column(DataType.STRING(255))
  @Column({ type: DataType.STRING(512) })
  public nlpEndpoint: string;

  @Column(DataType.STRING(255))
  public nlpAuthoringKey: string;

  @Column(DataType.STRING(255))
  public deploymentPaths: string;

  @Column(DataType.STRING(255))
  public searchHost: string;

  @Column(DataType.STRING(255))
  public searchKey: string;

  @Column(DataType.STRING(255))
  public searchIndex: string;

  @Column(DataType.STRING(255))
  public searchIndexer: string;

  @Column(DataType.STRING(255))
  public storageUsername: string;

  @Column(DataType.STRING(255))
  public storagePassword: string;

  @Column(DataType.STRING(255))
  public storageName: string;

  @Column(DataType.STRING(255))
  public storageServer: string;

  @Column(DataType.STRING(255))
  public storageDialect: string;

  @Column(DataType.STRING(255))
  public storagePath: string;

  @Column(DataType.STRING(255))
  public adminPass: string;

  @Column(DataType.FLOAT)
  public nlpVsSearch: number;  // TODO: Remove field.

  @Column(DataType.FLOAT)
  public searchScore: number;

  @Column(DataType.FLOAT)
  public nlpScore: number;

  @Column(DataType.DATE)
  @CreatedAt
  public createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  public updatedAt: Date;

  @Column(DataType.STRING(4000))
  public params: string;
}

/**
 * Each packaged listed for use in a bot instance.
 */
@Table
export class GuaribasPackage extends Model<GuaribasPackage> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.STRING(255))
  public packageId: number;

  @Column(DataType.STRING(255))
  public packageName: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  public createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  public updatedAt: Date;

  @Column({ type: DataType.STRING(512) })
  public custom: string;
}

/**
 * A bot channel.
 */
@Table
export class GuaribasChannel extends Model<GuaribasChannel> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.STRING(255))
  public channelId: number;

  @Column(DataType.STRING(255))
  public title: string;

  @Column(DataType.DATE)
  @CreatedAt
  public createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  public updatedAt: Date;
}

/**
 * An exception that has been thrown.
 */
@Table
//tslint:disable-next-line:max-classes-per-file
export class GuaribasException extends Model<GuaribasException> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.STRING(255))
  public exceptionId: number;

  @Column(DataType.STRING(255))
  public message: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  public createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  public updatedAt: Date;
}

@Table
//tslint:disable-next-line:max-classes-per-file
export class GuaribasApplications extends Model<GuaribasApplications> {

  @Column(DataType.STRING(255))
  public name: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  public createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  public updatedAt: Date;
}

@Table
//tslint:disable-next-line:max-classes-per-file
export class GuaribasSchedule extends Model<GuaribasSchedule> {

  @Column(DataType.STRING(255))
  public name: string;

  @Column(DataType.STRING(255))
  public schedule: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;

  @Column(DataType.DATE)
  @CreatedAt
  public createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  public updatedAt: Date;
}
