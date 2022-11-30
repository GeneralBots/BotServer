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

import { GuaribasChannel, GuaribasInstance } from '../../core.gbapp/models/GBModel.js';
import { GuaribasSubject } from '../../kb.gbapp/models/index.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';

/**
 * A conversation that groups many messages.
 */
@Table
export class GuaribasConversation extends Model<GuaribasConversation> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  conversationId: number;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  instanceId: number;

  @ForeignKey(() => GuaribasSubject)
  @Column(DataType.INTEGER)
  startSubjectId: number;

  @BelongsTo(() => GuaribasSubject)
  startSubject: GuaribasSubject;

  @ForeignKey(() => GuaribasChannel)
  @Column(DataType.INTEGER)
  channelId: string;

  @Column(DataType.DATE)
  rateDate: Date;

  @Column(DataType.FLOAT)
  rate: number;

  @Column(DataType.STRING(512))
  feedback: string;

  @CreatedAt
  @Column(DataType.DATE)
  declare createdAt: Date;

  @Column(DataType.STRING(255))
  text: string;

  @ForeignKey(() => GuaribasUser)
  @Column(DataType.INTEGER)
  startedByUserId: number;

  @BelongsTo(() => GuaribasUser)
  startedBy: GuaribasUser;
}

/**
 * A single message in a conversation.
 */
@Table
export class GuaribasConversationMessage extends Model<GuaribasConversationMessage> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  conversationMessageId: number;

  @ForeignKey(() => GuaribasSubject)
  @Column(DataType.INTEGER)
  subjectId: number;

  @Column(DataType.TEXT)
  content: string;

  @Column(DataType.DATE)
  @CreatedAt
  declare createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  declare updatedAt: Date;

  //tslint:disable-next-line:no-use-before-declare
  @ForeignKey(() => GuaribasConversation)
  @Column(DataType.INTEGER)
  conversationId: number;

  //tslint:disable-next-line:no-use-before-declare
  @BelongsTo(() => GuaribasConversation)
  conversation: GuaribasConversation;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  instanceId: number;

  @ForeignKey(() => GuaribasUser)
  @Column(DataType.INTEGER)
  userId: number;

  @BelongsTo(() => GuaribasUser)
  user: GuaribasUser;
}
