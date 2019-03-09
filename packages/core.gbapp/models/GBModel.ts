/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
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
  @Column
  public instanceId: number;

  @Column
  public botEndpoint: string;

  @Column
  public whoAmIVideo: string;

  @Column
  public botId: string;

  @Column
  public title: string;

  @Column
  public description: string;

  @Column
  public version: string;

  @Column
  public enabledAdmin: boolean;

  @Column
  public engineName: string;

  @Column
  public marketplaceId: string;

  @Column
  public textAnalyticsKey: string;

  @Column
  public textAnalyticsEndpoint: string;

  @Column
  public marketplacePassword: string;

  @Column
  public webchatKey: string;

  @Column
  public authenticatorTenant: string;

  @Column
  public authenticatorAuthorityHostUrl: string;

  @Column
  public authenticatorClientId: string;

  @Column
  public authenticatorClientSecret: string;

  @Column
  public cloudSubscriptionId: string;

  @Column
  public cloudUsername: string;

  @Column
  public cloudPassword: string;

  @Column
  public cloudLocation: string;

  @Column
  public whatsappBotKey: string;

  @Column
  public whatsappServiceKey: string;

  @Column
  public whatsappServiceNumber: string;

  @Column
  public whatsappServiceUrl: string;

  @Column
  public whatsappServiceWebhookUrl: string;

  @Column
  public smsKey: string;

  @Column
  public smsSecret: string;

  @Column
  public smsServiceNumber: string;

  @Column
  public speechKey: string;

  @Column
  public speechEndpoint: string;

  @Column
  public spellcheckerKey: string;

  @Column
  public spellcheckerEndpoint: string;

  @Column
  public theme: string;

  @Column
  public ui: string;

  @Column
  public kb: string;

  @Column
  public nlpAppId: string;

  @Column
  public nlpKey: string;

  @Column
  @Column({ type: DataType.STRING(512) })
  public nlpEndpoint: string;

  @Column
  public nlpAuthoringKey: string;

  @Column
  public deploymentPaths: string;

  @Column
  public searchHost: string;

  @Column
  public searchKey: string;

  @Column
  public searchIndex: string;

  @Column
  public searchIndexer: string;

  @Column
  public storageUsername: string;

  @Column
  public storagePassword: string;

  @Column
  public storageName: string;

  @Column
  public storageServer: string;

  @Column
  public storageDialect: string;

  @Column
  public storagePath: string;

  @Column
  public adminPass: string;

  @Column(DataType.FLOAT)
  public nlpVsSearch: number;

  @Column(DataType.FLOAT)
  public searchScore: number;

  @Column(DataType.FLOAT)
  public nlpScore: number;

  @Column
  @CreatedAt
  public createdAt: Date;

  @Column
  @UpdatedAt
  public updatedAt: Date;
}

/**
 * Each packaged listed for use in a bot instance.
 */
@Table
export class GuaribasPackage extends Model<GuaribasPackage> {
  @PrimaryKey
  @AutoIncrement
  @Column
  public packageId: number;

  @Column
  public packageName: string;

  @ForeignKey(() => GuaribasInstance)
  @Column
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;

  @Column
  @CreatedAt
  public createdAt: Date;

  @Column
  @UpdatedAt
  public updatedAt: Date;
}

/**
 * A bot channel.
 */
@Table
export class GuaribasChannel extends Model<GuaribasChannel> {
  @PrimaryKey
  @AutoIncrement
  @Column
  public channelId: number;

  @Column
  public title: string;

  @Column
  @CreatedAt
  public createdAt: Date;

  @Column
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
  @Column
  public exceptionId: number;

  @Column
  public message: string;

  @ForeignKey(() => GuaribasInstance)
  @Column
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;

  @Column
  @CreatedAt
  public createdAt: Date;

  @Column
  @UpdatedAt
  public updatedAt: Date;
}
