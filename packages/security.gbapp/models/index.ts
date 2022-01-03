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
  DataType,
  ForeignKey,
  Length,
  Model,
  PrimaryKey,
  Table
} from 'sequelize-typescript';

import { GuaribasInstance } from '../../core.gbapp/models/GBModel';

/**
 * A user and its metadata.
 */
@Table
export class GuaribasUser extends Model<GuaribasUser> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  public userId: number;

  @Column(DataType.STRING(255))
  public displayName: string;

  @Column(DataType.INTEGER)
  public userSystemId: string;
  @Column(DataType.STRING(255))
  public userName: string;

  @Column(DataType.STRING(255))
  public defaultChannel: string;

  @Column(DataType.STRING(255))
  public email: string;

  @Column(DataType.STRING(5))
  public locale: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;

  @Column(DataType.INTEGER)
  public agentSystemId: string;

  @Column(DataType.DATE)
  public agentContacted: Date;

  @Column(DataType.STRING(16))
  public agentMode: string;

  @Column(DataType.TEXT)
  public conversationReference: string;

  @Column(DataType.STRING(64))
  public hearOnDialog: string;
}

/**
 * A group of users.
 */
@Table
export class GuaribasGroup extends Model<GuaribasGroup> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
    public groupId: number;

  @Length({ min: 0, max: 512 })
  @Column(DataType.STRING(512))
  public displayName: string;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;
}

/**
 * Relation of groups and users.
 */
@Table
export class GuaribasUserGroup extends Model<GuaribasUserGroup> {
  @ForeignKey(() => GuaribasUser)
  @Column(DataType.INTEGER)
  public userId: number;

  @ForeignKey(() => GuaribasGroup)
  @Column(DataType.INTEGER)
  public groupId: number;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;

  @BelongsTo(() => GuaribasGroup)
  public group: GuaribasGroup;

  @BelongsTo(() => GuaribasUser)
  public user: GuaribasUser;
}
