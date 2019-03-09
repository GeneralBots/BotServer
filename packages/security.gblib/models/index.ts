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
  DataTypeDate,
  DataTypeDecimal,
  DataTypes,
  DataTypeUUIDv4
} from 'sequelize';

import {
  AutoIncrement,
  BelongsTo,
  BelongsToMany,
  Column,
  CreatedAt,
  DataType,
  ForeignKey,
  HasMany,
  IsUUID,
  Length,
  Model,
  PrimaryKey,
  Sequelize,
  Table,
  UpdatedAt
} from 'sequelize-typescript';

import { GuaribasInstance } from '../../core.gbapp/models/GBModel';

/**
 * A user and its metadata.
 */
@Table
export class GuaribasUser extends Model<GuaribasUser> {
  @PrimaryKey
  @AutoIncrement
  @Column
  public userId: number;

  @Column public displayName: string;

  @Column public userSystemId: string;
  @Column public userName: string;

  @Column public defaultChannel: string;

  @Column public email: string;

  @Column(DataType.STRING(512))
  public internalAddress: string;

  @ForeignKey(() => GuaribasInstance)
  @Column
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;
}

/**
 * A group of users.
 */
@Table
export class GuaribasGroup extends Model<GuaribasGroup> {
  @PrimaryKey
  @AutoIncrement
  @Column
  public groupId: number;

  @Length({ min: 0, max: 512 })
  @Column
  public displayName: string;

  @ForeignKey(() => GuaribasInstance)
  @Column
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
  @Column
  public userId: number;

  @ForeignKey(() => GuaribasGroup)
  @Column
  public groupId: number;

  @ForeignKey(() => GuaribasInstance)
  @Column
  public instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  public instance: GuaribasInstance;

  @BelongsTo(() => GuaribasGroup)
  public group: GuaribasGroup;

  @BelongsTo(() => GuaribasUser)
  public user: GuaribasUser;
}
