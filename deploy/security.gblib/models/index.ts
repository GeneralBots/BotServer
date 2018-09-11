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

"use strict"

import {
  DataTypes,
  DataTypeUUIDv4,
  DataTypeDate,
  DataTypeDecimal
} from "sequelize"

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
} from "sequelize-typescript"

import { GuaribasInstance } from "../../core.gbapp/models/GBModel"

@Table
export class GuaribasUser extends Model<GuaribasUser> {
  @PrimaryKey
  @AutoIncrement
  @Column
  userId: number

  @Column displayName: string

  @Column userSystemId: string
  @Column userName: string

  @Column defaultChannel: string

  @Column email: string

  @Column(DataType.STRING(512))
  internalAddress: string

  @ForeignKey(() => GuaribasInstance)
  @Column
  instanceId: number

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance
}

@Table
export class GuaribasGroup extends Model<GuaribasGroup> {
  @PrimaryKey
  @AutoIncrement
  @Column
  groupId: number

  @Length({ min: 0, max: 512 })
  @Column
  displayName: string

  @ForeignKey(() => GuaribasInstance)
  @Column
  instanceId: number

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance
}

@Table
export class GuaribasUserGroup extends Model<GuaribasUserGroup> {
  @ForeignKey(() => GuaribasUser)
  @Column
  userId: number

  @ForeignKey(() => GuaribasGroup)
  @Column
  groupId: number

  @ForeignKey(() => GuaribasInstance)
  @Column
  instanceId: number

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance

  @BelongsTo(() => GuaribasGroup)
  group: GuaribasGroup

  @BelongsTo(() => GuaribasUser)
  user: GuaribasUser
}
