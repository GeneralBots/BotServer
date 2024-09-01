/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

"use strict"

import {
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

@Table({ tableName: 'GBOnlineSubscription' })
export class GBOnlineSubscription extends Model<GBOnlineSubscription> {

  @PrimaryKey
  @AutoIncrement
  @Column
  Id: number

  @Column
  instanceId: number;

  @Column
  externalSubscriptionId: string // MSFT or

  @Column
  saasSubscriptionStatus: string

  @Column
  isFreeTrial: boolean

  @Column
  planId: string;

  @Column
  quantity: number;

  @Column
  lastCCFourDigits: number;

  @Column
  status: string;
}
